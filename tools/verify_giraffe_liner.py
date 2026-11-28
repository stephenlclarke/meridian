"""Replay exact source overlaps, structural separations and guarded Giraffe Liner routes."""

from collections import Counter
from fractions import Fraction
import json
from math import isqrt
import struct
import sys

from import_goshawk_envelope_source import components
from prepare_giraffe_liner import INTEGRATION, ORIGINAL, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def original_parts(model):
    points = [tuple(map(Fraction, point)) for point in model["vertices"]]
    result = []
    for indices in components(model):
        vertices = set(indices)
        faces = [face for face in model["faces"] if face[0] in vertices]
        edges = Counter((a, b) for x, y, z in faces for a, b in ((x, y), (y, z), (z, x)))
        require(len(indices) - len(edges) // 2 + len(faces) == 2
                and all(count == edges[(b, a)] == 1 for (a, b), count in edges.items()),
                "Giraffe Liner source component has changed topology")
        result.append(([points[i] for i in indices], [[points[i] for i in face] for face in faces]))
    return result


def separated(first, second, axis):
    def project(point):
        return sum(coordinate * factor for coordinate, factor in zip(point, axis, strict=True))
    distance = min(map(project, second)) - max(map(project, first))
    require(distance > 0, "Giraffe Liner projection witness has no strict separation")
    return isqrt((distance * 1000)**2 // sum(factor * factor for factor in axis))


def native_cap_gaps(parts):
    hull = parts[0][0]
    plane = Fraction(-28)
    require(min(p[2] for p in hull) == plane, "wrong Giraffe Liner pressure hull cap")
    hull_cap = [p for p in hull if p[2] == plane]
    result = {}
    for side, index, sign in (("port", 4, -1), ("starboard", 8, 1)):
        drive = parts[index][0]
        require(max(p[2] for p in drive) == plane, "wrong Giraffe Liner drive cap")
        drive_cap = [p for p in drive if p[2] == plane]
        distance = separated(hull_cap, drive_cap, (sign, 0, 0))
        require(distance == 551, "changed Giraffe Liner native drive cap gap")
        result[side] = distance
    return result


def verify_geometry(detail, native):
    witnesses = json.loads((INTEGRATION / "ORIGINAL-OVERLAPS.json").read_bytes())
    separations = json.loads((INTEGRATION / "ORIGINAL-SEPARATIONS.json").read_bytes())
    overlaps, gaps = {}, {}
    native_parts = original_parts(native)
    for lod, model, parts, expected_components in (
        ("design", detail, original_parts(detail), 30),
        ("nativeDesign", native, native_parts, 11),
    ):
        source = witnesses[lod]
        require(len(parts) == source["componentCount"] == expected_components
                and len(model["faces"]) == source["surfaceTriangles"], "changed Giraffe Liner source inventory")
        joined = []
        for row in source["interiorOverlaps"]:
            a, b = row["parts"]
            point = tuple(map(Fraction, row["pointMetres"]))
            hits = [list(inside_y(point, parts[index][1])) for index in (a, b)]
            require(hits == row["hitCountsAboveBelow"] and all(n % 2 for hit in hits for n in hit),
                    "changed Giraffe Liner overlap proof")
            joined.append((a, b))
        numerics = {}
        for row in separations[lod]:
            a, b = row["parts"]
            actual = separated(parts[a][0], parts[b][0], row["axis"])
            require(actual == row["separationMillimetres"], "changed Giraffe Liner structural gap")
            numerics[f"{a}-{b}"] = actual
        require(len(set(joined)) == len(joined), "duplicate Giraffe Liner overlap witness")
        overlaps[lod], gaps[lod] = joined, numerics
    require(len(overlaps["design"]) == 36 and {(0, 2), (0, 8), (1, 3), (7, 9), (13, 14)} <= set(overlaps["design"]),
            "missing Giraffe Liner detailed structural overlap")
    require(set(overlaps["nativeDesign"]) == {(1, 3), (5, 7), (9, 10)},
            "changed Giraffe Liner archived native overlaps")
    require(gaps["nativeDesign"]["0-2"] == gaps["nativeDesign"]["0-6"] == 878
            and gaps["design"]["1-2"] == gaps["design"]["7-8"] == 450
            and gaps["nativeDesign"]["1-2"] == gaps["nativeDesign"]["5-6"] == 450
            and gaps["design"]["1-7"] == gaps["nativeDesign"]["1-5"] == 5400,
            "changed Giraffe Liner gallery isolation")
    return {
        "sourceOverlapPairs": overlaps,
        "sourceSeparationsMillimetres": gaps,
        "nativeDriveCapLateralGapsMillimetres": native_cap_gaps(native_parts),
    }


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH093 proof input: {name}")
    names = json.loads(derived["PIECES.json"])["pieces"]
    pieces = [[] for _ in names]
    vertices = struct.iter_unpack("<3d", derived["source.vertices"][16:])
    for slot, vertex in zip(derived["source-piece-membership.bin"], vertices, strict=True):
        pieces[slot].append(tuple(map(Fraction, vertex)))

    routes = json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes())
    clearances = {}
    for route in routes:
        triangle = [tuple(map(Fraction, p)) for p in route["triangle"]]
        travel = tuple(map(Fraction, route["travel"]))
        sweep = triangle + [tuple(a - b for a, b in zip(p, travel, strict=True)) for p in triangle]
        distance = separating_planes(pieces, sweep, route["axes"])
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Giraffe Liner sweep")
        clearances[route["name"]] = distance
    require(len(routes) == 7 and set(clearances) == {
        "dorsal", "ventral", "between-aft-drives", "between-observation-galleries",
        "port-outboard", "starboard-outboard", "forward-above-bow",
    }, "wrong Giraffe Liner routes")
    _, _, _, detail, native = load_source()
    return {"clearanceMillimetres": clearances, **verify_geometry(detail, native)}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
