"""Encode the authenticated, already calibrated SH008 detailed presentation."""
import argparse
from dataclasses import dataclass
import hashlib
import json
import math
import struct
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESOURCE_SHA = "aafcb5dfec2cb5d141316756daae267888bf3541383f4f155c2561a4d7dd0bb7"
TEXTURES = ["l337-patrol-diffuse", "l337-strike-diffuse", "l337-service-details"]


@dataclass(frozen=True)
class VisualSource:
    hull: str
    resource_sha: str
    detailed_vertices: int
    detailed_faces: int
    native_vertices: int
    geometry_magic: bytes
    visual_magic: bytes


SH008 = VisualSource("SH008", RESOURCE_SHA, 2072, 3484, 536, b"MSH008V1", b"GOSHVIZ1")


def encode_for_hull(source, points, triangles, spec):
    if hashlib.sha256(source).hexdigest() != spec.resource_sha:
        raise ValueError(f"Changed selected {spec.hull} resource")
    resource = tomllib.loads(source.decode())
    if resource["id"] != spec.hull:
        raise ValueError("Changed original hull identity")
    model = resource["design"]
    return encode_model(model, points, triangles, spec.geometry_magic,
                        (spec.detailed_vertices, spec.native_vertices, spec.detailed_faces), spec.visual_magic)


def encode_model(model, points, triangles, source_magic, counts, visual_magic, *, allow_untextured=False):
    detailed_vertices, native_vertices, face_count = counts
    if (len(model["vertices"]), len(model["faces"])) != (detailed_vertices, face_count):
        raise ValueError("Changed detailed face set")
    if [t["asset"] for t in model["textures"]] != TEXTURES:
        raise ValueError("Changed source texture ordering")
    packed_points = b"".join(struct.pack("<3d", *p) for p in model["vertices"])
    packed_faces = b"".join(struct.pack("<3I", *face) for face in model["faces"])
    expected_header = struct.pack("<8sII", source_magic, detailed_vertices, native_vertices)
    if points[:16] != expected_header or packed_points != points[16:16 + len(packed_points)]:
        raise ValueError("Display positions differ from the reviewed detailed collision input")
    if packed_faces != triangles[:len(packed_faces)]:
        raise ValueError("Display winding differs from the reviewed detailed collision input")
    output = bytearray(struct.pack("<8sII", visual_magic, face_count * 3, 56))
    for face, material_index, mapping in zip(
        model["faces"], model["faceMaterials"], model["faceTextures"], strict=True
    ):
        vertices = [model["vertices"][i] for i in face]
        a = [q - p for p, q in zip(vertices[0], vertices[1], strict=True)]
        b = [q - p for p, q in zip(vertices[0], vertices[2], strict=True)]
        normal = [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
        length = math.sqrt(sum(n*n for n in normal))
        texture = mapping["texture"]
        if allow_untextured and texture == -1:
            texture = len(TEXTURES)  # Explicit no-map sentinel, not a fourth texture.
        elif not 0 <= texture < len(TEXTURES):
            raise ValueError("Invalid original face")
        if length <= 0 or not math.isfinite(length):
            raise ValueError("Invalid original face")
        material = model["materials"][material_index]
        color = [(material["colour"] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
        for vertex, uv in zip(vertices, mapping["uv"], strict=True):
            output.extend(struct.pack(
                "<12fII", *vertex, *(n / length for n in normal), *color, *uv,
                texture, int(material["emissive"])
            ))
    return bytes(output)


def encode(source, points, triangles):
    return encode_for_hull(source, points, triangles, SH008)


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
