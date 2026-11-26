"""Measure Rabbit Explorer's preserved arc, generated fidelity and inspection supports."""

import argparse
from fractions import Fraction
import json
import sys
from math import gcd

import numpy as np
from scipy.spatial import ConvexHull
import trimesh

from measure_banteng_geometry import adaptive_upper, section, solid, upper
from measure_lanner_geometry import load_reference_records, validated_reference_fan
from measure_seriema_courier_geometry import silhouette
from rabbit_explorer_art_mesh import ARC, INTEGRATION, SCALE, collision_groups, load, metre_space, require
from verify_eagle_cruiser_geometry import separating_planes
from vulture_mk_i_exact_fit import grid_index

ROUTES = (
    ("bow-arc-opening", 0, -.7, 12.05, 17.9),
    ("dorsal-exterior", 0, 8.1, -16.6, 16.6),
    ("ventral-exterior", 0, -3.25, -16.6, 16.6),
    ("port-inner-arc", -5.5, -1, -3, 7.0),
    ("starboard-inner-arc", 5.5, -1, -3, 7.0),
    ("port-outer-arc", -14.2, 0, -16.6, 16.6),
    ("starboard-outer-arc", 14.2, 0, -16.6, 16.6),
)


def raw_surface(points, faces):
    return trimesh.Trimesh(np.array(points) * float(SCALE), faces, process=False)


def source_surfaces(source):
    return {name: raw_surface(mesh["vertices"], mesh["faces"]) for name, (mesh, _) in source.items()}


def geometry(source, candidate):
    low = "candidateNative" if candidate else "archiveNative"
    _, meshes, groups = collision_groups(source["detailed"], source[low], candidate=candidate)
    points = np.array(meshes[0]["vertices"] + meshes[1]["vertices"]) * float(SCALE)
    return points, groups


def route_report(source, candidate):
    points, groups = geometry(source, candidate)
    clouds = [points[indices] for indices in groups.values()]
    basis = np.vstack([np.eye(3), -np.eye(3)])
    normals = [np.vstack([ConvexHull(cloud).equations[:, :3], basis]) for cloud in clouds]
    exact = [[tuple(map(Fraction, point)) for point in cloud] for cloud in clouds]
    results = []
    for name, x, y, near, far in ROUTES:
        triangle = np.array([[x - .035, y, far], [x + .035, y, far], [x, y + .07, far]])
        travel = np.array([0, 0, far - near])
        swept = np.vstack([triangle, triangle - travel])
        original = [tuple(map(Fraction, point)) for point in triangle]
        displacement = tuple(map(Fraction, travel))
        sweep = original + [tuple(a - b for a, b in zip(point, displacement, strict=True)) for point in original]
        axes = []
        for cloud, directions in zip(clouds, normals, strict=True):
            best = ((swept @ directions.T).min(0) - (cloud @ directions.T).max(0)).argmax()
            raw = tuple(int(v) for v in np.rint(directions[best] * 2**20))
            axes.append(tuple(value // gcd(*raw) for value in raw))
        guard = separating_planes(exact, sweep, axes)
        results.append({"name": name, "triangle": triangle.tolist(), "travel": travel.tolist(),
                        "axes": axes, "clearanceMillimetres": guard})
    return {"supportCount": len(groups), "routes": results}


REFERENCE_FANS_SHA = "e8ac60c477f5b2dcd010cdaa3a39894748e3b3ae15c93a5b4e98f8ad4f3c6d94"
REFERENCE_DIAGNOSTIC_SHA = "944f04b8eb701b9b012861606afa980d86cdf64bcd0b17bff84fe155f3e37ecf"


def support_fit(source, candidate):
    points, groups = geometry(source, candidate)
    mode = "candidate" if candidate else "archive"
    membership = np.concatenate(list(groups.values()))
    records = load_reference_records(INTEGRATION / "REFERENCE-FANS.json", REFERENCE_FANS_SHA,
                                     REFERENCE_DIAGNOSTIC_SHA, mode, points, membership, len(groups), "Rabbit")
    targets = {"detailed": 1800, "archiveNative": 2000}
    if candidate:
        targets["candidateNative"] = 2200
    surfaces = source_surfaces(source)
    indices = {name: grid_index(surfaces[name]) for name in targets}
    rows = {}
    for (name, vertices), record in zip(groups.items(), records, strict=True):
        fan = validated_reference_fan(record, (name, 1), points, np.array(vertices), "Rabbit")
        rows[name] = {lod: adaptive_upper(index, points[fan], targets[lod]) for lod, index in indices.items()}
    return {
        "nominalPaddingMetres": .02,
        "minimumRefinementTargetsMillimetres": targets,
        "fullUpperMillimetres": {lod: max(row[lod] for row in rows.values()) for lod in indices},
        "pieces": rows,
    }


def cap_width(model, width):
    left, right = model[0][:width], model[0][-width:]
    require(all(point[0] < 0 for point in left) and all(point[0] > 0 for point in right),
            "Rabbit Explorer arc lost its original separated caps")
    for start in (0, len(model[0]) - width):
        end = set(range(start, start + width))
        require(sum(set(face) <= end for face in model[1]) == width - 2, "Rabbit Explorer lost an arc cap")
    separation = (Fraction(str(min(p[0] for p in right))) - Fraction(str(max(p[0] for p in left)))) * SCALE
    return separation.numerator * 1000 // separation.denominator


def fidelity(source):
    detailed, _ = source["detailed"]
    historical, old_parts = source["archiveNative"]
    candidate, new_parts = source["candidateNative"]
    surfaces = source_surfaces(source)
    indices = {name: grid_index(surface) for name, surface in surfaces.items()}
    rows = []
    gaps = {}
    for old_part, new_part in zip(old_parts, new_parts, strict=True):
        index, name = old_part["index"], old_part["name"]
        require((index, name) == (new_part["index"], new_part["name"]), "changed Rabbit Explorer part ownership")
        old, new = section(historical, old_part), section(candidate, new_part)
        old_surface, new_surface = raw_surface(*old[:2]), raw_surface(*new[:2])
        solid(old_surface)
        solid(new_surface)
        require(set(old[2]) == set(new[2]), "changed Rabbit Explorer material set")
        exact = old == new
        require(exact == (index in {2, 3, 4, 5, 7, 8, 11}), "changed Rabbit Explorer fixed geometry")
        if name == ARC:
            gaps = {"archiveNative": cap_width(old, 8), "candidateNative": cap_width(new, 4)}
            require(gaps["archiveNative"] == gaps["candidateNative"], "changed Rabbit Explorer cap separation")
        bounds = {"candidateToArchive": 0, "archiveToCandidate": 0} if exact else {
            "candidateToArchive": upper(new_surface, grid_index(old_surface), 625),
            "archiveToCandidate": upper(old_surface, grid_index(new_surface), 625),
        }
        rows.append({"partIndex": index, "name": name, "exactVerticesFacesMaterialsAndUV": exact,
                     "boundsMillimetres": bounds})
    detailed_arc = section(detailed, source["detailed"][1][0])
    gaps["detailed"] = cap_width(detailed_arc, 12)
    return {
        "exactParts": sum(row["exactVerticesFacesMaterialsAndUV"] for row in rows),
        "allParts": len(rows),
        "arcEndcapLateralSeparationLowerMillimetres": gaps,
        "silhouette": {
            "directionsPerOrthographicView": 16,
            "againstArchivedNative": silhouette(metre_space(historical), metre_space(candidate)),
            "againstDetailed": silhouette(metre_space(detailed), metre_space(candidate)),
        },
        "fullUpperMillimetres": {
            name: max(row["boundsMillimetres"][name] for row in rows)
            for name in ("candidateToArchive", "archiveToCandidate")
        } | {
            "candidateToDetailed": upper(surfaces["candidateNative"], indices["detailed"]),
            "detailedToCandidate": upper(surfaces["detailed"], indices["candidateNative"]),
        },
        "archivedOriginalComparisonUpperMillimetres": {
            "archiveNativeToDetailed": upper(surfaces["archiveNative"], indices["detailed"]),
            "detailedToArchivedNative": upper(surfaces["detailed"], indices["archiveNative"]),
        },
        "parts": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    choices = ("archive-routes", "candidate-routes", "archive-fit", "candidate-fit", "candidate-fidelity")
    parser.add_argument("report", choices=choices)
    args = parser.parse_args()
    source = load()[3]
    if args.report == "candidate-fidelity":
        report = fidelity(source)
    else:
        measure = route_report if args.report.endswith("-routes") else support_fit
        report = measure(source, args.report.startswith("candidate"))
    sys.stdout.buffer.write((json.dumps(report, indent=2) + "\n").encode())


if __name__ == "__main__":
    main()
