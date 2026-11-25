"""Build the project-authored SH053 native artwork candidate from authenticated mesh data."""

import argparse
from collections import Counter
import json
from pathlib import Path
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from prepare_banteng_candidate import closed, loft, pin

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh053"
INTEGRATION = ROOT / "integration/sh053-caracal-scout"
PATH = ORIGINAL / "SH053-project-candidate.json"
KEEP_WELLS = frozenset({1, 2, 9, 13, 19, 23})
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
            raise ValueError(f"Changed frozen Caracal Scout source {name}")
    if pin(ownership)["sha256"] != "0d497c935c76f3e62da05c4d20160e8149feee5cfbee784dae217d15c9808739":
        raise ValueError("Changed frozen Caracal Scout ownership")
    raw = tomllib.loads(original.decode())
    archived = json.loads(archive)
    if raw["id"] != "SH053" or raw["nativeDesign"] != archived["nativeDesign"]:
        raise ValueError("The Caracal Scout archived sources differ")
    return raw["nativeDesign"], json.loads(ownership)["models"]["nativeDesign"]["parts"], archived["sourceSHA256"]


def build():
    raw, ownership, source_digest = source()
    candidate = {key: raw[key] for key in ("materials", "license", "appearanceLicense", "appearanceSource",
                                           "paletteName", "textures")}
    candidate["source"] = "Project-authored Caracal Scout native artwork candidate from the frozen original mesh"
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
    if len(parts) != 25 or len(omitted) != 14 or not 0 < len(candidate["faces"]) <= CLOSED_FACE_LIMIT:
        raise ValueError("The Caracal Scout project-authored candidate exceeds its visual budget")
    return {"id": "SH053", "status": "PROJECT_AUTHORED_ARTWORK_CANDIDATE",
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
            raise ValueError("Changed project-authored Caracal Scout candidate")
    else:
        replace_output(target, output)


if __name__ == "__main__":
    main()
