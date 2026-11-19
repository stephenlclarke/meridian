"""Encode the authenticated Goshawk Mk II detailed display in Showroom format."""
import argparse
from collections import Counter
import json
from pathlib import Path
import tomllib

from encode_goshawk_visual import ROOT, TEXTURES, VisualSource, encode_for_hull
from prepare_goshawk_mk2_geometry import ASSETS, LEGACY_REVISION, pin, prepare


SH009 = VisualSource(
    "SH009", "4fc835b6f40c1b7751ff8c03940c0501ce7c445b1bd0be95f15651a174c67552",
    2104, 3532, 568, b"MSH009V1", b"GOS2VIZ1",
)
STAGED_BASE = "06c2649da4bac073445f3e8e116c5b9a4850e1d5"
TEXTURE_DIRECTORY = ROOT / "assets/fleet/sh001"
TEXTURE_MANIFEST_SHA256 = "96c4c9699b81f50be5a8aefeeca74a6e4870e0bb981d1cd33a9ff2921cd2bd3c"
GEOMETRY_PINS = {
    "source.vertices": "022b4fe3b25d84085411d32ef4de8e19f17ff6f83ff178dc7bc4164edb95b2d7",
    "source-triangles.bin": "944398f70608e4def2783ceb0ff591de602a7799d36f7f88e63124ad8cc57d73",
}


def encode(source, points, triangles):
    visual = encode_for_hull(source, points, triangles, SH009)
    for name, data in zip(GEOMETRY_PINS, (points, triangles), strict=True):
        if pin(data)["sha256"] != GEOMETRY_PINS[name]:
            raise ValueError(f"Changed complete SH009 geometry: {name}")
    return visual


def texture_provenance(folder):
    manifest = (folder / "TEXTURE-SOURCE.json").read_bytes()
    if pin(manifest)["sha256"] != TEXTURE_MANIFEST_SHA256:
        raise ValueError("Changed original shared texture manifest")
    rows = {row["id"]: row for row in json.loads(manifest)["assets"]}
    result = {}
    for name in TEXTURES:
        row = rows[name]
        filename = row["rgbaFile"]
        data = (folder / filename).read_bytes()
        expected = {"bytes": row["width"] * row["height"] * 4, "sha256": row["rgbaSHA256"]}
        if pin(data) != expected or row["license"] != "MIT":
            raise ValueError(f"Changed original shared texture: {filename}")
        result[f"assets/fleet/sh001/{filename}"] = {
            **expected, "source": f"Sources/L337Core/Resources/OriginalShipTextures/{filename}",
            "license": row["license"], "width": row["width"], "height": row["height"],
            "authorship": row["authorship"],
        }
    return result


def fnv64(data):
    value = 0xcbf29ce484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    return f"{value:016x}"


def prepare_visual(folder, texture_folder=TEXTURE_DIRECTORY):
    geometry = prepare(folder)
    for name, data in geometry.items():
        if (folder / name).read_bytes() != data:
            raise ValueError(f"Changed staged SH009 geometry: {name}")
    source = (folder / "resource.toml").read_bytes()
    visual = encode(source, geometry["source.vertices"], geometry["source-triangles.bin"])
    model = tomllib.loads(source.decode())["design"]
    texture_counts = Counter(mapping["texture"] for mapping in model["faceTextures"])
    provenance = {
        "id": SH009.hull,
        "sourceRevision": LEGACY_REVISION,
        "stagedSourceCommit": STAGED_BASE,
        "resource": {**pin(source), "source": "Sources/L337Core/Resources/Ships/SH009.toml"},
        "geometry": {name: pin(geometry[name]) for name in GEOMETRY_PINS},
        "geometryReceipt": pin(geometry["SOURCE-GEOMETRY.json"]),
        "visual": {
            **pin(visual), "path": "assets/fleet/sh009/visual.bin", "fnv1a64": fnv64(visual),
            "magic": SH009.visual_magic.decode(), "strideBytes": 56,
            "faces": SH009.detailed_faces, "vertices": SH009.detailed_faces * 3,
            "textureFaceCounts": {name: texture_counts[index] for index, name in enumerate(TEXTURES)},
            "emissiveFaces": sum(model["materials"][index]["emissive"] for index in model["faceMaterials"]),
            "encoder": "tools/encode_goshawk_mk2_visual.py",
            "format": "SH008 Showroom record layout; SH009-specific header. Calibrated detailed positions, ordered flat normals, per-face RGBA, original corner UV, texture index and emissive bit.",
        },
        "sharedOriginalTextureManifestSHA256": TEXTURE_MANIFEST_SHA256,
        "sharedOriginalTextures": texture_provenance(texture_folder),
        "license": "MIT; see assets/fleet/sh009/LICENSE and the original shared texture manifest.",
        "presentation": "Opaque detailed Showroom asset only. Collision envelope, shimmer, registration and ordinary gameplay placement require separate integration and qualification.",
    }
    return {"visual.bin": visual, "VISUAL-PROVENANCE.json": (json.dumps(provenance, indent=2) + "\n").encode()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, default=ASSETS)
    parser.add_argument("--textures", type=Path, default=TEXTURE_DIRECTORY)
    parser.add_argument("--check", action="store_true")
    options = parser.parse_args()
    for name, data in prepare_visual(options.assets, options.textures).items():
        path = options.assets / name
        if options.check:
            if path.read_bytes() != data:
                raise ValueError(f"Changed SH009 visual output: {name}")
        else:
            path.write_bytes(data)
    print(f"SH009 checked: {SH009.detailed_faces} detailed faces, {SH009.detailed_faces * 3} ordered visual records.")


if __name__ == "__main__":
    main()
