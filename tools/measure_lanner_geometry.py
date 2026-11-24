"""Measure SH048 candidate fidelity and rounded supports against both reference meshes."""

import argparse
from collections import Counter
import json
import sys
import tomllib

import numpy as np
from scipy.spatial import ConvexHull
import trimesh

from lanner_art_mesh import assemble
import measure_goshawk_surface_fit as fit
from prepare_lanner_geometry import INTEGRATION, ORIGINAL, SCALE, build
from vulture_mk_i_exact_fit import bound_fan, full_upper, grid_index


EXPECTED_CHANGED = {0, 1, 6, 9, 10, 12, 13}


def section(model, part):
    vertex = part["vertexStart"]
    end_vertex = vertex + part["vertexCount"]
    face = part["faceStart"]
    end_face = face + part["faceCount"]
    return (
        model["vertices"][vertex:end_vertex],
        [[index - vertex for index in triangle] for triangle in model["faces"][face:end_face]],
        model["faceMaterials"][face:end_face],
        model["faceTextures"][face:end_face],
    )


def closed_surface(points, triangles):
    if set(i for triangle in triangles for i in triangle) != set(range(len(points))):
        raise ValueError("Unreferenced part vertex")
    directed = Counter((a, b) for a, b, c in triangles for a, b in ((a, b), (b, c), (c, a)))
    if not all(count == directed[(b, a)] == 1 for (a, b), count in directed.items()):
        raise ValueError("Source part has an open or inconsistently wound edge")
    geometry = trimesh.Trimesh(np.asarray(points) * float(SCALE), triangles, process=False)
    if not (geometry.area_faces > 0).all():
        raise ValueError("Source part has a degenerate face")
    return geometry


def compare(candidate_path):
    original = tomllib.loads((ORIGINAL / "resource.toml").read_text())["nativeDesign"]
    ownership = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]["nativeDesign"]["parts"]
    candidate_doc = json.loads(candidate_path.read_bytes())
    candidate, parts = candidate_doc["nativeCandidate"], candidate_doc["parts"]
    if len(original["faces"]) != 936 or not 0 < len(candidate["faces"]) <= 512:
        raise ValueError("Lanner original or candidate face budget differs")
    if len(parts) != len(ownership) or len(parts) != 18:
        raise ValueError("Lanner candidate did not preserve all original parts")
    rows = []
    for index, (old, new) in enumerate(zip(ownership, parts, strict=True)):
        if old["name"] != new["name"]:
            raise ValueError("Lanner candidate part identity changed")
        src, proposed = section(original, old), section(candidate, new)
        old_geom = closed_surface(src[0], src[1])
        new_geom = closed_surface(proposed[0], proposed[1])
        if set(src[2]) != set(proposed[2]):
            raise ValueError("A Lanner candidate part changed its original material set")
        if src == proposed:
            if index in EXPECTED_CHANGED:
                raise ValueError("Candidate did not resample a declared curved part")
            bounds = {"candidateToArchive": 0, "archiveToCandidate": 0}
        else:
            if index not in EXPECTED_CHANGED:
                raise ValueError("A fixed Lanner candidate part changed source coordinates, faces, materials or UVs")
            bounds = {
                "candidateToArchive": full_upper(grid_index(old_geom), new_geom.triangles, 500),
                "archiveToCandidate": full_upper(grid_index(new_geom), old_geom.triangles, 500),
            }
        rows.append({"index": index, "name": old["name"], "originalFaces": len(src[1]),
                     "candidateFaces": len(proposed[1]), "verifiedUpperMillimetres": bounds})
    return {"originalArchivedNativeFaces": len(original["faces"]), "candidateNativeFaces": len(candidate["faces"]),
            "archiveWithin512FaceBudget": False, "nativeCandidateWithin512FaceBudget": True, "parts": rows}


def make_fan(vertices, indices):
    unique, first = np.unique(vertices[indices], axis=0, return_index=True)
    hull = ConvexHull(unique)
    integers, _ = fit.dyadic(unique)
    center_sum = tuple(map(sum, zip(*integers, strict=True)))
    faces = []
    for a, b, c in hull.simplices:
        normal = fit.cross(fit.sub(integers[b], integers[a]), fit.sub(integers[c], integers[a]))
        outward = len(integers) * fit.dot(normal, integers[a]) - fit.dot(normal, center_sum)
        assert outward
        if outward < 0:
            b, c = c, b
        mapped = [int(indices[first[i]]) for i in (a, b, c)]
        start = mapped.index(min(mapped))
        faces.append(mapped[start:] + mapped[:start])
    faces.sort()
    bound_fan(vertices[indices], np.searchsorted(indices, faces))
    return faces


def envelope_fit(candidate_path):
    source = tomllib.loads((ORIGINAL / "resource.toml").read_text())
    parts = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
    candidate = json.loads(candidate_path.read_bytes()) if candidate_path else None
    native = (
        (candidate["nativeCandidate"], candidate["parts"], 4) if candidate
        else (source["nativeDesign"], parts["nativeDesign"]["parts"], 8)
    )
    slots, meshes = assemble((source["design"], parts["design"]["parts"], 12), native)
    scaled = np.array(meshes[0]["vertices"] + meshes[1]["vertices"]) * float(SCALE)
    support_ids = np.array(meshes[0]["groups"] + meshes[1]["groups"])
    targets = {"detailed": 1400, "archiveNative": 1400}
    original_native = source["nativeDesign"]
    references = {
        "detailed": (meshes[0]["vertices"], meshes[0]["triangles"]),
        "archiveNative": (original_native["vertices"], original_native["faces"]),
    }
    if candidate:
        targets["candidateNative"] = 1600
        references["candidateNative"] = (meshes[1]["vertices"], meshes[1]["triangles"])
    geometry = {
        lod: grid_index(trimesh.Trimesh(np.asarray(points) * float(SCALE), faces, process=False))
        for lod, (points, faces) in references.items()
    }
    rows = []
    for (name, ordinal), identifier in slots.items():
        point_ids = np.flatnonzero(support_ids == identifier)
        fan = make_fan(scaled, point_ids)
        rows.append({
            "part": name,
            "occurrence": ordinal,
            "upperMillimetres": {lod: full_upper(index, scaled[fan], targets[lod]) for lod, index in geometry.items()},
        })
    return {
        "scope": "Candidate supports against both native references" if candidate else
                 "Frozen 936-face native archive for comparison",
        "roundingMetres": .02,
        "refinementTargetsMillimetres": targets,
        "fullUpperMillimetres": {lod: max(row["upperMillimetres"][lod] for row in rows) for lod in targets},
        "pieces": rows,
    }



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", choices=("archive-fit", "candidate-fit", "candidate-fidelity"))
    args = parser.parse_args()
    build()
    candidate = ORIGINAL / "SH048-candidate.json"
    if args.report == "candidate-fidelity":
        report = compare(candidate)
    else:
        report = envelope_fit(candidate if args.report == "candidate-fit" else None)
    sys.stdout.buffer.write((json.dumps(report, indent=2) + "\n").encode())


if __name__ == "__main__":
    main()
