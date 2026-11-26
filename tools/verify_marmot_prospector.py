"""Replay exact source overlaps, structural separations and guarded Marmot Prospector routes."""

from collections import Counter
from fractions import Fraction
import json
from math import isqrt
import struct
import sys

from import_goshawk_envelope_source import components
from prepare_marmot_prospector import ABSENT, COLLARS, INTEGRATION, NATIVE_COMPONENTS, ORIGINAL, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes

try:
    from hull_contact_geometry import projected_weights
except ModuleNotFoundError as error:
    if error.name != "hull_contact_geometry":
        raise
    from verify_eagle_cruiser_geometry import projected_weights


def original_parts(model):
    points = [tuple(map(Fraction, point)) for point in model["vertices"]]
    result = []
    for order, indices in enumerate(components(model)):
        vertices = set(indices)
        faces = [face for face in model["faces"] if face[0] in vertices]
        edges = Counter((a, b) for x, y, z in faces for a, b in ((x, y), (y, z), (z, x)))
        require(len(indices) - len(edges) // 2 + len(faces) == (0 if order in COLLARS else 2)
                and all(count == edges[(b, a)] == 1 for (a, b), count in edges.items()),
                "Marmot Prospector source component has changed topology")
        result.append(([points[i] for i in indices], [[points[i] for i in face] for face in faces]))
    return result


def verify_geometry(detail, native):
    witnesses = json.loads((INTEGRATION / "ORIGINAL-OVERLAPS.json").read_bytes())
    overlaps = {}
    for lod, model, expected_components in (("design", detail, 35), ("nativeDesign", native, 16)):
        parts = original_parts(model)
        record = witnesses[lod]
        require(len(parts) == record["componentCount"] == expected_components
                and len(model["faces"]) == record["surfaceTriangles"], "changed Marmot Prospector source inventory")
        joined = set()
        for row in record["interiorOverlaps"]:
            pair = tuple(row["parts"])
            point = tuple(map(Fraction, row["pointMetres"]))
            counts = [list(inside_y(point, parts[index][1])) for index in pair]
            require(counts == row["hitCountsAboveBelow"] and all(n % 2 for hit in counts for n in hit),
                    "changed Marmot Prospector exact overlap")
            require(pair not in joined, "duplicate Marmot Prospector overlap")
            joined.add(pair)
        overlaps[lod] = joined
    expected = {'design': {(0, 1),
                (0, 2),
                (0, 5),
                (0, 8),
                (0, 11),
                (0, 15),
                (1, 19),
                (2, 3),
                (3, 4),
                (5, 6),
                (6, 7),
                (8, 9),
                (9, 10),
                (12, 13),
                (13, 14),
                (16, 17),
                (17, 18),
                (19, 20),
                (21, 22),
                (21, 23),
                (21, 24),
                (21, 25),
                (21, 26),
                (21, 27),
                (28, 29),
                (28, 30),
                (28, 31),
                (28, 32),
                (28, 33),
                (28, 34)},
     'nativeDesign': {(0, 1),
                      (0, 2),
                      (0, 5),
                      (0, 8),
                      (0, 11),
                      (0, 13),
                      (1, 15),
                      (2, 3),
                      (5, 6),
                      (8, 9)}}
    require(overlaps == expected, "changed Marmot Prospector measured overlap sets")
    shared = {tuple(NATIVE_COMPONENTS[i] for i in pair) for pair in overlaps["nativeDesign"]}
    require(shared <= overlaps["design"], "changed Marmot Prospector shared overlap set")
    return {"sourceOverlapPairs": {lod:sorted(pairs) for lod,pairs in overlaps.items()},
            "sharedOverlapCount": len(shared), "detailedOnlyOverlapCount": len(overlaps["design"] - shared)}


def verify_collar_bores(detail, native, supports):
    axes = ((2,1),(1,2),(-1,2),(-2,1),(-2,-1),(-1,-2),(1,-2),(2,-1))
    centers = (-8, 0, 8)
    for source in (detail, native):
        parts = original_parts(source)
        for c, index in zip(centers, COLLARS, strict=True):
            x, y, z = Fraction(c) + Fraction(13,1000), Fraction(-2), Fraction(17011,1000)
            hits = []
            for triangle in parts[index][1]:
                weights = projected_weights([(a, c) for a, _, c in triangle], x, z)
                if weights is None or min(weights) < 0:
                    continue
                height = sum(weight * vertex[1] for weight, vertex in zip(weights, triangle, strict=True))
                require(min(weights) > 0 and height != y, "Marmot collar parity meets an edge or face")
                hits.append(height > y)
            witnesses = (sum(hits), len(hits) - sum(hits))
            require(all(hit >= 2 and hit % 2 == 0 for hit in witnesses),
                    "original Marmot Prospector collar bore parity changed")
    floors = {}
    for order, center in enumerate(centers):
        guards = []
        for step, (x, y) in enumerate(axes):
            points = supports[order*len(axes)+step]
            distance = min(x*(point[0]-center) + y*(point[1]+2) for point in points)
            require(distance > 0, "Marmot Prospector local collar support enters its own axis")
            guards.append(isqrt((distance * 1000)**2 // (x*x+y*y)) - 21)
        floors[("port","central","starboard")[order]] = min(guards)
    return floors


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH130 proof input: {name}")
    names = json.loads(derived["PIECES.json"])["pieces"]
    pieces = [[] for _ in names]
    vertices = struct.iter_unpack("<3d", derived["source.vertices"][16:])
    membership = struct.iter_unpack("<BB", derived["source-piece-membership.bin"])
    for slots, vertex in zip(membership, vertices, strict=True):
        point = tuple(map(Fraction, vertex))
        for slot in slots:
            if slot != ABSENT:
                pieces[slot].append(point)

    routes = json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes())
    clearances = {}
    for route in routes:
        triangle = [tuple(map(Fraction, p)) for p in route["triangle"]]
        travel = tuple(map(Fraction, route["travel"]))
        sweep = triangle + [tuple(a - b for a, b in zip(p, travel, strict=True)) for p in triangle]
        distance = separating_planes(pieces, sweep, route["axes"])
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Marmot Prospector sweep")
        clearances[route["name"]] = distance
    require(len(routes) == 7 and set(clearances) == {
        "dorsal", "ventral", "between-aft-drives", "port-outboard", "starboard-outboard",
        "between-port-and-central-collars", "between-central-and-starboard-collars",
    }, "wrong Marmot Prospector route inventory")
    _, _, _, detail, native = load_source()
    return {"clearanceMillimetres": clearances, **verify_geometry(detail, native),
            "collarsPerLevel": 3, "collarAxisClearanceMillimetres": verify_collar_bores(detail, native, pieces)}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
