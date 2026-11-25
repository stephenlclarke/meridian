"""Build the project-authored SH052 native artwork candidate from authenticated mesh data."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "assets/fleet/sh052"
INTEGRATION = ROOT / "integration/sh052-banteng"
PATH = ORIGINAL / "SH052-project-candidate.json"
KEEP_WELLS = frozenset({1, 2, 3, 4, 9, 13, 19, 23})
CLOSED_FACE_LIMIT = 512
ORIGINAL_SIDES = 12
CANDIDATE_SIDES = (0, 3, 6, 9)
LOFT = "Hull.loft / Hull.solid"


def pin(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def source():
    pins = json.loads((INTEGRATION / "FROZEN-SOURCES.json").read_bytes())["paths"]
    original = (ORIGINAL / "resource.toml").read_bytes()
    archive = (ORIGINAL / "native-export.json").read_bytes()
    ownership = (INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes()
    for name, data in (("resource.toml", original), ("native-export.json", archive)):
        expected = pins[name]
        if pin(data) != {key: expected[key] for key in ("bytes", "sha256")}:
            raise ValueError(f"Changed frozen Banteng source {name}")
    if pin(ownership)["sha256"] != "2345b4e5f585a564e6dab623a10226208b3797135705229f9757a0646e27ed7a":
        raise ValueError("Changed frozen Banteng ownership")
    raw = tomllib.loads(original.decode())
    archived = json.loads(archive)
    if raw["id"] != "SH052" or raw["nativeDesign"] != archived["nativeDesign"]:
        raise ValueError("The Banteng archived sources differ")
    return raw["nativeDesign"], json.loads(ownership)["models"]["nativeDesign"]["parts"], archived["sourceSHA256"]


def closed(points, faces):
    used = {corner for face in faces for corner in face}
    if used != set(range(len(points))):
        raise ValueError("Resampled Banteng part references invalid points")
    edges = Counter((a, b) for a, b, c in faces for a, b in ((a, b), (b, c), (c, a)))
    if not all(count == edges[(b, a)] == 1 for (a, b), count in edges.items()):
        raise ValueError("Resampled Banteng part is open or changes winding")


def triangulate(rings):
    def cap(i, reversed_order=False):
        row = list(range(i * 4, (i + 1) * 4))
        if reversed_order:
            row.reverse()
        return [[row[0], row[1], row[2]], [row[0], row[2], row[3]]]

    triangles = cap(0, reversed_order=True)
    for ring in range(rings - 1):
        for side in range(4):
            this = ring * 4 + side
            neighbour = ring * 4 + (side + 1) % 4
            triangles.extend(([this, neighbour, neighbour + 4], [this, neighbour + 4, this + 4]))
    return triangles + cap(rings - 1)


def project_uv(points, face, asset):
    a, b, c = (points[index] for index in face)
    ab = [x - y for x, y in zip(b, a, strict=True)]
    ac = [x - y for x, y in zip(c, a, strict=True)]
    normal = tuple(ab[i] * ac[j] - ab[j] * ac[i] for i, j in ((1, 2), (2, 0), (0, 1)))
    if not any(normal):
        raise ValueError("Banteng candidate has a degenerate face")
    projection = ((2, 1), (0, 2), (0, 1))[max(range(3), key=lambda i: abs(normal[i]))]
    return {"texture": asset, "uv": [[round(points[index][axis] * .85, 6) for axis in projection]
                                     for index in face]}


def loft(points, material, appearance):
    rings, remainder = divmod(len(points), ORIGINAL_SIDES)
    if remainder or rings not in (2, 5):
        raise ValueError("Changed original Banteng longitudinal station count")
    candidate = []
    z_values = []
    for offset in range(0, len(points), ORIGINAL_SIDES):
        source_ring = points[offset:offset + ORIGINAL_SIDES]
        selected = [source_ring[side] for side in CANDIDATE_SIDES]
        z = {p[2] for p in source_ring}
        if len(z) != 1 or any((min(v), max(v)) != (min(w), max(w))
                               for v, w in zip(zip(*source_ring), zip(*selected), strict=True)):
            raise ValueError("Banteng ring extrema are not preserved")
        z_values.append(z.pop())
        candidate.extend(selected)
    if z_values != sorted(set(z_values)):
        raise ValueError("Banteng axial stations are not strictly ordered")
    face_materials = set(material)
    texture_assets = {mapping["texture"] for mapping in appearance}
    if len(face_materials) != 1 or len(texture_assets) != 1:
        raise ValueError("Resampled Banteng loft has mixed original material assignments")
    faces = triangulate(rings)
    closed(candidate, faces)
    return candidate, faces, [face_materials.pop()] * len(faces), [
        project_uv(candidate, face, next(iter(texture_assets))) for face in faces
    ]


def build():
    raw, ownership, source_digest = source()
    candidate = {key: raw[key] for key in ("materials", "license", "appearanceLicense", "appearanceSource",
                                           "paletteName", "textures")}
    candidate["source"] = "Project-authored Banteng native artwork candidate from the frozen original mesh"
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
    if len(parts) != 23 or len(omitted) != 12 or not 0 < len(candidate["faces"]) <= CLOSED_FACE_LIMIT:
        raise ValueError("The Banteng project-authored candidate exceeds its visual budget")
    return {"id": "SH052", "status": "PROJECT_AUTHORED_ARTWORK_CANDIDATE",
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
            raise ValueError("Changed project-authored Banteng candidate")
    else:
        replace_output(target, output)


if __name__ == "__main__":
    main()
