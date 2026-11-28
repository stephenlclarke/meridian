"""Recheck the original and candidate Rabbit sweeps using exact source geometry."""

from fractions import Fraction
import json
import struct
import sys

from prepare_rabbit_explorer import ORIGINAL, INTEGRATION, build, encode
from rabbit_explorer_art_mesh import ABSENT_SECONDARY, SCALE, collision_groups, load, require
from verify_eagle_cruiser_geometry import separating_planes


def candidate_clouds():
    outputs = build()
    for name in ("source.vertices", "source-triangles.bin", "collision-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == outputs[name], f"changed Rabbit Explorer proof input: {name}")
    vertices = [tuple(map(Fraction, p)) for p in struct.iter_unpack("<3d", outputs["source.vertices"][16:])]
    pairs = list(struct.iter_unpack("<2B", outputs["collision-piece-membership.bin"]))
    group_names = json.loads(outputs["PIECES.json"])["pieces"]
    clouds = [[] for _ in group_names]
    for vertex, pair in zip(vertices, pairs, strict=True):
        for group in pair:
            if group != ABSENT_SECONDARY:
                clouds[group].append(vertex)
    faces = list(struct.iter_unpack("<3I", outputs["source-triangles.bin"]))
    require(all(any(slot != ABSENT_SECONDARY and all(slot in pairs[index] for index in face)
                    for slot in pairs[face[0]]) for face in faces), "incomplete Rabbit Explorer face coverage")
    return clouds


def archive_clouds(source):
    _, meshes, groups = collision_groups(source["detailed"], source["archiveNative"], candidate=False)
    points = [tuple(Fraction(value * float(SCALE)) for value in vertex)
              for mesh in meshes for vertex in mesh["vertices"]]
    return [[points[index] for index in indices] for indices in groups.values()]


def verify():
    source = load()[3]
    results = {}
    for label, pieces in (("archive", archive_clouds(source)), ("candidate", candidate_clouds())):
        document = json.loads((INTEGRATION / f"{label.upper()}-ROUTES.json").read_bytes())
        require(len(pieces) == document["supportCount"] == 35, "changed Rabbit Explorer route support inventory")
        guards = {}
        for route in document["routes"]:
            triangle = [tuple(map(Fraction, point)) for point in route["triangle"]]
            travel = tuple(map(Fraction, route["travel"]))
            sweep = triangle + [tuple(a - b for a, b in zip(point, travel, strict=True)) for point in triangle]
            guard = separating_planes(pieces, sweep, route["axes"])
            require(guard == route["clearanceMillimetres"] and guard > 100, "changed Rabbit Explorer route guard")
            guards[route["name"]] = guard
        require(len(guards) == len(document["routes"]) == 7, "changed Rabbit Explorer route count")
        results[label] = guards
    return results


if __name__ == "__main__":
    sys.stdout.buffer.write(encode(verify()))
