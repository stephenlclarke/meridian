"""Replay the exact original SH077 clearance and joint proofs."""

from fractions import Fraction
import json
import struct
import sys

from prepare_tamaraw import INTEGRATION, ORIGINAL, encode_json, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def prove_rail_contacts(vertices, faces, membership, names, detail_faces):
    y, z = Fraction(79, 100), Fraction(37, 100)
    result = {}
    for side, sign in (("port", -1), ("starboard", 1)):
        x = sign * Fraction(27, 2)
        rail = names.index(f"{side}-rail")
        hull = names.index("central-hull")
        directions = {hull: sign, rail: -sign}
        found = {}
        require(all(membership[corner] != rail for face in faces[detail_faces:] for corner in face),
                "Tamaraw rail appeared in the original native mesh")
        for index, face in enumerate(faces[:detail_faces]):
            slot = membership[face[0]]
            if slot not in directions or not all(membership[i] == slot and vertices[i][0] == x for i in face):
                continue
            a, b, c = [vertices[i] for i in face]
            outward = (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1])
            if not all(((q[1] - p[1]) * (z - p[2]) - (q[2] - p[2]) * (y - p[1])) * outward > 0
                       for p, q in ((a, b), (b, c), (c, a))):
                continue
            require(outward * directions[slot] > 0 and slot not in found, "wrong Tamaraw flush face orientation")
            found[slot] = index
        require(set(found) == set(directions), "missing Tamaraw flush side-rail contact")
        result[side] = {"pointMetres": [float(x), float(y), float(z)],
                        "detailedFaces": {names[slot]: index for slot, index in found.items()}}
    return result


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH077 proof input: {name}")

    names = json.loads(derived["PIECES.json"])["pieces"]
    detail_vertices, detail_faces = 1992, 3864
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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Tamaraw sweep")
        clearances[route["name"]] = distance
    require(len(clearances) == len(routes) == 7, "wrong Tamaraw route count")

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
                        "changed Tamaraw original joint")
        checked.append(joint["name"])
    require(len(set(checked)) == len(checked) == 8, "wrong Tamaraw original joint count")
    contacts = prove_rail_contacts(vertices, faces, membership, names, detail_faces)
    return {"clearanceMillimetres": clearances, "sourceJointNames": checked, "detailedFlushRailContacts": contacts}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
