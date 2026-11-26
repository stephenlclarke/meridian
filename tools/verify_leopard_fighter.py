"""Replay exact source overlaps, structural separations and guarded Leopard Fighter routes."""

from collections import Counter
from fractions import Fraction
import json
import struct
import sys

from import_goshawk_envelope_source import components
from prepare_leopard_fighter import INTEGRATION, NATIVE_COMPONENTS, ORIGINAL, encode_json, load_source, prepare, require
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
                "Leopard Fighter source component has changed topology")
        result.append(([points[i] for i in indices], [[points[i] for i in face] for face in faces]))
    return result


def verify_geometry(detail, native):
    witnesses = json.loads((INTEGRATION / "ORIGINAL-OVERLAPS.json").read_bytes())
    overlaps = {}
    for lod, model, expected_components in (("design", detail, 26), ("nativeDesign", native, 7)):
        parts = original_parts(model)
        record = witnesses[lod]
        require(len(parts) == record["componentCount"] == expected_components
                and len(model["faces"]) == record["surfaceTriangles"], "changed Leopard Fighter source inventory")
        joined = set()
        for row in record["interiorOverlaps"]:
            pair = tuple(row["parts"])
            point = tuple(map(Fraction, row["pointMetres"]))
            counts = [list(inside_y(point, parts[index][1])) for index in pair]
            require(counts == row["hitCountsAboveBelow"] and all(n % 2 for hit in counts for n in hit),
                    "changed Leopard Fighter exact overlap")
            require(pair not in joined, "duplicate Leopard Fighter overlap")
            joined.add(pair)
        overlaps[lod] = joined
    expected = {'design': {(0, 1),
                (0, 2),
                (0, 6),
                (1, 10),
                (1, 12),
                (1, 16),
                (1, 17),
                (1, 18),
                (1, 19),
                (1, 23),
                (1, 24),
                (1, 25),
                (3, 4),
                (4, 5),
                (7, 8),
                (8, 9),
                (10, 11),
                (12, 13),
                (12, 14),
                (12, 15),
                (12, 16),
                (12, 17),
                (19, 20),
                (19, 21),
                (19, 22),
                (19, 23),
                (19, 24)},
     'nativeDesign': {(0, 1), (1, 6), (0, 2), (0, 4)}}
    require(overlaps == expected, "changed Leopard Fighter measured overlap sets")
    shared = {tuple(NATIVE_COMPONENTS[i] for i in pair) for pair in overlaps["nativeDesign"]}
    require(shared <= overlaps["design"], "changed Leopard Fighter shared original overlap set")
    details, low = map(original_parts, (detail, native))
    for parts, pairs in ((details, ((2, 3), (6, 7))), (low, ((2, 3), (4, 5)))):
        for boom, drive in pairs:
            gap = min(point[2] for point in parts[boom][0]) - max(point[2] for point in parts[drive][0])
            require(gap == 2, "changed Leopard Fighter original boom-to-drive separation")
    return {"sourceOverlapPairs": {lod:sorted(pairs) for lod,pairs in overlaps.items()},
            "sharedOverlapCount": len(shared), "detailedOnlyOverlapCount": len(overlaps["design"] - shared),
            "boomToDriveSeparationMillimetres": 2000}


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH110 proof input: {name}")
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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Leopard Fighter sweep")
        clearances[route["name"]] = distance
    require(len(routes) == 7 and set(clearances) == {
        "dorsal", "ventral", "between-aft-drives", "port-outboard", "starboard-outboard",
        "inboard-of-port-boom", "inboard-of-starboard-boom",
    }, "wrong Leopard Fighter route inventory")
    _, _, _, detail, native = load_source()
    return {"clearanceMillimetres": clearances, **verify_geometry(detail, native)}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
