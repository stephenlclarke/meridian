"""Replay exact Wombat routes on the bundled support stream and source joints."""

from fractions import Fraction
import json
import struct
import sys
import tomllib

from prepare_wombat_geometry import INTEGRATION, ORIGINAL, SCALE, assemble, build, encode
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def require(condition, message):
    if not condition:
        raise ValueError(message)


def grouped_points(groups, points, count):
    pieces = [[] for _ in range(count)]
    for slot, point in zip(groups, points, strict=True):
        pieces[slot].append(tuple(map(Fraction, point)))
    require(all(pieces), "empty Wombat inspection support")
    return pieces


def verify_routes(record, pieces, names):
    require(record["scale"] == [SCALE.numerator, SCALE.denominator], "changed Wombat route units")
    require(record["groups"] == names, "changed Wombat route support order")
    observed = {}
    for route in record["routes"]:
        triangle = [tuple(map(Fraction, point)) for point in route["triangle"]]
        travel = tuple(map(Fraction, route["travel"]))
        sweep = triangle + [tuple(a - b for a, b in zip(point, travel, strict=True)) for point in triangle]
        clearance = separating_planes(pieces, sweep, route["axes"])
        require(clearance == route["clearanceMillimetres"] and clearance > 100, "changed Wombat shape route")
        observed[route["name"]] = clearance
    require(len(observed) == len(record["routes"]) == 7, "wrong Wombat route count")
    return observed


def prove_joints(source_meshes):
    fixtures = [("cockpit-crossmember-to-glazing", (.021, .479, -.291),
                 "cockpit connected structural crossmember", "cockpit glazing envelope")]
    for side, sign, lateral in (("port", -1, 0), ("starboard", 1, 1)):
        for edge, y, transverse in (("lower", -.463, 0), ("upper", .103, 1)):
            fixtures.append((f"{side}-{edge}-frame-corner", (sign * .947, y, .581),
                             f"dock lateral structural member {lateral}", f"dock transverse structural member {transverse}"))
    joints = {}
    for name, point, first, second in fixtures:
        exact = tuple(map(Fraction, point))
        results = {}
        for lod, (mesh, parts) in source_meshes.items():
            spans = {part["name"]: part for part in parts if part["name"] in (first, second)}
            require(set(spans) == {first, second}, "missing Wombat joint part")
            results[lod] = {}
            for part, span in spans.items():
                upper = span["faceStart"] + span["faceCount"]
                faces = mesh["faces"][span["faceStart"]:upper]
                vertices = mesh["vertices"]
                exact_faces = [[tuple(map(Fraction, vertices[i])) for i in face] for face in faces]
                results[lod][part] = inside_y(exact, exact_faces)
        joints[name] = {"pointSourceUnits": point, "hitCountsAboveBelow": results}
    return joints


def verify():
    generated = build()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin", "PIECES.json"):
        require((ORIGINAL / name).read_bytes() == generated[name], f"changed Wombat route input {name}")

    points = list(struct.iter_unpack("<3d", generated["source.vertices"][16:]))
    membership = generated["source-piece-membership.bin"]
    faces = list(struct.iter_unpack("<3I", generated["source-triangles.bin"]))
    require(len(faces) == 3544 + 496 and all(membership[a] == membership[b] == membership[c] for a, b, c in faces),
            "Wombat source triangle crosses local supports")
    names = [f"{i:03d} {piece['name']}[{piece['occurrence']}]"
             for i, piece in enumerate(json.loads(generated["PIECES.json"])["pieces"])]
    actual_pieces = grouped_points(membership, points, len(names))
    candidate_routes = json.loads((INTEGRATION / "candidate-support.json").read_bytes())
    actual = verify_routes(candidate_routes, actual_pieces, names)

    source = tomllib.loads((ORIGINAL / "resource.toml").read_text())
    owners = json.loads((INTEGRATION / "ORIGINAL-OWNERSHIP.json").read_bytes())["models"]
    candidate = json.loads((ORIGINAL / "SH060-project-candidate.json").read_bytes())
    detailed = source["design"], owners["design"]["parts"]
    archive = source["nativeDesign"], owners["nativeDesign"]["parts"]
    reduced = candidate["nativeCandidate"], candidate["parts"]
    original_slots, original_meshes = assemble(detailed, archive)
    original_names = [f"{i:03d} {name}[{ordinal}]" for (name, ordinal), i in original_slots.items()]
    original_pieces = grouped_points(
        [group for mesh in original_meshes for group in mesh["groups"]],
        [[value * float(SCALE) for value in point] for mesh in original_meshes for point in mesh["vertices"]],
        len(original_slots),
    )
    original_routes = json.loads((INTEGRATION / "ARCHIVE-SUPPORT-PREFLIGHT.json").read_bytes())
    archived = verify_routes(original_routes, original_pieces, original_names)
    require(actual == archived, "Wombat candidate changes original guarded clearances")
    return {
        "detailedWithArchivedNative": archived,
        "detailedWithCandidateNative": actual,
        "bundledFaceCoverage": {"detailed": 3544, "candidate": 496},
        "sourceJoints": prove_joints({"detailed": detailed, "archivedNative": archive, "candidateNative": reduced}),
    }


if __name__ == "__main__":
    sys.stdout.buffer.write(encode(verify()))
