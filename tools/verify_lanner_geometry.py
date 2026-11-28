"""Replay the guarded SH048 shape routes in both archival and candidate envelopes."""

from fractions import Fraction
import json
import sys
import tomllib

from lanner_art_mesh import assemble
from prepare_lanner_geometry import INTEGRATION, ORIGINAL, SCALE, build, encode
from verify_eagle_cruiser_geometry import separating_planes


def verify_route_set(record, native, detailed):
    slots, meshes = assemble(detailed, native)
    if record["coordinateScale"] != [SCALE.numerator, SCALE.denominator]:
        raise ValueError("Changed frozen Lanner units")
    pieces = [[] for _ in slots]
    for mesh in meshes:
        for group, point in zip(mesh["groups"], mesh["vertices"], strict=True):
            pieces[group].append(tuple(Fraction(value * float(SCALE)) for value in point))
    observed = {}
    for route in record["routes"]:
        triangle = [tuple(map(Fraction, p)) for p in route["triangle"]]
        travel = tuple(map(Fraction, route["travel"]))
        sweep = triangle + [tuple(a - b for a, b in zip(p, travel, strict=True)) for p in triangle]
        clearance = separating_planes(pieces, sweep, route["axes"])
        if clearance != route["clearanceMillimetres"] or clearance <= 100:
            raise ValueError("Changed Lanner shape route")
        observed[route["name"]] = clearance
    if len(observed) != len(record["routes"]) or len(observed) != 7:
        raise ValueError("Unexpected Lanner route count")
    return observed


def verify():
    generated = build()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        if (ORIGINAL / name).read_bytes() != generated[name]:
            raise ValueError(f"Changed Lanner route input {name}")
    source = tomllib.loads((ORIGINAL / "resource.toml").read_text())
    owners = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
    candidate = json.loads((ORIGINAL / "SH048-candidate.json").read_bytes())
    detailed = source["design"], owners["design"]["parts"], 12
    archive = source["nativeDesign"], owners["nativeDesign"]["parts"], 8
    reduced = candidate["nativeCandidate"], candidate["parts"], 4
    archived_routes = json.loads((INTEGRATION / "ARCHIVE-SUPPORT-PREFLIGHT.json").read_bytes())
    candidate_routes = json.loads((INTEGRATION / "candidate-support.json").read_bytes())
    return {
        "detailedWithArchivedNative": verify_route_set(archived_routes, archive, detailed),
        "detailedWithCandidateNative": verify_route_set(candidate_routes, reduced, detailed),
    }


if __name__ == "__main__":
    sys.stdout.buffer.write(encode(verify()))
