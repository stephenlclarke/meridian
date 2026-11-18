"""Require native-only hits in the production viewport, scale and queue adapters."""

import json
from pathlib import Path
from coverage_workspace import workspace_path


SOURCE = Path("crates/meridian-desktop/src")
REQUIRED = {
    SOURCE / "app.rs": {"r.map(Renderer::logical_viewport)": 1},
    SOURCE / "renderer.rs": {
        "let scale = self.window.scale_factor();": 2,
        "|bytes| self.queue.write_buffer(&self.hud_vertices, 0, bytes),": 1,
    },
}


def read_hits(lines, paths):
    known = {path.as_posix(): path for path in paths}
    hits = {path: {} for path in paths}
    current = None
    for line in lines:
        if line.startswith("SF:"):
            name = line.removeprefix("SF:").replace("\\", "/")
            current = next((path for suffix, path in known.items()
                            if name == suffix or name.endswith("/" + suffix)), None)
        elif line.startswith("DA:") and current is not None:
            number, count, *_ = line.removeprefix("DA:").split(",")
            number, count = int(number), int(count)
            hits[current][number] = max(count, hits[current].get(number, 0))
        elif line == "end_of_record":
            current = None
    return hits


def main():
    lcov = workspace_path("coverage/native/desktop.lcov")
    hits = read_hits(lcov.read_text().splitlines(), REQUIRED)
    measurements = {}
    for path, markers in REQUIRED.items():
        lines = workspace_path(path).read_text().splitlines()
        rows = {}
        for marker, expected_count in markers.items():
            matches = [n for n, source in enumerate(lines, 1) if source.strip() == marker]
            if len(matches) != expected_count or any(hits[path].get(n, 0) <= 0 for n in matches):
                raise SystemExit(f"Native coverage is missing for {path}: {marker}")
            rows[marker] = [{"line": n, "hits": hits[path][n]} for n in matches]
        measurements[str(path)] = rows
    print(json.dumps(measurements, indent=2))

MULE_GPU = SOURCE / "mule_render/gpu.rs"
MULE_GPU_MARKERS = (
    "queue.write_texture(",
    "let solid = pipeline(false);",
    "let shimmer = pipeline(true);",
    "queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&data));",
    "pass.set_pipeline(&self.solid);",
    "pass.draw(0..VERTEX_COUNT as u32, 0..1);",
    "pass.set_pipeline(&self.shimmer);",
    "pass.draw(0..3, 0..1);",
)
# The failed main analysis found 267 uncovered lines in this byte-exact source file.
MULE_BASELINE_UNCOVERED = 267


def check_mule_coverage():
    lcov = workspace_path("coverage/native/mule-desktop.lcov")
    hits = read_hits(lcov.read_text().splitlines(), {MULE_GPU})[MULE_GPU]
    source = workspace_path(MULE_GPU).read_text().splitlines()
    markers = {}
    for marker in MULE_GPU_MARKERS:
        matches = [n for n, line in enumerate(source, 1) if line.strip() == marker]
        if len(matches) != 1 or hits.get(matches[0], 0) <= 0:
            raise SystemExit(f"Mule native coverage is missing for {MULE_GPU}: {marker}")
        markers[marker] = {"line": matches[0], "hits": hits[matches[0]]}
    covered = sum(count > 0 for count in hits.values())
    print(json.dumps({
        "source": str(MULE_GPU), "covered": covered, "executable": len(hits),
        "original_sonar_uncovered": MULE_BASELINE_UNCOVERED, "markers": markers,
        "uncovered_lines": sorted(line for line, count in hits.items() if count == 0),
    }, indent=2))
    if covered * 5 < max(len(hits), MULE_BASELINE_UNCOVERED) * 4:
        raise SystemExit(f"Mule native coverage is below 80 percent: {covered}/{len(hits)}")


if __name__ == "__main__":
    main()
    check_mule_coverage()
