#!/usr/bin/env python3
"""Verify independent PR76 styled face expectations, not the raw PR87 evidence."""
import argparse
import csv
import json
from pathlib import Path

from verify_joined_material_faces import COLUMNS, HEIGHT, HULLS, TILE, WIDTH, require, source_faces

# Independently evaluated PR76 pale-floor rule for the two original colours.
# Do not import or run the paint implementation when calculating expected values.
STYLED = {(116, 191, 204): (100, 175, 188), (185, 235, 217): (100, 150, 132)}


def rgb_text(rgb):
    return ",".join(str(channel) for channel in rgb)


def verify_styled_grid(evidence, hull, faces):
    stem = f"{hull}-styled-material-faces"
    ppm = (evidence / f"{stem}.ppm").read_bytes()
    magic, dimensions, depth, pixels = ppm.split(b"\n", 3)
    require((magic, dimensions, depth) == (b"P6", b"320 256", b"255"), f"{hull}: wrong styled PPM header")
    require(len(pixels) == WIDTH * HEIGHT * 3, f"{hull}: wrong styled PPM byte count")
    with (evidence / f"{stem}.tsv").open(newline="") as stream:
        receipt = list(csv.DictReader(stream, delimiter="\t"))
    require(len(receipt) == len(faces), f"{hull}: incomplete styled face receipt")
    for slot, ((source_face, raw), row) in enumerate(zip(faces, receipt, strict=True)):
        require(raw in STYLED, f"{hull} face {source_face}: unknown original colour")
        styled = STYLED[raw]
        x, y = slot % COLUMNS * TILE + TILE // 2, slot // COLUMNS * TILE + TILE // 2
        offset = (y * WIDTH + x) * 3
        require(row == {"source_face": str(source_face), "x": str(x), "y": str(y), "source_raw_rgb": rgb_text(raw), "styled_expected_rgb": rgb_text(styled), "actual_rgb": rgb_text(styled)}, f"{hull} face {source_face}: styled receipt mismatch")
        require(tuple(pixels[offset:offset + 3]) == styled, f"{hull} face {source_face}: styled GPU colour mismatch")
    return len(faces)


def verify_visible_hull(evidence, hull):
    with (evidence / f"{hull}-styled-visible.tsv").open(newline="") as stream:
        receipt = list(csv.DictReader(stream, delimiter="\t"))
    poses = ["front", "oblique", "upper", "aft"]
    require(len(receipt) == len(poses), f"{hull}: missing styled whole-hull poses")
    totals = [0, 0]
    for pose, row in zip(poses, receipt, strict=True):
        magic, dimensions, depth, pixels = (evidence / f"{hull}-{pose}-styled-solid.ppm").read_bytes().split(b"\n", 3)
        width, height = map(int, dimensions.split())
        require(magic == b"P6" and depth == b"255" and width > 0 and height > 0 and len(pixels) == width * height * 3, f"{hull}: invalid whole-hull PPM")
        counts = [sum(pixels[offset:offset + 3] == bytes(styled) for offset in range(0, len(pixels), 3)) for styled in STYLED.values()]
        require(row == {"pose": pose, "styled_glazing_pixels": str(counts[0]), "styled_emitter_pixels": str(counts[1])}, f"{hull} {pose}: styled whole-hull receipt mismatch")
        totals = [a + b for a, b in zip(totals, counts, strict=True)]
    require(all(totals), f"{hull}: styled material not visible in actual hull poses")
    return totals


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, help="Actual combined production styled Metal grid; omit for source-only expectations")
    parser.add_argument("--cache-evidence", type=Path, help="Styled cache directory including actual whole-hull material visibility captures")
    args = parser.parse_args()
    report = {}
    for hull in HULLS:
        faces, source_sha, visual_sha = source_faces(hull)
        require(all(raw in STYLED for _, raw in faces), f"{hull}: original material policy changed")
        report[hull] = {"sourceSha256": source_sha, "visualSha256": visual_sha, "styledFaceCount": len(faces), "colours": [{"raw": raw, "styled": STYLED[raw], "faces": sum(colour == raw for _, colour in faces)} for raw in STYLED]}
        if args.evidence:
            report[hull]["verifiedStyledGpuFaces"] = verify_styled_grid(args.evidence, hull, faces)
        if args.cache_evidence:
            report[hull]["visibleStyledPixels"] = verify_visible_hull(args.cache_evidence, hull)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
