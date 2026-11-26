"""Replay the exact original SH084 clearance and joint proofs."""

from fractions import Fraction
import json
from math import isqrt
import struct
import sys

from prepare_llama import INTEGRATION, ORIGINAL, encode_json, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def prove_nozzle_contacts(vertices, faces, membership, names, detail_faces):
    z = Fraction(-23)
    result = {}
    for side, cx in (("port", -7), ("starboard", 7)):
        cy = -2
        stem = side
        x = cx + Fraction(1, 5)
        y = cy + Fraction(7, 100)
        directions = {names.index(f"{stem}-hull"): -1, names.index(f"{stem}-nozzle"): 1}
        original_faces = {}
        for lod, start, end in (("detailed", 0, detail_faces), ("native", detail_faces, len(faces))):
            found = {}
            for index in range(start, end):
                face = faces[index]
                slot = membership[face[0]]
                if slot not in directions or not all(membership[i] == slot and vertices[i][2] == z for i in face):
                    continue
                a, b, c = [vertices[i] for i in face]
                outward = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
                if not all(((q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0])) * outward > 0
                           for p, q in ((a, b), (b, c), (c, a))):
                    continue
                require(outward * directions[slot] > 0 and slot not in found, "wrong Llama nozzle cap orientation")
                found[slot] = index - start
            require(set(found) == set(directions), "missing original Llama nozzle cap contact")
            original_faces[lod] = {names[slot]: index for slot, index in found.items()}
        result[stem] = {"pointMetres": [float(x), float(y), float(z)], "originalFaces": original_faces}
    return result


def prove_crossbeam_separation(vertices, membership, names, detail_vertices):
    result = {}
    for lod, start, end, axis, expected in (
        ("detailed", 0, detail_vertices, (0, 260485, 29446), 1680),
        ("native", detail_vertices, len(vertices), (0, 1028791, 202732), 2270),
    ):
        pieces = [[] for _ in names]
        for slot, point in zip(membership[start:end], vertices[start:end], strict=True):
            pieces[slot].append(point)

        def project(point):
            return sum(a * b for a, b in zip(axis, point, strict=True))

        crossbeam = min(map(project, pieces[names.index("crossbeam")]))
        for side in ("port", "starboard"):
            gap = crossbeam - max(map(project, pieces[names.index(f"{side}-hull")]))
            require(gap > 0, "changed original Llama crossbeam separation")
            distance = isqrt((gap * 1000)**2 // sum(value * value for value in axis))
            require(distance == expected, "changed original Llama crossbeam gap")
            result[f"{lod}-{side}"] = {"axis": axis, "separationMillimetres": distance}
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
                        "changed Llama original joint")
        checked.append(joint["name"])
    require(len(set(checked)) == len(checked), "duplicate Llama original joint")
    return checked


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH084 proof input: {name}")

    names = json.loads(derived["PIECES.json"])["pieces"]
    detail_vertices, detail_faces = 1604, 3096
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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Llama sweep")
        clearances[route["name"]] = distance
    require(len(clearances) == len(routes) == 7, "wrong Llama route count")

    faces = list(struct.iter_unpack("<3I", derived["source-triangles.bin"]))
    geometry = (vertices, faces, membership, names, detail_vertices, detail_faces)
    shared_source = json.loads((INTEGRATION / "ORIGINAL-JOINTS.json").read_bytes())
    shared = prove_joints(shared_source, {"detailed", "native"}, *geometry)
    expected = {"canopy-to-cabin", "crossbeam-to-cabin",
                "port-deck-rail-to-hull", "starboard-deck-rail-to-hull"}
    require(set(shared) == expected, "changed Llama original shared overlaps")
    detail_source = json.loads((INTEGRATION / "DETAILED-ONLY-JOINTS.json").read_bytes())
    detail = prove_joints(detail_source, {"detailed"}, *geometry)
    require(detail == ["canopy-ridge-to-canopy"], "changed Llama detailed overlap")
    require(names.index("canopy-ridge") not in membership[detail_vertices:], "Llama native gained a ridge")
    contacts = prove_nozzle_contacts(vertices, faces, membership, names, detail_faces)
    gaps = prove_crossbeam_separation(vertices, membership, names, detail_vertices)
    return {"clearanceMillimetres": clearances, "sourceJointNames": shared, "detailedOnlyJointNames": detail,
            "flushNozzleContacts": contacts, "originalCrossbeamHullSeparation": gaps}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
