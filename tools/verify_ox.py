"""Replay Ox's exact deck, brace, cabin and aft source geometry checks."""

from collections import Counter
from fractions import Fraction
import json
import struct
import sys

from import_goshawk_envelope_source import components
from prepare_ox import INTEGRATION, ORIGINAL, SCALE, calibrated, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes

OVERLAPS = {(0, 2), (0, 3), (0, 7), (0, 10), (1, 2), (1, 3), (1, 6), (1, 7),
            (1, 9), (1, 10), (4, 5), (6, 7), (9, 10)}


def original_parts(model):
    vertices = [tuple(map(Fraction, p)) for p in model["vertices"]]
    parts = []
    for indices in components(model):
        ids = set(indices)
        faces = [face for face in model["faces"] if face[0] in ids]
        edges = Counter((a,b) for x,y,z in faces for a,b in ((x,y),(y,z),(z,x)))
        require(len(indices) - len(edges) // 2 + len(faces) == 2
                and all(count == edges[(b,a)] == 1 for (a,b),count in edges.items()),
                "changed Ox closed-component topology")
        parts.append(([vertices[i] for i in indices], [[vertices[i] for i in row] for row in faces]))
    return parts


def structural_gaps(parts):
    positions = [part[0] for part in parts]

    def minimum(index, axis):
        return min(p[axis] for p in positions[index])

    def maximum(index, axis):
        return max(p[axis] for p in positions[index])

    distances = {
        "betweenDeckBodies": minimum(1, 1) - maximum(0, 1),
        "betweenUpperDeckAndGlazing": minimum(4, 1) - maximum(1, 1),
        "portDriveRecess": minimum(7, 2) - maximum(8, 2),
        "starboardDriveRecess": minimum(10, 2) - maximum(11, 2),
    }
    require(all(distance > 0 for distance in distances.values()), "changed Ox original structural separation")
    return {label:int(distance * SCALE * 1000) for label,distance in distances.items()}


def verify_geometry(detail, native):
    source = json.loads((INTEGRATION / "ORIGINAL-OVERLAPS.json").read_bytes())
    found = {}
    source_parts = [original_parts(model) for model in (detail,native)]
    for label,model,parts in zip(("design","nativeDesign"), (detail,native), source_parts, strict=True):
        record = source[label]
        require(record["componentCount"] == len(parts) == 12
                and record["surfaceTriangles"] == len(model["faces"]), "changed Ox component counts")
        in_metres = original_parts(calibrated(model))
        pairs = set()
        for row in record["interiorOverlaps"]:
            pair = tuple(row["parts"])
            point = tuple(map(Fraction,row["pointMetres"]))
            parity = [list(inside_y(point, in_metres[index][1])) for index in pair]
            require(parity == row["hitCountsAboveBelow"] and all(n % 2 for hit in parity for n in hit),
                    "changed Ox exact contact witness")
            require(pair not in pairs, "duplicate Ox original overlap")
            pairs.add(pair)
        require(pairs == OVERLAPS, "missing Ox original overlap")
        found[label] = sorted(pairs)
    detail_gaps, native_gaps = map(structural_gaps, source_parts)
    require(detail_gaps == native_gaps == {
        "betweenDeckBodies": 7723, "betweenUpperDeckAndGlazing": 441,
        "portDriveRecess": 137, "starboardDriveRecess": 137,
    }, "changed Ox structural gap floor")
    return {"sourceOverlapPairs": found, "structuralSeparationsMillimetres": detail_gaps}


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH099 proof input: {name}")
    names = json.loads(derived["PIECES.json"])["pieces"]
    pieces = [[] for _ in names]
    vertices = struct.iter_unpack("<3d", derived["source.vertices"][16:])
    for slot, vertex in zip(derived["source-piece-membership.bin"], vertices, strict=True):
        pieces[slot].append(tuple(map(Fraction, vertex)))

    routes = json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes())
    clearances = {}
    for route in routes:
        triangle = [tuple(map(Fraction,p)) for p in route["triangle"]]
        travel = tuple(map(Fraction,route["travel"]))
        sweep = triangle + [tuple(a-b for a,b in zip(p,travel,strict=True)) for p in triangle]
        distance = separating_planes(pieces,sweep,route["axes"])
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Ox swept guard")
        clearances[route["name"]] = distance
    require(len(routes) == 7 and set(clearances) == {
        "dorsal", "ventral", "port-outboard", "starboard-outboard", "centre-service-gap",
        "port-forward-service-gap", "starboard-forward-service-gap",
    }, "wrong Ox route inventory")
    _, _, _, detail, native = load_source()
    return {"clearanceMillimetres":clearances, **verify_geometry(detail,native)}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
