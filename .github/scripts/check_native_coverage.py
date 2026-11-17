"""Require native-only hits in the production viewport, scale and queue adapters."""

import json
from pathlib import Path
import sys


SOURCE = Path("crates/meridian-desktop/src")
REQUIRED = {
    SOURCE / "app.rs": ("r.map(Renderer::logical_viewport)",),
    SOURCE / "renderer.rs": (
        "|| window.scale_factor(),",
        "|bytes| queue.write_buffer(vertices, 0, bytes),",
    ),
}


def read_hits(lcov, paths):
    known = {str(path): path for path in paths}
    hits = {path: {} for path in paths}
    current = None
    for line in lcov.read_text().splitlines():
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


def main(lcov):
    hits = read_hits(lcov, REQUIRED)
    measurements = {}
    for path, markers in REQUIRED.items():
        lines = path.read_text().splitlines()
        rows = {}
        for marker in markers:
            matches = [n for n, source in enumerate(lines, 1) if source.strip() == marker]
            if len(matches) != 1 or hits[path].get(matches[0], 0) <= 0:
                raise SystemExit(f"Native coverage is missing for {path}: {marker}")
            rows[marker] = {"line": matches[0], "hits": hits[path][matches[0]]}
        measurements[str(path)] = rows
    print(json.dumps(measurements, indent=2))


if __name__ == "__main__":
    main(Path(sys.argv[1]))
