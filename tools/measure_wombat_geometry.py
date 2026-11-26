"""Measure the separate SH060 project candidate and both original references."""

import argparse
from fractions import Fraction
import json
from math import isqrt
import sys
import tomllib

import numpy as np
import trimesh

from wombat_art_mesh import SCALE, assemble
from measure_banteng_geometry import adaptive_upper, section, solid, upper
from measure_lanner_geometry import load_reference_records, validated_reference_fan
from prepare_wombat_geometry import INTEGRATION, ORIGINAL, build
from vulture_mk_i_exact_fit import grid_index


REFERENCE_FANS_SHA = "121ec763dc8227253c859a607645b3745947ee1ab0aeb75e6f09b753cb36fc61"
REFERENCE_DIAGNOSTIC_SHA = "88060109ac37f96f2cf92ceefea68841d1c56d456e82049fe58b28906940ec8a"


def surface(section_data):
    points, faces = section_data[:2]
    return trimesh.Trimesh(np.asarray(points) * float(SCALE), faces, process=False)


def outward_mm(squared_distance):
    distance = squared_distance * 1_000_000
    lower = isqrt(distance.numerator // distance.denominator)
    return lower + (lower * lower < distance)


def well_depth(points):
    if len(points) != 12:
        raise ValueError("Changed Wombat blind well topology")
    squared = [
        sum((Fraction(str(a)) - Fraction(str(b)))**2 for a, b in zip(mouth, floor, strict=True)) * SCALE**2
        for mouth, floor in zip(points[:4], points[-4:], strict=True)
    ]
    if min(squared) <= 0:
        raise ValueError("Changed Wombat blind well depth")
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


def compare(candidate_path):
    source = tomllib.loads((ORIGINAL / "resource.toml").read_text())
    original = source["nativeDesign"]
    detailed = source["design"]
    parts = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]["nativeDesign"]["parts"]
    derivative = json.loads(candidate_path.read_bytes())
    candidate = derivative["nativeCandidate"]
    remaining = {item["originalPartIndex"]: item for item in derivative["parts"]}
    omitted = set(derivative["omittedBlindWellParts"])
    assert len(original["faces"]) == 1024 and len(candidate["faces"]) == 496
    assert set(remaining) | omitted == set(range(32)) and not set(remaining) & omitted
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
        solid(old)
        if index in omitted:
            if part["creatorMethod"] != "Mesh.well":
                raise ValueError("A Wombat omission is not a blind well")
            upper_bounds = {"archiveToCandidate": upper(old, candidate_index)}
            depths[str(index)] = well_depth(frozen[0])
            mode = "omitted independent inboard pod blind well"
        else:
            choice = remaining[index]
            proposed = section(candidate, choice)
            new = surface(proposed)
            solid(new)
            if set(proposed[2]) != set(frozen[2]):
                raise ValueError("Wombat material identifiers changed")
            mode = choice["mode"]
            if mode == "original":
                if proposed != frozen:
                    raise ValueError("A fixed Wombat part differs from the source")
                upper_bounds = {"candidateToArchive": 0, "archiveToCandidate": 0}
            else:
                if part["creatorMethod"] != "Hull.loft / Hull.solid":
                    raise ValueError("Wombat resampling affected an unrelated part")
                rows_original = [frozen[0][i:i + 12] for i in range(0, len(frozen[0]), 12)]
                rows_candidate = [proposed[0][i:i + 4] for i in range(0, len(proposed[0]), 4)]
                for first, second in zip(rows_original, rows_candidate, strict=True):
                    if second != [first[i] for i in (0, 3, 6, 9)]:
                        raise ValueError("Wombat retained ring station or extrema moved")
                upper_bounds = {"candidateToArchive": upper(new, archive_index),
                                "archiveToCandidate": upper(old, candidate_index)}
        rows.append({"index": index, "name": part["name"], "mode": mode, "verifiedUpperMillimetres": upper_bounds})
    archived_projection = projection_deltas(original, candidate)
    if any(any(row) for row in archived_projection["axisExtremaAbsoluteDeltaMillimetres"]):
        raise ValueError("Wombat candidate moved the archived native axis extrema")
    return {
        "originalArchivedNativeFaces": len(original["faces"]),
        "candidateNativeFaces": len(candidate["faces"]),
        "fixedParts": sum(item["mode"] == "original" for item in rows),
        "resampledParts": sum(item["mode"] == "resampled original longitudinal stations" for item in rows),
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
    mode = "candidate" if candidate else "archive"
    records = load_reference_records(INTEGRATION / "REFERENCE-FANS.json", REFERENCE_FANS_SHA,
                                     REFERENCE_DIAGNOSTIC_SHA, mode, scaled, support_ids, len(slots), "Wombat")
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
    for ((name, ordinal), identifier), record in zip(slots.items(), records, strict=True):
        point_ids = np.flatnonzero(support_ids == identifier)
        fan = validated_reference_fan(record, (name, ordinal), scaled, point_ids, "Wombat")
        rows.append({
            "part": name,
            "occurrence": ordinal,
            "upperMillimetres": {lod: adaptive_upper(index, scaled[fan], targets[lod]) for lod, index in geometry.items()},
        })
    return {
        "scope": "Candidate supports against both native references" if candidate else
                 "Frozen 1024-face native archive for comparison",
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
    candidate = ORIGINAL / "SH060-project-candidate.json"
    if args.report == "candidate-fidelity":
        report = compare(candidate)
    else:
        report = envelope_fit(candidate if args.report == "candidate-fit" else None)
    sys.stdout.buffer.write((json.dumps(report, indent=2) + "\n").encode())


if __name__ == "__main__":
    main()
