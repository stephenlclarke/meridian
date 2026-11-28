"""Replay the exact original SH064 clearance and joint proofs."""

from fractions import Fraction
import json
import struct
import sys

from prepare_arna_cutter import INTEGRATION, ORIGINAL, encode_json, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def prove_deck_contacts(vertices, faces, membership, group_names, detail_faces):
    """Find each unchanged, oppositely oriented deck contact in both mesh levels."""
    y = Fraction(-7, 2)
    x = Fraction(37, 100)
    fixtures = {"aft-compartment": Fraction(-152, 10), "centre-compartment": Fraction(6, 10),
                "forward-compartment": Fraction(149, 10)}
    result = {}
    for compartment, z in fixtures.items():
        groups = {group_names.index("keel-deck"): 1, group_names.index(compartment): -1}
        views = {}
        for lod, offset, end in (("detailed", 0, detail_faces), ("native", detail_faces, len(faces))):
            found = {}
            for face_index in range(offset, end):
                a, b, c = face = faces[face_index]
                slot = membership[a]
                if slot not in groups or not all(membership[index] == slot and vertices[index][1] == y for index in face):
                    continue
                triangle = [vertices[i] for i in face]
                pa, pb, pc = triangle
                normal_y = (pb[2] - pa[2]) * (pc[0] - pa[0]) - (pb[0] - pa[0]) * (pc[2] - pa[2])
                # Each oriented edge must place the witness strictly inside the x-z face projection.
                edge_pairs = ((pa, pb), (pb, pc), (pc, pa))
                if not all(((q[2] - p[2]) * (x - p[0]) - (q[0] - p[0]) * (z - p[2])) * normal_y > 0
                           for p, q in edge_pairs):
                    continue
                require(normal_y * groups[slot] > 0 and slot not in found, "wrong Arna Cutter deck face orientation")
                found[slot] = face_index - offset
            require(set(found) == set(groups), "missing Arna Cutter flush deck contact")
            views[lod] = {group_names[slot]: found[slot] for slot in groups}
        result[compartment] = {"pointMetres": [float(x), float(y), float(z)], "sourceFaces": views}
    return result


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH064 proof input: {name}")

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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Arna Cutter sweep")
        clearances[route["name"]] = distance
    require(len(clearances) == len(routes) == 7, "wrong Arna Cutter route count")

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
                        "changed Arna Cutter original joint")
        checked.append(joint["name"])
    require(len(set(checked)) == len(checked) == 5, "wrong Arna Cutter original joint count")
    deck = prove_deck_contacts(vertices, faces, membership, names, detail_faces)
    return {"clearanceMillimetres": clearances, "sourceJointNames": checked, "flushDeckContacts": deck}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
