"""Measure the separate SH053 project candidate and both original references."""

import argparse
import json
import sys
import tomllib

import numpy as np
import trimesh

from caracal_scout_art_mesh import SCALE, assemble
from measure_banteng_geometry import adaptive_upper, section, solid, upper
from measure_lanner_geometry import load_reference_records, validated_reference_fan
from prepare_caracal_scout_geometry import INTEGRATION, ORIGINAL, build
from vulture_mk_i_exact_fit import grid_index


REFERENCE_FANS_SHA = "57d334ca12d86a668931951936c113f6c22c7006ddcc1487b7b4fca633d2b2d6"
REFERENCE_DIAGNOSTIC_SHA = "cea0652cd77665eb472e89d35f59bfbbf8d9271b1fc2c3e6bd2254065af76539"


def surface(section_data):
    points, faces = section_data[:2]
    return trimesh.Trimesh(np.asarray(points) * float(SCALE), faces, process=False)


def compare(candidate_path):
    original = tomllib.loads((ORIGINAL / "resource.toml").read_text())["nativeDesign"]
    parts = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]["nativeDesign"]["parts"]
    derivative = json.loads(candidate_path.read_bytes())
    candidate = derivative["nativeCandidate"]
    remaining = {item["originalPartIndex"]: item for item in derivative["parts"]}
    omitted = set(derivative["omittedBlindWellParts"])
    assert len(original["faces"]) == 1380 and len(candidate["faces"]) == 492
    assert set(remaining) | omitted == set(range(39)) and not set(remaining) & omitted
    whole_candidate = trimesh.Trimesh(np.asarray(candidate["vertices"]) * float(SCALE), candidate["faces"], process=False)
    whole_archive = trimesh.Trimesh(np.asarray(original["vertices"]) * float(SCALE), original["faces"], process=False)
    candidate_index = grid_index(whole_candidate)
    archive_index = grid_index(whole_archive)
    rows = []
    for index, part in enumerate(parts):
        frozen = section(original, part)
        old = surface(frozen)
        solid(old)
        if index in omitted:
            if part["creatorMethod"] != "Mesh.well":
                raise ValueError("A Caracal Scout omission is not a blind well")
            upper_bounds = {"archiveToCandidate": upper(old, candidate_index)}
            mode = "omitted independent shallow well"
        else:
            choice = remaining[index]
            proposed = section(candidate, choice)
            new = surface(proposed)
            solid(new)
            if set(proposed[2]) != set(frozen[2]):
                raise ValueError("Caracal Scout material identifiers changed")
            mode = choice["mode"]
            if mode == "original":
                if proposed != frozen:
                    raise ValueError("A fixed Caracal Scout part differs from the source")
                upper_bounds = {"candidateToArchive": 0, "archiveToCandidate": 0}
            else:
                if part["creatorMethod"] != "Hull.loft / Hull.solid":
                    raise ValueError("Caracal Scout resampling affected an unrelated part")
                rows_original = [frozen[0][i:i + 12] for i in range(0, len(frozen[0]), 12)]
                rows_candidate = [proposed[0][i:i + 4] for i in range(0, len(proposed[0]), 4)]
                for first, second in zip(rows_original, rows_candidate, strict=True):
                    if second != [first[i] for i in (0, 3, 6, 9)]:
                        raise ValueError("Caracal Scout retained ring station or extrema moved")
                upper_bounds = {"candidateToArchive": upper(new, archive_index),
                                "archiveToCandidate": upper(old, candidate_index)}
        rows.append({"index": index, "name": part["name"], "mode": mode, "verifiedUpperMillimetres": upper_bounds})
    return {
        "originalArchivedNativeFaces": len(original["faces"]),
        "candidateNativeFaces": len(candidate["faces"]),
        "fixedParts": sum(item["mode"] == "original" for item in rows),
        "resampledParts": sum(item["mode"] == "resampled original longitudinal stations" for item in rows),
        "omittedBlindWellParts": len(omitted),
        "fullUpperMillimetres": {
            key: max(row["verifiedUpperMillimetres"].get(key, 0) for row in rows)
            for key in ("candidateToArchive", "archiveToCandidate")
        },
        "parts": rows,
    }


def reference_records(mode, scaled, support_ids, count):
    return load_reference_records(INTEGRATION / "REFERENCE-FANS.json", REFERENCE_FANS_SHA, REFERENCE_DIAGNOSTIC_SHA,
                                  mode, scaled, support_ids, count, "Caracal Scout")


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
    records = reference_records("candidate" if candidate else "archive", scaled, support_ids, len(slots))
    fans = [validated_reference_fan(record, key, scaled, np.flatnonzero(support_ids == identifier), "Caracal Scout")
            for (key, identifier), record in zip(slots.items(), records, strict=True)]
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
    for (name, ordinal), fan in zip(slots, fans, strict=True):
        rows.append({
            "part": name,
            "occurrence": ordinal,
            "upperMillimetres": {lod: adaptive_upper(index, scaled[fan], targets[lod]) for lod, index in geometry.items()},
        })
    return {
        "scope": "Candidate supports against both native references" if candidate else
                 "Frozen 1380-face native archive for comparison",
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
    candidate = ORIGINAL / "SH053-project-candidate.json"
    if args.report == "candidate-fidelity":
        report = compare(candidate)
    else:
        report = envelope_fit(candidate if args.report == "candidate-fit" else None)
    sys.stdout.buffer.write((json.dumps(report, indent=2) + "\n").encode())


if __name__ == "__main__":
    main()
