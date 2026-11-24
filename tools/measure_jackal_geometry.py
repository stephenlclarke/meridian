"""Measure SH051 candidate fidelity and rounded supports against the archived art."""

import argparse
from collections import Counter
import json
from math import isqrt
import sys
import tomllib

import numpy as np
import trimesh

from jackal_art_mesh import SCALE, assemble
from measure_goshawk_surface_fit import GRID
from measure_lanner_geometry import make_fan
from prepare_jackal_geometry import INTEGRATION, ORIGINAL, build
from vulture_mk_i_exact_fit import full_upper, grid_index


EXPECTED_CHANGED = {0, 7, 12, 17, 20, 21, 23, 24}


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


def directed_upper(source, target):
    index = grid_index(target)
    lower = 0
    for point in np.rint(source.vertices * GRID).astype(np.int64).tolist():
        (numerator, denominator), _ = index.nearest(tuple(point))
        lower = max(lower, isqrt(numerator * 1_000_000 // (denominator * GRID**2)))
    refinement_target = max(500, (lower // 250 + 2) * 250)
    return full_upper(index, source.triangles, refinement_target)


def compare(candidate_path):
    original = tomllib.loads((ORIGINAL / "resource.toml").read_text())["nativeDesign"]
    ownership = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]["nativeDesign"]["parts"]
    candidate_doc = json.loads(candidate_path.read_bytes())
    candidate, parts = candidate_doc["nativeCandidate"], candidate_doc["parts"]
    if len(original["faces"]) != 940 or not 0 < len(candidate["faces"]) <= 512:
        raise ValueError("Jackal original or candidate face budget differs")
    if len(parts) != len(ownership) or len(parts) != 25:
        raise ValueError("Jackal candidate did not preserve all original parts")
    rows = []
    for index, (old, new) in enumerate(zip(ownership, parts, strict=True)):
        if old["name"] != new["name"]:
            raise ValueError("Jackal candidate part identity changed")
        src, proposed = section(original, old), section(candidate, new)
        old_geom = closed_surface(src[0], src[1])
        new_geom = closed_surface(proposed[0], proposed[1])
        if set(src[2]) != set(proposed[2]):
            raise ValueError("A Jackal candidate part changed its original material set")
        if src == proposed:
            if index in EXPECTED_CHANGED:
                raise ValueError("Candidate did not resample a declared curved part")
            bounds = {"candidateToArchive": 0, "archiveToCandidate": 0}
        else:
            if index not in EXPECTED_CHANGED:
                raise ValueError("A fixed Jackal candidate part changed source coordinates, faces, materials or UVs")
            bounds = {
                "candidateToArchive": directed_upper(new_geom, old_geom),
                "archiveToCandidate": directed_upper(old_geom, new_geom),
            }
        rows.append({"index": index, "name": old["name"], "originalFaces": len(src[1]),
                     "candidateFaces": len(proposed[1]), "verifiedUpperMillimetres": bounds})
    return {"originalArchivedNativeFaces": len(original["faces"]), "candidateNativeFaces": len(candidate["faces"]),
            "archiveWithin512FaceBudget": False, "nativeCandidateWithin512FaceBudget": True, "parts": rows}


def adaptive_upper(index, fan, target):
    centroids = np.mean(fan, axis=1)
    vertices = np.unique(fan.reshape(-1, 3), axis=0)
    lower = 0
    for point in np.rint(np.vstack([centroids, vertices]) * GRID).astype(np.int64).tolist():
        (num, den), _ = index.nearest(tuple(point))
        lower = max(lower, isqrt(num * 1_000_000 // (den * GRID**2)))
    refined = max(target, (lower // 250 + 3) * 250)
    return full_upper(index, fan, refined)


def envelope_fit(candidate_path):
    source = tomllib.loads((ORIGINAL / "resource.toml").read_text())
    parts = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
    candidate = json.loads(candidate_path.read_bytes()) if candidate_path else None
    native = (
        (candidate["nativeCandidate"], candidate["parts"]) if candidate
        else (source["nativeDesign"], parts["nativeDesign"]["parts"])
    )
    slots, meshes = assemble((source["design"], parts["design"]["parts"]), native)
    scaled = np.array(meshes[0]["vertices"] + meshes[1]["vertices"]) * float(SCALE)
    support_ids = np.array(meshes[0]["groups"] + meshes[1]["groups"])
    targets = {"detailed": 1800, "archiveNative": 1800}
    original_native = source["nativeDesign"]
    references = {
        "detailed": (meshes[0]["vertices"], meshes[0]["triangles"]),
        "archiveNative": (original_native["vertices"], original_native["faces"]),
    }
    if candidate:
        targets["candidateNative"] = 2300
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
            "upperMillimetres": {lod: adaptive_upper(index, scaled[fan], targets[lod]) for lod, index in geometry.items()},
        })
    return {
        "scope": "Candidate supports against both native references" if candidate else
                 "Frozen 940-face native archive for comparison",
        "roundingMetres": .02,
        "minimumRefinementTargetsMillimetres": targets,
        "fullUpperMillimetres": {lod: max(row["upperMillimetres"][lod] for row in rows) for lod in targets},
        "pieces": rows,
    }



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", choices=("archive-fit", "candidate-fit", "candidate-fidelity"))
    args = parser.parse_args()
    build()
    candidate = ORIGINAL / "SH051-candidate.json"
    if args.report == "candidate-fidelity":
        report = compare(candidate)
    else:
        report = envelope_fit(candidate if args.report == "candidate-fit" else None)
    sys.stdout.buffer.write((json.dumps(report, indent=2) + "\n").encode())


if __name__ == "__main__":
    main()
