"""Reproduce the first display mesh from the exact immutable SH025 source file."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib


SOURCE_SHA256 = "aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883"
SOURCE_HEAD = "a11fe16924d87d06021816f9037952afcbac08db"
SOURCE_PATH = "Sources/L337Core/Resources/Ships/SH025.toml"
ROOT = Path(__file__).resolve().parents[1]


def convert(raw: bytes) -> bytes:
    """Refuse a different reference and preserve all source triangles in order."""
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("SH025 source hash differs from the selected immutable reference")
    source = tomllib.loads(raw.decode())
    design = source["design"]
    if design["license"] != "MIT" or design["appearanceLicense"] != "MIT":
        raise ValueError("Selected source notices do not match this import")
    vertices = design["vertices"]
    faces = design["faces"]
    materials = design["materials"]
    assignments = design["faceMaterials"]
    if len(faces) != len(assignments):
        raise ValueError("Every source triangle needs its original material assignment")
    output = bytearray()
    for face, material_id in zip(faces, assignments, strict=True):
        if len(face) != 3 or not 0 <= material_id < len(materials):
            raise ValueError("Only valid source triangles are supported")
        color = materials[material_id]["colour"]
        # Source stores RGBA as 0xRRGGBBAA. Texture/UV support is a later renderer slice.
        rgb = [((color >> shift) & 255) / 255 for shift in [24, 16, 8]]
        # Source palette channels are sRGB; the GPU surface expects linear shader output.
        linear = [c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in rgb]
        for index in face:
            if not 0 <= index < len(vertices):
                raise ValueError("Source triangle index is out of range")
            position = vertices[index]
            if len(position) != 3 or not all(math.isfinite(x) for x in position):
                raise ValueError("Source position is not finite 3D data")
            output.extend(struct.pack("<6f", *position, *linear))
    return bytes(output)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Exact SH025.toml from the pinned reference")
    parser.add_argument("--check", action="store_true", help="Compare without writing outputs")
    args = parser.parse_args()
    raw = args.source.read_bytes()
    output = convert(raw)
    destination = ROOT / "assets/legacy/sh025.mesh"
    manifest = {
        "sourceRevision": SOURCE_HEAD,
        "sourcePath": SOURCE_PATH,
        "sourceSHA256": SOURCE_SHA256,
        "sourceBytes": len(raw),
        "output": "assets/legacy/sh025.mesh",
        "outputSHA256": hashlib.sha256(output).hexdigest(),
        "outputBytes": len(output),
        "triangles": len(output) // 72,
        "format": "Triangle-list LE f32 xyz + linear RGB, 24 bytes per vertex",
        "transform": "Source triangle order/xyz unchanged; expand indices; sRGB palette to linear RGB",
        "coordinateConvention": "+Z forward, +Y up; source SH025 calibration retained as estimate",
        "license": "MIT; source design and appearance declarations; retained top-level notice",
        "notice": "assets/legacy/Elite-L337-MIT.txt",
        "scope": "Display-only prototype. No UV/textures, collision mesh or hull admission claim.",
        "importer": "tools/import_ship.py",
    }
    manifest_path = ROOT / "assets/legacy/manifest.json"
    encoded = (json.dumps(manifest, indent=2) + "\n").encode()
    if args.check:
        if destination.read_bytes() != output or manifest_path.read_bytes() != encoded:
            raise ValueError("Committed mesh or manifest differs from reproducible import")
        print("SH025 reference mesh and manifest match")
    else:
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(output)
        manifest_path.write_bytes(encoded)
        print(f"Imported {manifest['triangles']} display triangles")


if __name__ == "__main__":
    main()
