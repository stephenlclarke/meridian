"""Build the project-authored SH060 native artwork candidate from authenticated mesh data."""

import argparse
from collections import Counter
import json
from pathlib import Path
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from prepare_banteng_candidate import closed, loft, pin

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh060"
INTEGRATION = ROOT / "integration/sh060-wombat"
PATH = ORIGINAL / "SH060-project-candidate.json"
KEEP_WELLS = frozenset({1, 2, 3, 4, 13, 14, 17, 20})
CLOSED_FACE_LIMIT = 512
LOFT = "Hull.loft / Hull.solid"


def source():
    pins = json.loads((INTEGRATION / "FROZEN-SOURCES.json").read_bytes())["paths"]
    original = (ORIGINAL / "resource.toml").read_bytes()
    archive = (ORIGINAL / "native-export.json").read_bytes()
    ownership = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    for name, data in (("resource.toml", original), ("native-export.json", archive)):
        expected = pins[name]
        if pin(data) != {key: expected[key] for key in ("bytes", "sha256")}:
            raise ValueError(f"Changed frozen Wombat source {name}")
    if pin(ownership)["sha256"] != "e224806e20761e2eeb17a5c5c30c5ce8397cb5ad98e54f4a4256c8c51040260d":
        raise ValueError("Changed frozen Wombat ownership")
    raw = tomllib.loads(original.decode())
    archived = json.loads(archive)
    if raw["id"] != "SH060" or raw["nativeDesign"] != archived["nativeDesign"]:
        raise ValueError("The Wombat archived sources differ")
    return raw["nativeDesign"], json.loads(ownership)["models"]["nativeDesign"]["parts"], archived["sourceSHA256"]


def build():
    raw, ownership, source_digest = source()
    candidate = {key: raw[key] for key in ("materials", "license", "appearanceLicense", "appearanceSource",
                                           "paletteName", "textures")}
    candidate["source"] = "Project-authored Wombat native artwork candidate from the frozen original mesh"
    candidate.update({"vertices": [], "faces": [], "faceMaterials": [], "faceTextures": []})
    parts, omitted, occurrences = [], [], Counter()
    for index, original in enumerate(ownership):
        name, start, first = original["name"], original["vertexStart"], original["faceStart"]
        occurrences[name] += 1
        if original["creatorMethod"] == "Mesh.well" and index not in KEEP_WELLS:
            omitted.append(index)
            continue
        span = slice(first, first + original["faceCount"])
        points = raw["vertices"][start:start + original["vertexCount"]]
        faces = [[vertex - start for vertex in row] for row in raw["faces"][span]]
        materials, textures = raw["faceMaterials"][span], raw["faceTextures"][span]
        mode = "original"
        if original["creatorMethod"] == LOFT:
            points, faces, materials, textures = loft(points, materials, textures)
            mode = "resampled original longitudinal stations"
        closed(points, faces)
        start = len(candidate["vertices"])
        parts.append({
            "name": name, "originalPartIndex": index, "originalOccurrence": occurrences[name], "mode": mode,
            "vertexStart": start, "vertexCount": len(points),
            "faceStart": len(candidate["faces"]), "faceCount": len(faces),
        })
        candidate["vertices"].extend(points)
        candidate["faces"].extend([[start + corner for corner in face] for face in faces])
        candidate["faceMaterials"].extend(materials)
        candidate["faceTextures"].extend(textures)
    if len(parts) != 28 or len(omitted) != 4 or len(candidate["faces"]) != 496 or len(candidate["faces"]) > CLOSED_FACE_LIMIT:
        raise ValueError("The Wombat project-authored candidate exceeds its visual budget")
    return {"id": "SH060", "status": "PROJECT_AUTHORED_ARTWORK_CANDIDATE",
            "originalSourceSHA256": source_digest, "archivedNativeFaces": len(raw["faces"]), "nativeFaceLimit": CLOSED_FACE_LIMIT,
            "omittedBlindWellParts": omitted, "parts": parts, "nativeCandidate": candidate}


def encode_candidate():
    return (json.dumps(build(), separators=(",", ":"), allow_nan=False) + "\n").encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    checked_directory(ORIGINAL)
    target = checked_file(PATH)
    output = encode_candidate()
    if args.check:
        if target.read_bytes() != output:
            raise ValueError("Changed project-authored Wombat candidate")
    else:
        replace_output(target, output)


if __name__ == "__main__":
    main()
