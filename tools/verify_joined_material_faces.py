#!/usr/bin/env python3
"""Independently bind SH012/SH019 raw material faces and optional Metal pixel grids to original TOML."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import struct
import tomllib

ROOT = Path(__file__).resolve().parents[1]
HULLS = {
    "SH012": (b"SBLRVIZ1", 0xffffffff, 1240),
    "SH019": (b"SH019VZ1", 3, 1216),
}
WIDTH, HEIGHT, COLUMNS, TILE = 320, 256, 40, 8


def require(value, reason):
    if not value:
        raise ValueError(reason)


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def source_faces(hull):
    magic, marker, expected_count = HULLS[hull]
    folder = ROOT / "assets/fleet" / hull.lower()
    resource = (folder / "resource.toml").read_bytes()
    original = tomllib.loads(resource.decode())["design"]
    packed = (folder / "visual.bin").read_bytes()
    require(struct.unpack_from("<8sII", packed) == (magic, len(original["faces"]) * 3, 56), f"{hull}: wrong visual header")
    records = list(struct.iter_unpack("<12fII", packed[16:]))
    require(len(records) == len(original["faces"]) * 3, f"{hull}: wrong visual length")
    result = []
    for index, (mapping, material_index) in enumerate(zip(original["faceTextures"], original["faceMaterials"], strict=True)):
        material = original["materials"][material_index]
        rgba = material["colour"].to_bytes(4, "big")
        texture = marker if mapping["texture"] == -1 else mapping["texture"]
        for corner, record in enumerate(records[index * 3:index * 3 + 3]):
            require(record[12] == texture, f"{hull} face {index}: source texture mismatch")
            require(record[6:10] == tuple(f32(channel / 255) for channel in rgba), f"{hull} face {index}: packed raw colour mismatch")
            require(record[10:12] == tuple(f32(value) for value in mapping["uv"][corner]), f"{hull} face {index}: source UV mismatch")
            require(record[13] == int(material["emissive"]), f"{hull} face {index}: source emissive mismatch")
        if mapping["texture"] == -1:
            result.append((index, tuple(rgba[:3])))
    require(len(result) == expected_count, f"{hull}: wrong source no-map face count")
    return result, hashlib.sha256(resource).hexdigest(), hashlib.sha256(packed).hexdigest()


def verify_grid(evidence, hull, faces):
    ppm = (evidence / f"{hull}-material-faces.ppm").read_bytes()
    magic, dimensions, depth, pixels = ppm.split(b"\n", 3)
    require((magic, dimensions, depth) == (b"P6", b"320 256", b"255"), f"{hull}: wrong PPM header")
    require(len(pixels) == WIDTH * HEIGHT * 3, f"{hull}: wrong PPM byte count")
    with (evidence / f"{hull}-material-faces.tsv").open(newline="") as stream:
        receipt = list(csv.DictReader(stream, delimiter="\t"))
    require(len(receipt) == len(faces), f"{hull}: incomplete material receipt")
    for slot, ((source_face, expected), row) in enumerate(zip(faces, receipt, strict=True)):
        x, y = slot % COLUMNS * TILE + TILE // 2, slot // COLUMNS * TILE + TILE // 2
        offset = (y * WIDTH + x) * 3
        raw = ",".join(str(channel) for channel in expected)
        require(row == {"source_face": str(source_face), "x": str(x), "y": str(y), "raw_expected_rgb": raw, "actual_rgb": raw}, f"{hull} face {source_face}: receipt mismatch")
        require(tuple(pixels[offset:offset + 3]) == expected, f"{hull} face {source_face}: raw GPU colour mismatch")
    return len(faces)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, help="Actual combined raw Metal grid directory; omit for source-only verification")
    args = parser.parse_args()
    report = {}
    for hull in HULLS:
        faces, source_sha, visual_sha = source_faces(hull)
        report[hull] = {"sourceSha256": source_sha, "visualSha256": visual_sha, "sourceFaces": len(faces)}
        if args.evidence:
            report[hull]["verifiedRawGpuFaces"] = verify_grid(args.evidence, hull, faces)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
