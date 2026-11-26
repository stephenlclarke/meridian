"""Replay exact source overlaps, structural separations and guarded Wildebeest routes."""

from collections import Counter
from fractions import Fraction
import json
import struct
import sys

from import_goshawk_envelope_source import components
from prepare_wildebeest import INTEGRATION, ORIGINAL, calibrated, encode_json, load_source, prepare, require
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
                "Wildebeest source component has changed topology")
        result.append(([points[i] for i in indices], [[points[i] for i in face] for face in faces]))
    return result


def verify_geometry(detail, native):
    witnesses = json.loads((INTEGRATION / "ORIGINAL-OVERLAPS.json").read_bytes())
    overlaps = {}
    for lod, model, expected_components in (("design", calibrated(detail), 16), ("nativeDesign", calibrated(native), 16)):
        parts = original_parts(model)
        record = witnesses[lod]
        require(len(parts) == record["componentCount"] == expected_components
                and len(model["faces"]) == record["surfaceTriangles"], "changed Wildebeest source inventory")
        joined = set()
        for row in record["interiorOverlaps"]:
            pair = tuple(row["parts"])
            point = tuple(map(Fraction, row["pointMetres"]))
            counts = [list(inside_y(point, parts[index][1])) for index in pair]
            require(counts == row["hitCountsAboveBelow"] and all(n % 2 for hit in counts for n in hit),
                    "changed Wildebeest exact overlap")
            require(pair not in joined, "duplicate Wildebeest overlap")
            joined.add(pair)
        overlaps[lod] = joined
    expected = {'design': {(0, 2),
                (0, 3),
                (0, 4),
                (0, 6),
                (0, 7),
                (0, 10),
                (0, 11),
                (0, 13),
                (0, 14),
                (1, 10),
                (1, 13),
                (2, 4),
                (2, 5),
                (2, 6),
                (3, 4),
                (3, 5),
                (3, 7),
                (4, 6),
                (4, 7),
                (5, 6),
                (5, 7),
                (8, 9),
                (10, 11),
                (13, 14)},
     'nativeDesign': {(0, 2),
                      (0, 3),
                      (0, 4),
                      (0, 6),
                      (0, 7),
                      (0, 10),
                      (0, 11),
                      (0, 13),
                      (0, 14),
                      (1, 10),
                      (1, 13),
                      (2, 4),
                      (2, 5),
                      (2, 6),
                      (3, 4),
                      (3, 5),
                      (3, 7),
                      (4, 6),
                      (4, 7),
                      (5, 6),
                      (5, 7),
                      (8, 9),
                      (10, 11),
                      (13, 14)}}
    require(overlaps == expected, "changed Wildebeest original overlap sets")
    shared = overlaps["design"] & overlaps["nativeDesign"]
    return {"sourceOverlapPairs": {lod:sorted(pairs) for lod,pairs in overlaps.items()},
            "sharedOverlapCount": len(shared), "detailedOnlyOverlapCount": len(overlaps["design"] - shared)}


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH104 proof input: {name}")
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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Wildebeest sweep")
        clearances[route["name"]] = distance
    require(len(routes) == 7 and set(clearances) == {
        "dorsal", "ventral", "centre-cargo-throat", "port-outboard", "starboard-outboard",
        "port-cargo-throat", "starboard-cargo-throat",
    }, "wrong Wildebeest route inventory")
    _, _, _, detail, native = load_source()
    return {"clearanceMillimetres": clearances, **verify_geometry(detail, native)}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
