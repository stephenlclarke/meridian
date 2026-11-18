"""Encode the authenticated, already calibrated SH008 detailed presentation."""
import argparse
import hashlib
import json
import math
import struct
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESOURCE_SHA = "aafcb5dfec2cb5d141316756daae267888bf3541383f4f155c2561a4d7dd0bb7"
TEXTURES = ["l337-patrol-diffuse", "l337-strike-diffuse", "l337-service-details"]


def encode(source, points, triangles):
    if hashlib.sha256(source).hexdigest() != RESOURCE_SHA:
        raise ValueError("Changed selected SH008 resource")
    model = tomllib.loads(source.decode())["design"]
    if (len(model["vertices"]), len(model["faces"])) != (2072, 3484):
        raise ValueError("Changed detailed face set")
    if [t["asset"] for t in model["textures"]] != TEXTURES:
        raise ValueError("Changed source texture ordering")
    packed_points = b"".join(struct.pack("<3d", *p) for p in model["vertices"])
    packed_faces = b"".join(struct.pack("<3I", *face) for face in model["faces"])
    if points[:16] != struct.pack("<8sII", b"MSH008V1", 2072, 536) or packed_points != points[16:16 + len(packed_points)]:
        raise ValueError("Display positions differ from the reviewed detailed collision input")
    if packed_faces != triangles[:len(packed_faces)]:
        raise ValueError("Display winding differs from the reviewed detailed collision input")
    output = bytearray(struct.pack("<8sII", b"GOSHVIZ1", 3484 * 3, 56))
    for face, material_index, mapping in zip(
        model["faces"], model["faceMaterials"], model["faceTextures"], strict=True
    ):
        vertices = [model["vertices"][i] for i in face]
        a = [q - p for p, q in zip(vertices[0], vertices[1], strict=True)]
        b = [q - p for p, q in zip(vertices[0], vertices[2], strict=True)]
        normal = [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
        length = math.sqrt(sum(n*n for n in normal))
        if length <= 0 or not math.isfinite(length) or not 0 <= mapping["texture"] < len(TEXTURES):
            raise ValueError("Invalid original face")
        material = model["materials"][material_index]
        color = [(material["colour"] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
        for vertex, uv in zip(vertices, mapping["uv"], strict=True):
            output.extend(struct.pack(
                "<12fII", *vertex, *(n / length for n in normal), *color, *uv,
                mapping["texture"], int(material["emissive"])
            ))
    return bytes(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    folder = ROOT / "assets/fleet/sh008"
    encoded = encode(*((folder / name).read_bytes() for name in (
        "resource.toml", "source.vertices", "source-triangles.bin"
    )))
    if args.check:
        if (folder / "visual.bin").read_bytes() != encoded:
            raise ValueError("SH008 packed visual differs from exact source")
    else:
        (folder / "visual.bin").write_bytes(encoded)
    print(json.dumps({"faces": 3484, "vertices": 10452, "bytes": len(encoded),
                      "sha256": hashlib.sha256(encoded).hexdigest()}))


if __name__ == "__main__":
    main()
