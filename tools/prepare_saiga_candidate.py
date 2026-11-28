"""Build the project-authored SH061 native artwork candidate from authenticated mesh data."""

import argparse
from collections import Counter
import json
from pathlib import Path
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from prepare_banteng_candidate import closed, loft, pin, project_uv

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh061"
INTEGRATION = ROOT / "integration/sh061-saiga"
PATH = ORIGINAL / "SH061-project-candidate.json"
KEEP_WELLS = frozenset({1, 2, 8, 9, 13, 16})
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
            raise ValueError(f"Changed frozen Saiga source {name}")
    if pin(ownership)["sha256"] != "00205460ab315a511c3040f992bacbfb5f4b81bec3b97b0279dcb2902c81011d":
        raise ValueError("Changed frozen Saiga ownership")
    raw = tomllib.loads(original.decode())
    archived = json.loads(archive)
    if raw["id"] != "SH061" or raw["nativeDesign"] != archived["nativeDesign"]:
        raise ValueError("The Saiga archived sources differ")
    return raw["nativeDesign"], json.loads(ownership)["models"]["nativeDesign"]["parts"], archived["sourceSHA256"]


def collar(points, materials, appearance):
    """Keep the original collar axis extrema and its open, genus-one topology."""
    if len(points) != 24 * 8:
        raise ValueError("Changed Saiga native collar stations")
    major, minor = 8, 4
    reduced = [points[station * 8 + side] for station in range(0, 24, 3) for side in range(0, 8, 2)]
    for original, derived in zip(zip(*points), zip(*reduced), strict=True):
        if (min(original), max(original)) != (min(derived), max(derived)):
            raise ValueError("Saiga collar axis extremum moved")
    faces = []
    for ring in range(major):
        for side in range(minor):
            a = ring * minor + side
            b = ((ring + 1) % major) * minor + side
            c = ((ring + 1) % major) * minor + (side + 1) % minor
            d = ring * minor + (side + 1) % minor
            faces.extend(([a, b, c], [a, c, d]))
    fixed_material = set(materials)
    fixed_texture = {entry["texture"] for entry in appearance}
    if len(fixed_material) != 1 or len(fixed_texture) != 1:
        raise ValueError("Saiga collar mixes image or material selections")
    texture = fixed_texture.pop()
    return reduced, faces, [fixed_material.pop()] * len(faces), [project_uv(reduced, face, texture) for face in faces]


def build():
    raw, ownership, source_digest = source()
    candidate = {key: raw[key] for key in ("materials", "license", "appearanceLicense", "appearanceSource",
                                           "paletteName", "textures")}
    candidate["source"] = "Project-authored Saiga native artwork candidate from the frozen original mesh"
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
        elif original["creatorMethod"] == "Mesh.torus / Mesh.raw":
            points, faces, materials, textures = collar(points, materials, textures)
            mode = "resampled original toroidal rings"
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
    if len(parts) != 22 or len(omitted) != 6 or len(candidate["faces"]) != CLOSED_FACE_LIMIT:
        raise ValueError("The Saiga project-authored candidate exceeds its visual budget")
    return {"id": "SH061", "status": "PROJECT_AUTHORED_ARTWORK_CANDIDATE",
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
            raise ValueError("Changed project-authored Saiga candidate")
    else:
        replace_output(target, output)


if __name__ == "__main__":
    main()
