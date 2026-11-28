"""Replay the exact original SH046 clearance and joint proofs."""

from fractions import Fraction
import json
import struct
import sys

from prepare_harpy_dropship import INTEGRATION, ORIGINAL, encode_json, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH046 proof input: {name}")

    names = json.loads(derived["PIECES.json"])["pieces"]
    detail_vertices, detail_faces = 1628, 3132
    vertices = [tuple(map(Fraction, item)) for item in struct.iter_unpack("<3d", derived["source.vertices"][16:])]
    membership = derived["source-piece-membership.bin"]
    pieces = [[] for _ in names]
    for slot, vertex in zip(membership, vertices, strict=True):
        pieces[slot].append(vertex)

    routes = json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes())
    clearances = {}
    for route in routes:
        triangle = [tuple(map(Fraction, p)) for p in route["triangle"]]
        travel = tuple(map(Fraction, route["travel"]))
        sweep = triangle + [tuple(a - b for a, b in zip(p, travel, strict=True)) for p in triangle]
        distance = separating_planes(pieces, sweep, route["axes"])
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Harpy Dropship sweep")
        clearances[route["name"]] = distance
    require(len(clearances) == len(routes) == 7, "wrong Harpy Dropship route count")

    faces = list(struct.iter_unpack("<3I", derived["source-triangles.bin"]))
    bindings = json.loads((INTEGRATION / "ORIGINAL-JOINTS.json").read_bytes())
    checked = []
    for joint in bindings:
        point = tuple(map(Fraction, joint["pointMetres"]))
        require(set(joint["originalFaces"]) == {"detailed", "native"}, "missing joint source mesh")
        require(set(joint["originalFaces"]["detailed"]) == set(joint["originalFaces"]["native"]),
                "joint solids differ by mesh")
        for lod, supports in joint["originalFaces"].items():
            require(len(supports) == 2, "joint must be shared by two source solids")
            offset = 0 if lod == "detailed" else detail_faces
            first, last = (0, detail_vertices) if lod == "detailed" else (detail_vertices, len(vertices))
            for name, proof in supports.items():
                start, end = (offset + i for i in proof["faceRange"])
                slot = names.index(name)
                source_indices = faces[start:end]
                require(bool(source_indices) and all(first <= i < last and membership[i] == slot
                        for triangle in source_indices for i in triangle), "joint faces leave original solid")
                triangles = [[vertices[i] for i in triangle] for triangle in source_indices]
                require(list(inside_y(point, triangles)) == proof["hitCountsAboveBelow"],
                        "changed Harpy Dropship original joint")
        checked.append(joint["name"])
    require(len(set(checked)) == len(checked) == 7, "wrong Harpy Dropship original joint count")
    return {"clearanceMillimetres": clearances, "sourceJointNames": checked}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
