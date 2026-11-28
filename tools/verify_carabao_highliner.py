"""Replay the exact original SH080 clearance and joint proofs."""

from fractions import Fraction
import json
import struct
import sys

from prepare_carabao_highliner import INTEGRATION, ORIGINAL, encode_json, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def prove_rail_contacts(vertices, faces, membership, names, detail_faces):
    y = Fraction(11, 2)
    result = {}
    for side, sign in (("port", -1), ("starboard", 1)):
        x = sign * Fraction(1213, 100)
        rail = names.index(f"{side}-rail")
        require(all(membership[corner] != rail for face in faces[detail_faces:] for corner in face),
                "Carabao Highliner rail appeared in the original native mesh")
        for number, position in enumerate((-243, -73, 97, 272), 1):
            z = Fraction(position, 10)
            bay_name = f"{side}-bay-{number}"
            bay = names.index(bay_name)
            directions = {bay: 1, rail: -1}
            found = {}
            for index, face in enumerate(faces[:detail_faces]):
                slot = membership[face[0]]
                if slot not in directions or not all(membership[i] == slot and vertices[i][1] == y for i in face):
                    continue
                a, b, c = [vertices[i] for i in face]
                outward = (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2])
                if not all(((q[2] - p[2]) * (x - p[0]) - (q[0] - p[0]) * (z - p[2])) * outward > 0
                           for p, q in ((a, b), (b, c), (c, a))):
                    continue
                require(outward * directions[slot] > 0 and slot not in found, "wrong Carabao Highliner rail orientation")
                found[slot] = index
            require(set(found) == set(directions), "missing Carabao Highliner flush rail contact")
            result[bay_name] = {"pointMetres": [float(x), float(y), float(z)],
                                "detailedFaces": {names[slot]: index for slot, index in found.items()}}
    return result


def prove_joints(bindings, expected_lods, vertices, faces, membership, names, detail_vertices, detail_faces):
    checked = []
    for joint in bindings:
        point = tuple(map(Fraction, joint["pointMetres"]))
        require(set(joint["originalFaces"]) == expected_lods, "missing joint source mesh")
        reference = set(joint["originalFaces"]["detailed"])
        require(all(set(parts) == reference for parts in joint["originalFaces"].values()), "joint solids differ by mesh")
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
                        "changed Carabao Highliner original joint")
        checked.append(joint["name"])
    require(len(set(checked)) == len(checked), "duplicate Carabao Highliner original joint")
    return checked


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH080 proof input: {name}")

    names = json.loads(derived["PIECES.json"])["pieces"]
    detail_vertices, detail_faces = 1506, 2856
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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Carabao Highliner sweep")
        clearances[route["name"]] = distance
    require(len(clearances) == len(routes) == 7, "wrong Carabao Highliner route count")

    faces = list(struct.iter_unpack("<3I", derived["source-triangles.bin"]))
    geometry = (vertices, faces, membership, names, detail_vertices, detail_faces)
    shared_source = json.loads((INTEGRATION / "ORIGINAL-JOINTS.json").read_bytes())
    shared = prove_joints(shared_source, {"detailed", "native"}, *geometry)
    require(len(shared) == 14 and "passenger-spine-to-cabin" in shared, "missing original shared Highliner joints")
    rail_source = json.loads((INTEGRATION / "DETAILED-SPINE-JOINTS.json").read_bytes())
    spine_contacts = prove_joints(rail_source, {"detailed"}, *geometry)
    require(set(spine_contacts) == {"port-spine-rail", "starboard-spine-rail"}, "missing original Highliner rail joints")
    rail_slots = {names.index(name) for name in spine_contacts}
    require(not rail_slots.intersection(membership[detail_vertices:]), "Highliner native mesh gained a detail rail")
    gaps = {}
    for side in ("port", "starboard"):
        for number in range(1, 4):
            first, second = (pieces[names.index(f"{side}-bay-{i}")] for i in (number, number + 1))
            distance = min(point[2] for point in second) - max(point[2] for point in first)
            require(distance == 4, "changed Carabao Highliner original interbay spacing")
            gaps[f"{side}-interbay-{number}"] = 4000
    contacts = prove_rail_contacts(vertices, faces, membership, names, detail_faces)
    return {"clearanceMillimetres": clearances, "sourceJointNames": shared, "detailedSpineRailJoints": spine_contacts, "detailedFlushRailContacts": contacts, "originalInterbaySpacingMillimetres": gaps}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
