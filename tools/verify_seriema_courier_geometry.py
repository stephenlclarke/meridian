"""Replay Seriema Courier collision routes on the exported metre-space geometry."""

from fractions import Fraction
import json
import struct
import sys
import tomllib

from banteng_art_mesh import assemble
from prepare_seriema_courier_geometry import INTEGRATION, ORIGINAL, SCALE, build, encode, metre_space, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def verify_routes(record, slots, points, membership):
    require(record["sourceScaleRatio"] == [SCALE.numerator, SCALE.denominator]
            and record["coordinateUnits"] == "metres", "changed Seriema Courier route calibration")
    labels = [f"{i:03d} {name}[{ordinal}]" for (name, ordinal), i in slots.items()]
    require(record["groups"] == labels, "changed Seriema Courier route support order")
    groups = [[] for _ in labels]
    for group, point in zip(membership, points, strict=True):
        groups[group].append(tuple(map(Fraction, point)))
    require(all(groups), "empty Seriema Courier support")
    result = {}
    for route in record["routes"]:
        triangle = [tuple(map(Fraction, point)) for point in route["triangle"]]
        travel = tuple(map(Fraction, route["travel"]))
        sweep = triangle + [tuple(a - b for a, b in zip(point, travel, strict=True)) for point in triangle]
        clearance = separating_planes(groups, sweep, route["axes"])
        require(clearance == route["clearanceMillimetres"] and clearance > 100, "changed Seriema Courier guarded route")
        result[route["name"]] = clearance
    require(len(record["routes"]) == len(result) == 7, "wrong Seriema Courier route count")
    return result


def prove_joints(configurations):
    fixtures = {
        "blade-to-main-body": ((.09, -1.119, .871), ("vertical blade rail", 1),
                               ("body primary pressure envelope", 1)),
        "cockpit-crossmember-to-glazing": ((.193, 2.071, 5.923),
                                           ("cockpit connected structural crossmember", 1),
                                           ("cockpit glazing envelope", 1)),
    }
    for side, sign, ordinal in (("port", -1, 1), ("starboard", 1, 2)):
        fixtures[f"{side}-forward-exterior-bracket"] = (
            (sign * 1.833, -2.806, 6.579), ("explicit military mount upright", ordinal),
            ("explicit military mount longitudinal member", ordinal),
        )
    results = {}
    for name, (coords, *part_names) in fixtures.items():
        point = tuple(map(Fraction, coords))
        locators = {}
        for view, (mesh, parts) in configurations.items():
            locators[view] = []
            for part_name, ordinal in part_names:
                original = [part for part in parts if part["name"] == part_name][ordinal - 1]
                start, end = original["faceStart"], original["faceStart"] + original["faceCount"]
                vertices = mesh["vertices"]
                faces = [[tuple(map(Fraction, vertices[index])) for index in face] for face in mesh["faces"][start:end]]
                locators[view].append(inside_y(point, faces))
        results[name] = {"pointMetres": coords, "parts": part_names, "hitCountsAboveBelow": locators}
    return results


def verify():
    generated = build()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin", "PIECES.json"):
        require((ORIGINAL / name).read_bytes() == generated[name], f"changed Seriema Courier collision input: {name}")
    points = list(struct.iter_unpack("<3d", generated["source.vertices"][16:]))
    faces = list(struct.iter_unpack("<3I", generated["source-triangles.bin"]))
    membership = generated["source-piece-membership.bin"]
    require(len(faces) == 1656 + 512 and all(membership[a] == membership[b] == membership[c] for a, b, c in faces),
            "Seriema Courier face crosses a support group")

    raw = tomllib.loads((ORIGINAL / "resource.toml").read_text())
    source = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
    collision = json.loads((ORIGINAL / "CREATOR-CANDIDATE.json").read_bytes())
    detail = metre_space(raw["design"]), source["design"]["parts"]
    native = metre_space(raw["nativeDesign"]), source["nativeDesign"]["parts"]
    candidate = collision["nativeDesign"], collision["parts"]
    slots, _ = assemble(detail, candidate)
    actual = verify_routes(json.loads((INTEGRATION / "candidate-support.json").read_bytes()),
                           slots, points, membership)
    archived_slots, archived_meshes = assemble(detail, native)
    archived = verify_routes(
        json.loads((INTEGRATION / "ARCHIVE-SUPPORT-PREFLIGHT.json").read_bytes()), archived_slots,
        [point for model in archived_meshes for point in model["vertices"]],
        [group for model in archived_meshes for group in model["groups"]],
    )
    require(archived == actual, "Seriema Courier archived collision changes a guarded route")
    return {
        "archivedVisibleNativeRoutesMillimetres": archived,
        "exportedCollisionRoutesMillimetres": actual,
        "exportedFacesCovered": {"detailed": 1656, "collision": 512},
        "sourceJoints": prove_joints({"detailed": detail, "archivedVisibleNative": native,
                                     "archivedCollision": candidate}),
    }


if __name__ == "__main__":
    sys.stdout.buffer.write(encode(verify()))
