"""Replay the exact original SH074 clearance and joint proofs."""

from fractions import Fraction
import json
import struct
import sys

from prepare_dzo import INTEGRATION, ORIGINAL, encode_json, prepare, require
from dzo_collar_support import ABSENT
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin", "collision-piece-membership.bin", "COLLISION-PIECES.json"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH074 proof input: {name}")

    names = json.loads(derived["PIECES.json"])["pieces"]
    detail_vertices, detail_faces = 4668, 9228
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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Dzo sweep")
        clearances[route["name"]] = distance
    require(len(clearances) == len(routes) == 7, "wrong Dzo route count")

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
                        "changed Dzo original joint")
        checked.append(joint["name"])
    require(len(set(checked)) == len(checked) == 5, "wrong Dzo original joint count")
    centers = {}
    for side, x in (("port", -7.025), ("starboard", 7.025)):
        slot = names.index(f"{side}-float")
        for end, z in (("aft", -12.989), ("forward", 12.011)):
            point = (x, -.96, z)
            result = centers[f"{side}-{end}"] = {"pointMetres": list(point)}
            for lod, part_faces in (("detailed", faces[:detail_faces]), ("native", faces[detail_faces:])):
                triangles = [[vertices[i] for i in face] for face in part_faces if all(membership[i] == slot for i in face)]
                require(len(triangles) == (384 if lod == "detailed" else 32), "wrong Dzo float surface")
                result[lod] = list(inside_y(tuple(map(Fraction, point)), triangles))
    collision_names = json.loads(derived["COLLISION-PIECES.json"])["pieces"]
    collisions = [[] for _ in collision_names]
    memberships = list(struct.iter_unpack("<2B", derived["collision-piece-membership.bin"]))
    require(len(collisions) == 55 and sum(second != ABSENT for _, second in memberships) == 512,
            "wrong Dzo local collar spans")
    for point, pair in zip(vertices, memberships, strict=True):
        for group in pair:
            if group != ABSENT:
                collisions[group].append(point)
    require(all(any(slot != ABSENT and all(slot in memberships[i] for i in face)
                    for slot in memberships[face[0]]) for face in faces), "uncovered Dzo source faces")
    refinements = {}
    split_routes = json.loads((INTEGRATION / "SPLIT-ROUTES.json").read_bytes())
    for route, split in zip(routes, split_routes, strict=True):
        require((route["name"], route["triangle"], route["travel"]) == (split["name"], split["triangle"], split["travel"]),
                "changed Dzo split inspection sweep")
        triangle = [tuple(map(Fraction, p)) for p in split["triangle"]]
        travel = tuple(map(Fraction, split["travel"]))
        sweep = triangle + [tuple(a - b for a, b in zip(p, travel, strict=True)) for p in triangle]
        guard = separating_planes(collisions, sweep, split["axes"])
        require(guard == split["clearanceMillimetres"] and guard >= clearances[split["name"]],
                "changed Dzo local collar span clearance")
        refinements[split["name"]] = guard
    return {"clearanceMillimetres": clearances, "collisionClearanceMillimetres": refinements, "sourceJointNames": checked, "occupiedCollarCentres": centers}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
