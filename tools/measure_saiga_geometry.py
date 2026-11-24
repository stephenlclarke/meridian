"""Measure the separate SH060 project candidate and both original references."""

from fractions import Fraction
import argparse
import json
from math import isqrt
import tomllib

import numpy as np
import trimesh

from measure_banteng_geometry import adaptive_upper, section, solid, upper
from prepare_banteng_candidate import closed
from measure_lanner_geometry import make_fan
from prepare_saiga_geometry import INTEGRATION, ORIGINAL, build
from saiga_art_mesh import SCALE, assemble, collision_groups
from verify_saiga_geometry import load_collision
from vulture_mk_i_exact_fit import grid_index


def surface(section_data):
    points, faces = section_data[:2]
    return trimesh.Trimesh(np.asarray(points) * float(SCALE), faces, process=False)


def outward_mm(squared_distance):
    distance = squared_distance * 1_000_000
    lower = isqrt(distance.numerator // distance.denominator)
    return lower + (lower * lower < distance)


def well_depth(points):
    if len(points) != 12:
        raise ValueError("Changed Saiga blind well topology")
    squared = [
        sum((Fraction(str(a)) - Fraction(str(b)))**2 for a, b in zip(mouth, floor, strict=True)) * SCALE**2
        for mouth, floor in zip(points[:4], points[-4:], strict=True)
    ]
    if min(squared) <= 0:
        raise ValueError("Changed Saiga blind well depth")
    return {"mouthToFloorUpperMillimetres": [outward_mm(value) for value in squared]}


def projection_deltas(reference, candidate):
    direction_steps = ((1, 0), (2, 1), (1, 1), (1, 2), (0, 1), (-1, 2), (-1, 1), (-2, 1))
    directions = direction_steps + tuple((-x, -y) for x, y in direction_steps)
    original = [tuple(Fraction(str(value)) * SCALE for value in point) for point in reference["vertices"]]
    reduced = [tuple(Fraction(str(value)) * SCALE for value in point) for point in candidate["vertices"]]
    offsets = {}
    for name, first, second in (("xy", 0, 1), ("xz", 0, 2), ("yz", 1, 2)):
        measured = []
        for x, y in directions:
            upper = max(x * point[first] + y * point[second] for point in original)
            lower = max(x * point[first] + y * point[second] for point in reduced)
            measured.append(outward_mm((upper - lower)**2 / (x*x + y*y)))
        offsets[name] = max(measured)
    axis_offsets = [
        [outward_mm((edge(v[i] for v in original) - edge(v[i] for v in reduced))**2)
         for edge in (min, max)]
        for i in range(3)
    ]
    return {"axisExtremaAbsoluteDeltaMillimetres": axis_offsets, "maxSampledSilhouetteOffsetMillimetres": offsets}


def validate_surface(mesh, creator_method):
    if creator_method != "Mesh.torus / Mesh.raw":
        solid(mesh)
        return
    closed(mesh.vertices, mesh.faces)
    undirected = {tuple(sorted((a,b))) for a,b,c in mesh.faces.tolist() for a,b in ((a,b),(b,c),(c,a))}
    if len(mesh.vertices) - len(undirected) + len(mesh.faces) != 0 or mesh.volume <= 0 or not np.all(mesh.area_faces > 0):
        raise ValueError("Saiga collar lost its closed genus-one topology")


def compare(candidate_path):
    source = tomllib.loads((ORIGINAL / "resource.toml").read_text())
    original = source["nativeDesign"]
    detailed = source["design"]
    parts = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]["nativeDesign"]["parts"]
    derivative = json.loads(candidate_path.read_bytes())
    candidate = derivative["nativeCandidate"]
    remaining = {item["originalPartIndex"]: item for item in derivative["parts"]}
    omitted = set(derivative["omittedBlindWellParts"])
    assert len(original["faces"]) == 1720 and len(candidate["faces"]) == 512
    assert set(remaining) | omitted == set(range(28)) and not set(remaining) & omitted
    whole_candidate = trimesh.Trimesh(np.asarray(candidate["vertices"]) * float(SCALE), candidate["faces"], process=False)
    whole_archive = trimesh.Trimesh(np.asarray(original["vertices"]) * float(SCALE), original["faces"], process=False)
    whole_detail = surface((detailed["vertices"], detailed["faces"]))
    candidate_index = grid_index(whole_candidate)
    archive_index = grid_index(whole_archive)
    detail_index = grid_index(whole_detail)
    rows = []
    depths = {}
    for index, part in enumerate(parts):
        frozen = section(original, part)
        old = surface(frozen)
        validate_surface(old, part["creatorMethod"])
        if index in omitted:
            if part["creatorMethod"] != "Mesh.well":
                raise ValueError("A Saiga omission is not a blind well")
            upper_bounds = {"archiveToCandidate": upper(old, candidate_index)}
            depths[str(index)] = well_depth(frozen[0])
            mode = "omitted independent inboard blind well"
        else:
            choice = remaining[index]
            proposed = section(candidate, choice)
            new = surface(proposed)
            validate_surface(new, part["creatorMethod"])
            if set(proposed[2]) != set(frozen[2]):
                raise ValueError("Saiga material identifiers changed")
            mode = choice["mode"]
            if mode == "original":
                if proposed != frozen:
                    raise ValueError("A fixed Saiga part differs from the source")
                upper_bounds = {"candidateToArchive": 0, "archiveToCandidate": 0}
            elif part["creatorMethod"] == "Mesh.torus / Mesh.raw":
                original_points = frozen[0]
                expected = [original_points[station * 8 + side] for station in range(0, 24, 3) for side in range(0, 8, 2)]
                if proposed[0] != expected:
                    raise ValueError("Saiga collar projection extrema changed")
                upper_bounds = {"candidateToArchive": upper(new, archive_index),
                                "archiveToCandidate": upper(old, candidate_index)}
            else:
                if part["creatorMethod"] != "Hull.loft / Hull.solid":
                    raise ValueError("Saiga resampling affected an unrelated part")
                rows_original = [frozen[0][i:i + 12] for i in range(0, len(frozen[0]), 12)]
                rows_candidate = [proposed[0][i:i + 4] for i in range(0, len(proposed[0]), 4)]
                for first, second in zip(rows_original, rows_candidate, strict=True):
                    if second != [first[i] for i in (0, 3, 6, 9)]:
                        raise ValueError("Saiga retained ring station or extrema moved")
                upper_bounds = {"candidateToArchive": upper(new, archive_index),
                                "archiveToCandidate": upper(old, candidate_index)}
        rows.append({"index": index, "name": part["name"], "mode": mode, "verifiedUpperMillimetres": upper_bounds})
    archived_projection = projection_deltas(original, candidate)
    if any(any(row) for row in archived_projection["axisExtremaAbsoluteDeltaMillimetres"]):
        raise ValueError("Saiga candidate moved the archived native axis extrema")
    return {
        "originalArchivedNativeFaces": len(original["faces"]),
        "candidateNativeFaces": len(candidate["faces"]),
        "fixedParts": sum(item["mode"] == "original" for item in rows),
        "resampledParts": sum(item["mode"] == "resampled original longitudinal stations" for item in rows),
        "resampledCollars": sum(item["mode"] == "resampled original toroidal rings" for item in rows),
        "omittedBlindWellParts": len(omitted),
        "omittedWellDepths": depths,
        "silhouette": {
            "directionsPerOrthographicView": 16,
            "againstArchivedNative": archived_projection,
            "againstDetailed": projection_deltas(detailed, candidate),
        },
        "fullUpperMillimetres": {
            key: max(row["verifiedUpperMillimetres"].get(key, 0) for row in rows)
            for key in ("candidateToArchive", "archiveToCandidate")
        } | {
            "candidateToDetailed": upper(whole_candidate, detail_index),
            "detailedToCandidate": upper(whole_detail, candidate_index),
        },
        "archivedOriginalComparisonUpperMillimetres": {
            "archivedNativeToDetailed": upper(whole_archive, detail_index),
            "detailedToArchivedNative": upper(whole_detail, archive_index),
        },
        "parts": rows,
    }


def measure_fit(use_candidate):
    source = tomllib.loads((ORIGINAL / "resource.toml").read_text())
    original_parts = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
    detailed = source["design"], original_parts["design"]["parts"]
    archived = source["nativeDesign"], original_parts["nativeDesign"]["parts"]
    targets = {"detailed": 1800, "archiveNative": 2000}
    references = {"detailed": detailed[0], "archiveNative": archived[0]}
    if use_candidate:
        raw_points, _, _, groups = load_collision()
        candidate = json.loads((ORIGINAL / "SH061-project-candidate.json").read_bytes())
        targets["candidateNative"] = 2400
        references["candidateNative"] = candidate["nativeCandidate"]
    else:
        artwork_slots, meshes = assemble(detailed, archived)
        groups = collision_groups(detailed, archived, artwork_slots, meshes, archived=True)
        raw_points = [
            [coordinate * float(SCALE) for coordinate in point]
            for mesh in meshes for point in mesh["vertices"]
        ]
    points = np.array(raw_points)
    indices = {
        lod: grid_index(surface((mesh["vertices"], mesh["faces"])))
        for lod, mesh in references.items()
    }
    rows = {}
    for name, group in groups.items():
        vertices = np.array(group)
        fan = make_fan(points, vertices)
        rows[name] = {lod: adaptive_upper(index, points[fan], targets[lod]) for lod, index in indices.items()}
    return {
        "roundingMetres": .02, "minimumRefinementTargetsMillimetres": targets,
        "fullUpperMillimetres": {lod: max(row[lod] for row in rows.values()) for lod in targets},
        "pieces": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", choices=("archive-fit", "candidate-fit", "candidate-fidelity"))
    args = parser.parse_args()
    build()
    if args.report == "candidate-fidelity":
        result = compare(ORIGINAL / "SH061-project-candidate.json")
    else:
        result = measure_fit(args.report == "candidate-fit")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
