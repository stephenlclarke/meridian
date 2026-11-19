"""Check the same-pose Metal SH008 evidence without opening the game."""
import argparse
import csv
import hashlib
import json
import math
import statistics
from pathlib import Path

POSES = ("front", "oblique", "upper")
VARIANTS = ("solid", "normal", "repeat", "phase")
SAMPLES = 16


def ppm(path):
    raw = path.read_bytes()
    magic, dimensions, maximum, pixels = raw.split(b"\n", 3)
    width, height = map(int, dimensions.split())
    if magic != b"P6" or maximum != b"255" or min(width, height) <= 0 or len(pixels) != width * height * 3:
        raise ValueError(f"Invalid PPM sample: {path.name}")
    return (width, height), pixels, hashlib.sha256(raw).hexdigest()


def compare(before, after):
    changed = peak = 0
    delta = bytearray()
    for start in range(0, len(before), 3):
        channels = [abs(a - b) for a, b in zip(
            before[start:start + 3], after[start:start + 3], strict=True
        )]
        maximum = max(channels)
        changed += maximum > 0
        peak = max(peak, maximum)
        delta.extend(min(255, value * 16) for value in channels)
    return {"changed_pixels": changed, "peak_channel_delta": peak}, bytes(delta)


def timing(path):
    with path.open(newline="") as source:
        header = source.readline().rstrip()
        if header == "unsupported: adapter has no TIMESTAMP_QUERY":
            if source.read():
                raise ValueError("Unsupported timing file contains samples")
            return {"status": "unsupported"}
        if header != "sample\tstart_tick\tend_tick\tns_per_tick":
            raise ValueError("Invalid GPU timestamp header")
        durations = []
        period_ns = None
        for index, (sample, start, end, period) in enumerate(csv.reader(source, delimiter="\t")):
            current_period = float(period)
            if period_ns is None:
                period_ns = current_period
            if (
                int(sample) != index or int(start) < 0 or int(end) <= int(start)
                or not math.isfinite(current_period) or current_period <= 0
                or current_period != period_ns
            ):
                raise ValueError("Invalid GPU timestamp sample")
            durations.append((int(end) - int(start)) * current_period)
    if len(durations) != SAMPLES:
        raise ValueError("Unexpected GPU timestamp sample count")
    ordered = sorted(durations)
    return {
        "status": "measured",
        "scope": "one shimmer draw and its isolated render-pass attachment load/store",
        "samples": len(ordered),
        "min_ns": ordered[0],
        "median_ns": statistics.median(ordered),
        "p95_ns": ordered[math.ceil(0.95 * len(ordered)) - 1],
        "max_ns": ordered[-1],
    }


def verify(folder, write_diffs=False):
    poses = {}
    for pose in POSES:
        frames = {variant: ppm(folder / f"{pose}-{variant}.ppm") for variant in VARIANTS}
        dimensions, solid, _ = frames["solid"]
        if any(frame[0] != dimensions for frame in frames.values()) or not any(solid):
            raise ValueError(f"{pose}: missing solid hull or inconsistent crop")
        if frames["normal"][1] != frames["repeat"][1]:
            raise ValueError(f"{pose}: repeated production frame changed")
        normal = frames["normal"][1]
        shimmer, effect_pixels = compare(solid, normal)
        phase, phase_pixels = compare(normal, frames["phase"][1])
        # A 1.8% linear blend of the existing blue-green tint over black is at
        # most 22 sRGB codes. Allow two codes for quantization and conversion.
        if shimmer["peak_channel_delta"] > 24:
            raise ValueError(f"{pose}: pixel difference exceeds the subtle-shimmer bound")
        if write_diffs:
            header = f"P6\n{dimensions[0]} {dimensions[1]}\n255\n".encode("ascii")
            for name, pixels in (("shimmer", effect_pixels), ("phase", phase_pixels)):
                with (folder / f"{pose}-{name}-delta-16x.ppm").open("xb") as image:
                    image.write(header + pixels)
        poses[pose] = {
            "crop_pixels": dimensions,
            "sha256": {name: frame[2] for name, frame in frames.items()},
            "shimmer": shimmer,
            "phase": phase,
            "gpu_timing": timing(folder / f"{pose}-gpu.tsv"),
        }
    if any(not any(pose[effect]["changed_pixels"] for pose in poses.values())
           for effect in ("shimmer", "phase")):
        raise ValueError("No nonzero same-pose shimmer or phase difference resolved")
    if len({pose["gpu_timing"]["status"] for pose in poses.values()}) != 1:
        raise ValueError("Inconsistent timestamp capability for one adapter")
    return poses


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("--write-diffs", action="store_true")
    args = parser.parse_args()
    print(json.dumps(verify(args.evidence, args.write_diffs), indent=2))
