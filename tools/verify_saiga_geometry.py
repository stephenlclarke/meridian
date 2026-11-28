"""Verify both Saiga collar apertures against the bundled overlapping supports."""

from fractions import Fraction
import json
import struct
import sys
import tomllib

from prepare_saiga_geometry import INTEGRATION, ORIGINAL, build, encode, require
from saiga_art_mesh import ABSENT_SECONDARY, COLLAR, SCALE, assemble, collision_groups
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def load_collision():
    generated = build()
    for name in ("source.vertices", "source-triangles.bin", "collision-piece-membership.bin", "PIECES.json"):
        require((ORIGINAL / name).read_bytes() == generated[name], f"changed Saiga bundled collision input: {name}")
    points = list(struct.iter_unpack("<3d", generated["source.vertices"][16:]))
    triangles = list(struct.iter_unpack("<3I", generated["source-triangles.bin"]))
    membership = list(struct.iter_unpack("2B", generated["collision-piece-membership.bin"]))
    groups = {name: [] for name in json.loads(generated["PIECES.json"])["pieces"]}
    names = tuple(groups)
    require(len(points) == len(membership) == 3216 + 296 and len(names) == 72, "wrong Saiga support counts")
    slots = []
    for vertex, (primary, secondary) in enumerate(membership):
        require(primary < len(names), "invalid Saiga primary collision support")
        selected = {primary}
        if secondary != ABSENT_SECONDARY:
            require(primary < secondary < len(names), "invalid Saiga secondary collision support")
            first, _, a = names[primary].partition("-sector-")
            second, _, b = names[secondary].partition("-sector-")
            require(first == second and first.startswith(COLLAR) and abs(int(a) - int(b)) in (1, 7),
                    "Saiga overlap leaves adjacent sectors of a collar")
            selected.add(secondary)
        slots.append(selected)
        for slot in selected:
            groups[names[slot]].append(vertex)
    return points, triangles, slots, groups


def verify_routes(record, points, groups):
    require(record["pieces"] == list(groups), "changed Saiga route support order")
    vertices = [tuple(map(Fraction, point)) for point in points]
    clouds = [[vertices[index] for index in indices] for indices in groups.values()]
    result = {}
    for route in record["routes"]:
        triangle = [tuple(map(Fraction, point)) for point in route["triangle"]]
        travel = tuple(map(Fraction, route["travel"]))
        sweep = triangle + [tuple(a-b for a,b in zip(point, travel, strict=True)) for point in triangle]
        clearance = separating_planes(clouds, sweep, route["axes"])
        require(clearance == route["clearanceMillimetres"] and clearance > 100, "changed Saiga collar route")
        result[route["name"]] = clearance
    require(len(record["routes"]) == len(result) == 7, "wrong Saiga route count")
    return result


def prove_web_control(meshes):
    point = (.017, .037, .005)
    hits = {}
    for lod, (mesh, parts) in meshes.items():
        part = next(part for part in parts if part["name"] == "radial connected support")
        start, end = part["faceStart"], part["faceStart"] + part["faceCount"]
        source = mesh["vertices"]
        faces = [[tuple(map(Fraction, source[index])) for index in face] for face in mesh["faces"][start:end]]
        hits[lod] = inside_y(tuple(map(Fraction, point)), faces)
    return {"pointSourceUnits": point, "hitCountsAboveBelow": hits}


def verify():
    points, triangles, membership, groups = load_collision()
    collar_faces = {"detailed": 0, "candidateNative": 0}
    names = tuple(groups)
    require(len(triangles) == 5776 + 512, "wrong Saiga exported face count")
    for index, (a, b, c) in enumerate(triangles):
        coverage = membership[a] & membership[b] & membership[c]
        require(bool(coverage), "Saiga exported face crosses local collision supports")
        if any(names[slot].startswith(COLLAR) for slot in coverage):
            collar_faces["detailed" if index < 5776 else "candidateNative"] += 1
    require(collar_faces == {"detailed": 2304, "candidateNative": 128}, "wrong Saiga exported collar coverage")
    overlap = sum(len(slots) == 2 for slots in membership)
    require(overlap == 256, "wrong Saiga collar boundary vertex count")
    candidate_record = json.loads((INTEGRATION / "candidate-sector-support.json").read_bytes())
    actual = verify_routes(candidate_record, points, groups)

    source = tomllib.loads((ORIGINAL / "resource.toml").read_text())
    owners = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
    candidate = json.loads((ORIGINAL / "SH061-project-candidate.json").read_bytes())
    detailed = source["design"], owners["design"]["parts"]
    archived = source["nativeDesign"], owners["nativeDesign"]["parts"]
    reduced = candidate["nativeCandidate"], candidate["parts"]
    archives, meshes = assemble(detailed, archived)
    archive_groups = collision_groups(detailed, archived, archives, meshes, archived=True)
    archived_points = [
        [coordinate * float(SCALE) for coordinate in point]
        for mesh in meshes for point in mesh["vertices"]
    ]
    archive_record = json.loads((INTEGRATION / "archive-sector-support.json").read_bytes())
    archived_routes = verify_routes(archive_record, archived_points, archive_groups)
    require(archived_routes == actual, "Saiga candidate changes original collar clearances")
    return {
        "collisionSupportGroups": len(groups),
        "overlappingBoundaryVertices": overlap,
        "exportedFacesCovered": {"detailed": 5776, "candidateNative": 512},
        "exportedCollarFacesCovered": collar_faces,
        "archivedOriginalRoutesMillimetres": archived_routes,
        "exportedCandidateRoutesMillimetres": actual,
        "centralWebBlockedControl": prove_web_control({"detailed": detailed, "archivedNative": archived,
                                                        "candidateNative": reduced}),
    }


if __name__ == "__main__":
    sys.stdout.buffer.write(encode(verify()))
