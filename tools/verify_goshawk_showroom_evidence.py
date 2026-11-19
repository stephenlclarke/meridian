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
METAL_COUNTER_ERROR = (1 << 64) - 1


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


def view_directions(path):
    with path.open(newline="") as source:
        rows = csv.reader(source, delimiter="\t")
        if next(rows, None) != ["pose", "x", "y", "z"]:
            raise ValueError("Invalid hull-local view direction header")
        directions = {}
        for expected, (name, x, y, z) in zip(POSES, rows, strict=True):
            direction = tuple(map(float, (x, y, z)))
            if expected != name or not math.isclose(math.hypot(*direction), 1.0, abs_tol=1e-6):
                raise ValueError("Invalid hull-local viewing direction")
            directions[name] = direction
    front, oblique, upper = directions.values()
    if not (
        abs(front[0]) < 0.05 and front[2] > 0.9
        and abs(oblique[0]) > 0.4 and oblique[2] > 0.7
        and upper[1] > 0.6 and upper[2] > 0.4
    ):
        raise ValueError("Missing distinct front, oblique and upper hull-local directions")
    return directions


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
        first_tick = last_tick = None
        period_ns = None
        for index, (sample, start, end, period) in enumerate(csv.reader(source, delimiter="\t")):
            start, end = int(start), int(end)
            current_period = float(period)
            if period_ns is None:
                period_ns = current_period
            if (
                int(sample) != index or not 0 < start < end < METAL_COUNTER_ERROR
                or not math.isfinite(current_period) or current_period <= 0
                or current_period != period_ns
            ):
                raise ValueError("Invalid GPU timestamp sample")
            if last_tick is not None and start < last_tick:
                raise ValueError("GPU intervals overlap or move backwards within a pose")
            if first_tick is None:
                first_tick = start
            last_tick = end
            durations.append((end - start) * current_period)
    if len(durations) != SAMPLES:
        raise ValueError("Unexpected GPU timestamp sample count")
    ordered = sorted(durations)
    return {
        "status": "measured",
        "scope": "one shimmer draw and its isolated render-pass attachment load/store",
        "samples": len(ordered),
        "ns_per_tick": period_ns,
        "first_tick": first_tick,
        "last_tick": last_tick,
        "min_ns": ordered[0],
        "median_ns": statistics.median(ordered),
        "p95_ns": ordered[math.ceil(0.95 * len(ordered)) - 1],
        "max_ns": ordered[-1],
    }


def verify(folder, write_diffs=False, timing_dir=None):
    directions = view_directions(folder / "hull-local-views.tsv")
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
        shifted = frames["phase"][1]
        shifted_shimmer, _ = compare(solid, shifted)
        phase, phase_pixels = compare(normal, shifted)
        # The strongest tint over black gives about 22 sRGB codes at 1.8% alpha.
        for name, effect in (("normal", shimmer), ("shifted", shifted_shimmer)):
            if effect["peak_channel_delta"] > 24:
                raise ValueError(f"{pose}: {name} versus solid exceeds the subtle-shimmer bound")
        # The wave moves alpha by at most 0.018 * 0.10. Across every base
        # channel the distance to the tint is <= 0.88, so the maximum sRGB
        # slope 12.92 gives <= 5.22 codes before 8-bit quantization.
        if phase["peak_channel_delta"] > 7:
            raise ValueError(f"{pose}: shifted versus normal exceeds the phase-modulation bound")
        if write_diffs:
            header = f"P6\n{dimensions[0]} {dimensions[1]}\n255\n".encode("ascii")
            for name, pixels in (("shimmer", effect_pixels), ("phase", phase_pixels)):
                with (folder / f"{pose}-{name}-delta-16x.ppm").open("xb") as image:
                    image.write(header + pixels)
        poses[pose] = {
            "crop_pixels": dimensions,
            "hull_local_viewer_direction": directions[pose],
            "sha256": {name: frame[2] for name, frame in frames.items()},
            "shimmer": shimmer,
            "shifted_shimmer": shifted_shimmer,
            "phase": phase,
        }
    if any(not any(pose[effect]["changed_pixels"] for pose in poses.values())
           for effect in ("shimmer", "phase")):
        raise ValueError("No nonzero same-pose shimmer or phase difference resolved")
    if timing_dir is None:
        return poses
    reports = []
    for name, pose in poses.items():
        report = timing(timing_dir / f"{name}-gpu.tsv")
        pose["gpu_timing"] = report
        reports.append(report)
    if len({report["status"] for report in reports}) != 1:
        raise ValueError("Inconsistent timestamp capability for one adapter")
    if reports[0]["status"] == "measured":
        if len({report["ns_per_tick"] for report in reports}) != 1:
            raise ValueError("GPU timestamp period changed across poses on one queue")
        if any(current["first_tick"] < previous["last_tick"]
               for previous, current in zip(reports, reports[1:])):
            raise ValueError("GPU intervals overlap or move backwards across poses")
    return poses


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("--write-diffs", action="store_true")
    parser.add_argument("--timing-dir", type=Path, help="Also verify the separate GPU timestamp run")
    args = parser.parse_args()
    print(json.dumps(verify(args.evidence, args.write_diffs, args.timing_dir), indent=2))
