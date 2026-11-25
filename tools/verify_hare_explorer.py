"""Replay the exact original SH083 clearance and joint proofs."""

from fractions import Fraction
import json
from math import isqrt
import struct
import sys

from prepare_hare_explorer import INTEGRATION, ORIGINAL, encode_json, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def prove_nozzle_contacts(vertices, faces, membership, names, detail_faces):
    z = Fraction(-15)
    result = {}
    for side, sign in (("port", -1), ("starboard", 1)):
        x, y = sign * Fraction(119, 100), Fraction(3, 10)
        hull, nozzle = (names.index(name) for name in ("central-hull", f"{side}-nozzle"))
        directions = {hull: -1, nozzle: 1}
        found = {}
        for index, face in enumerate(faces[:detail_faces]):
            slot = membership[face[0]]
            if slot not in directions or not all(membership[i] == slot and vertices[i][2] == z for i in face):
                continue
            a, b, c = [vertices[i] for i in face]
            outward = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
            if not all(((q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0])) * outward > 0
                       for p, q in ((a, b), (b, c), (c, a))):
                continue
            require(outward * directions[slot] > 0 and slot not in found, "wrong Hare Explorer cap orientation")
            found[slot] = index
        require(set(found) == set(directions), "missing Hare Explorer detailed cap contact")
        result[side] = {"pointMetres": [float(x), float(y), float(z)],
                        "detailedFaces": {names[slot]: index for slot, index in found.items()}}
    return result


def prove_native_separation(vertices, membership, names, detail_vertices):
    native = [[] for _ in names]
    for slot, point in zip(membership[detail_vertices:], vertices[detail_vertices:], strict=True):
        native[slot].append(point)
    hull = native[names.index("central-hull")]
    result = {}
    for side, sign in (("port", -1), ("starboard", 1)):
        for part, axis, expected in (
            ("nozzle", (sign * 687254, -687254, -393543), 800),
            ("forward-boom", (sign * 593280, -844975, 183161), 732),
        ):
            name = f"{side}-{part}"

            def project(point):
                return sum(a * b for a, b in zip(axis, point, strict=True))

            gap = min(map(project, native[names.index(name)])) - max(map(project, hull))
            require(gap > 0, "changed original Hare Explorer native separation")
            millimetres = isqrt((gap * 1000)**2 // sum(value * value for value in axis))
            require(millimetres == expected, "changed original Hare Explorer native gap")
            result[name] = {"axis": axis, "separationMillimetres": millimetres}
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
                        "changed Hare Explorer original joint")
        checked.append(joint["name"])
    require(len(set(checked)) == len(checked), "duplicate Hare Explorer original joint")
    return checked


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH083 proof input: {name}")

    names = json.loads(derived["PIECES.json"])["pieces"]
    detail_vertices, detail_faces = 1612, 3108
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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Hare Explorer sweep")
        clearances[route["name"]] = distance
    require(len(clearances) == len(routes) == 7, "wrong Hare Explorer route count")

    faces = list(struct.iter_unpack("<3I", derived["source-triangles.bin"]))
    geometry = (vertices, faces, membership, names, detail_vertices, detail_faces)
    shared_source = json.loads((INTEGRATION / "ORIGINAL-JOINTS.json").read_bytes())
    shared = prove_joints(shared_source, {"detailed", "native"}, *geometry)
    expected = {"canopy-to-hull", *(f"{side}-wing-mount-to-{part}"
                for side in ("port", "starboard") for part in ("hull", "wing"))}
    require(set(shared) == expected, "changed Hare Explorer original shared overlaps")
    detail_source = json.loads((INTEGRATION / "DETAILED-ONLY-JOINTS.json").read_bytes())
    detail = prove_joints(detail_source, {"detailed"}, *geometry)
    require(set(detail) == {"canopy-ridge-to-canopy", "port-forward-boom-to-hull",
                           "starboard-forward-boom-to-hull"}, "changed Hare Explorer detailed overlaps")
    require(names.index("canopy-ridge") not in membership[detail_vertices:], "Hare Explorer native gained a ridge")
    contacts = prove_nozzle_contacts(vertices, faces, membership, names, detail_faces)
    gaps = prove_native_separation(vertices, membership, names, detail_vertices)
    return {"clearanceMillimetres": clearances, "sourceJointNames": shared, "detailedOnlyJointNames": detail,
            "detailedFlushNozzleContacts": contacts, "originalNativeHullSeparation": gaps}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
