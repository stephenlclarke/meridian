"""Replay exact source overlaps, structural separations and guarded Warthog Escort routes."""

from collections import Counter
from fractions import Fraction
import json
import struct
import sys

from import_goshawk_envelope_source import components
from prepare_warthog_escort import INTEGRATION, NATIVE_COMPONENTS, ORIGINAL, encode_json, load_source, prepare, require
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
                "Warthog Escort source component has changed topology")
        result.append(([points[i] for i in indices], [[points[i] for i in face] for face in faces]))
    return result


def verify_geometry(detail, native):
    witnesses = json.loads((INTEGRATION / "ORIGINAL-OVERLAPS.json").read_bytes())
    overlaps = {}
    for lod, model, expected_components in (("design", detail, 32), ("nativeDesign", native, 11)):
        parts = original_parts(model)
        record = witnesses[lod]
        require(len(parts) == record["componentCount"] == expected_components
                and len(model["faces"]) == record["surfaceTriangles"], "changed Warthog Escort source inventory")
        joined = set()
        for row in record["interiorOverlaps"]:
            pair = tuple(row["parts"])
            point = tuple(map(Fraction, row["pointMetres"]))
            counts = [list(inside_y(point, parts[index][1])) for index in pair]
            require(counts == row["hitCountsAboveBelow"] and all(n % 2 for hit in counts for n in hit),
                    "changed Warthog Escort exact overlap")
            require(pair not in joined, "duplicate Warthog Escort overlap")
            joined.add(pair)
        overlaps[lod] = joined
    expected = {'design': {(0, 1),
                (0, 3),
                (0, 6),
                (0, 8),
                (0, 11),
                (1, 2),
                (3, 4),
                (4, 5),
                (6, 7),
                (8, 9),
                (9, 10),
                (11, 13),
                (11, 15),
                (11, 16),
                (12, 13),
                (14, 15),
                (16, 17),
                (18, 19),
                (18, 20),
                (18, 21),
                (18, 22),
                (18, 23),
                (18, 24),
                (25, 26),
                (25, 27),
                (25, 28),
                (25, 29),
                (25, 30),
                (25, 31)},
     'nativeDesign': {(0, 1),
                      (0, 2),
                      (0, 3),
                      (0, 4),
                      (0, 5),
                      (5, 7),
                      (5, 9),
                      (5, 10),
                      (6, 7),
                      (8, 9)}}
    require(overlaps == expected, "changed Warthog Escort measured overlap sets")
    shared = {tuple(NATIVE_COMPONENTS[i] for i in pair) for pair in overlaps["nativeDesign"]}
    require(shared <= overlaps["design"], "changed Warthog Escort shared original overlap set")
    return {"sourceOverlapPairs": {lod:sorted(pairs) for lod,pairs in overlaps.items()},
            "sharedOverlapCount": len(shared), "detailedOnlyOverlapCount": len(overlaps["design"] - shared)}


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH129 proof input: {name}")
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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Warthog Escort sweep")
        clearances[route["name"]] = distance
    require(len(routes) == 7 and set(clearances) == {
        "dorsal", "ventral", "above-aft-pressure-body", "port-outboard", "starboard-outboard",
        "over-port-drive", "over-starboard-drive",
    }, "wrong Warthog Escort route inventory")
    _, _, _, detail, native = load_source()
    return {"clearanceMillimetres": clearances, **verify_geometry(detail, native)}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
