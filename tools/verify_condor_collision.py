"""Check the exact collision members and sweeps loaded by the Condor Rust model."""

from fractions import Fraction
import json
import struct
import sys

from prepare_condor import INTEGRATION, ORIGINAL, encode_json, prepare, require
from verify_eagle_cruiser_geometry import separating_planes


def load_collision():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "collision-piece-membership.bin", "COLLISION-PIECES.json"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed Condor collision input: {name}")
    names = json.loads(derived["COLLISION-PIECES.json"])["pieces"]
    point_stream = derived["source.vertices"]
    points = list(struct.iter_unpack("<3d", point_stream[16:]))
    faces = list(struct.iter_unpack("<3I", derived["source-triangles.bin"]))
    packed = list(struct.iter_unpack("2B", derived["collision-piece-membership.bin"]))
    require(len(packed) == len(points), "wrong Condor collision membership length")
    members = []
    groups = {name: [] for name in names}
    for vertex, (primary, secondary) in enumerate(packed):
        require(primary < len(names), "invalid Condor primary support")
        if secondary != 255:
            require(primary < secondary < len(names), "invalid Condor secondary support")
            first, second = names[primary], names[secondary]
            require(first.endswith("-wing-1") and second == first[:-1] + "2", "overlap leaves the Condor wing seam")
        slots = {primary} if secondary == 255 else {primary, secondary}
        members.append(slots)
        for slot in slots:
            groups[names[slot]].append(vertex)
    return points, faces, members, groups


def verify():
    points, faces, members, groups = load_collision()
    names = tuple(groups)
    require(len(faces) == 4504 + 244, "wrong Condor source face count")
    wing_faces = {"detailed": 0, "native": 0}
    for index, (a, b, c) in enumerate(faces):
        supports = members[a] & members[b] & members[c]
        require(bool(supports), "Condor source triangle leaves local collision supports")
        if any("-wing-" in names[slot] for slot in supports):
            wing_faces["detailed" if index < 4504 else "native"] += 1
    require(wing_faces == {"detailed": 1432, "native": 72}, "wrong Condor wing coverage")
    overlap = sum(len(supports) == 2 for supports in members)
    require(overlap == 56, "wrong Condor wing seam overlap count")

    vertices = [tuple(map(Fraction, p)) for p in points]
    clouds = [[vertices[i] for i in ids] for ids in groups.values()]
    axes = json.loads((INTEGRATION / "COLLISION-ROUTE-AXES.json").read_bytes())
    original = json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes())
    require(axes["pieces"] == list(names), "Condor collision axes use a different group order")
    require(set(axes["routes"]) == {route["name"] for route in original}, "wrong Condor collision routes")
    clearance = {}
    for fixture in original:
        name = fixture["name"]
        face = [tuple(map(Fraction, p)) for p in fixture["triangle"]]
        travel = tuple(map(Fraction, fixture["travel"]))
        sweep = face + [tuple(a - b for a, b in zip(point, travel, strict=True)) for point in face]
        clearance[name] = separating_planes(clouds, sweep, axes["routes"][name])
        require(clearance[name] == fixture["clearanceMillimetres"], "changed guarded Condor collision route")
    return {
        "supportGroups": len(groups), "overlappingVertices": overlap,
        "sourceFacesCovered": {"detailed": 4504, "native": 244}, "wingFacesCovered": wing_faces,
        "clearanceMillimetres": clearance,
    }


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
