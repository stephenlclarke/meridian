"""Replay the exact original SH078 clearance and joint proofs."""

from collections import Counter
from fractions import Fraction
import json
import struct
import sys

from prepare_oryx_voyager import INTEGRATION, ORIGINAL, encode_json, prepare, require
from dzo_collar_support import ABSENT
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin", "collision-piece-membership.bin", "COLLISION-PIECES.json"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH078 proof input: {name}")

    names = json.loads(derived["PIECES.json"])["pieces"]
    detail_vertices, detail_faces = 2364, 4620
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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Oryx Voyager sweep")
        clearances[route["name"]] = distance
    require(len(clearances) == len(routes) == 5, "wrong Oryx Voyager route count")

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
                        "changed Oryx Voyager original joint")
        checked.append(joint["name"])
    require(len(set(checked)) == len(checked) == 5, "wrong Oryx Voyager original joint count")
    ring = names.index("ring-wing")
    hull = names.index("central-hull")
    occupied = {}
    topology = {}
    center = (.19, .13, -2.11)
    for lod, source in (("detailed", faces[:detail_faces]), ("native", faces[detail_faces:])):
        ring_faces = [face for face in source if all(membership[i] == ring for i in face)]
        directed = Counter((a, b) for triangle in ring_faces for a, b in zip(triangle, (*triangle[1:], triangle[0]), strict=True))
        require(all(count == directed[b, a] == 1 for (a, b), count in directed.items()),
                "Oryx Voyager ring winding or closure changed")
        vertex_count = len({i for face in ring_faces for i in face})
        expected_faces = 1536 if lod == "detailed" else 64
        require(len(ring_faces) == expected_faces and vertex_count + len(ring_faces) == len(directed) // 2,
                "Oryx Voyager ring lost its genus-one topology")
        topology[lod] = {"vertices": vertex_count, "faces": len(ring_faces), "eulerCharacteristic": 0}
        center_hull = [[vertices[i] for i in face] for face in source if all(membership[i] == hull for i in face)]
        occupied[lod] = list(inside_y(tuple(map(Fraction, center)), center_hull))
    collision_names = json.loads(derived["COLLISION-PIECES.json"])["pieces"]
    collisions = [[] for _ in collision_names]
    memberships = list(struct.iter_unpack("<2B", derived["collision-piece-membership.bin"]))
    require(len(collisions) == 31 and sum(second != ABSENT for _, second in memberships) == 128,
            "wrong Oryx Voyager local collar spans")
    for point, pair in zip(vertices, memberships, strict=True):
        for group in pair:
            if group != ABSENT:
                collisions[group].append(point)
    require(all(any(slot != ABSENT and all(slot in memberships[i] for i in face)
                    for slot in memberships[face[0]]) for face in faces), "uncovered Oryx Voyager source faces")
    refinements = {}
    split_routes = json.loads((INTEGRATION / "SPLIT-ROUTES.json").read_bytes())
    original_routes = {row["name"]: row for row in routes}
    for split in split_routes:
        name = split["name"]
        if name in original_routes:
            route = original_routes[name]
            require((route["triangle"], route["travel"]) == (split["triangle"], split["travel"]),
                    "changed Oryx Voyager exterior inspection sweep")
        triangle = [tuple(map(Fraction, p)) for p in split["triangle"]]
        travel = tuple(map(Fraction, split["travel"]))
        sweep = triangle + [tuple(a - b for a, b in zip(p, travel, strict=True)) for p in triangle]
        guard = separating_planes(collisions, sweep, split["axes"])
        require(guard == split["clearanceMillimetres"] and guard >= clearances.get(name, 100),
                "changed Oryx Voyager local ring span clearance")
        refinements[name] = guard
    require(len(split_routes) == len(refinements) == 7 and set(refinements) == set(clearances) | {
        "port-inner-ring", "starboard-inner-ring",
    }, "wrong Oryx Voyager route inventory")
    return {"clearanceMillimetres": clearances, "collisionClearanceMillimetres": refinements, "sourceJointNames": checked, "ringTopology": topology, "occupiedCentreHull": {"pointMetres": center, "hitCountsAboveBelow": occupied}}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
