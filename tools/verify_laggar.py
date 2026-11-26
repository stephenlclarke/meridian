"""Replay the exact original SH085 clearance and joint proofs."""

from fractions import Fraction
import json
import struct
import sys

from prepare_laggar import INTEGRATION, ORIGINAL, encode_json, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def prove_fin_separation(vertices, membership, names, detail_vertices):
    result = {}
    for lod, start, end in (("detailed", 0, detail_vertices), ("native", detail_vertices, len(vertices))):
        pieces = [[] for _ in names]
        for slot, vertex in zip(membership[start:end], vertices[start:end], strict=True):
            pieces[slot].append(vertex)
        for side, sign in (("port", -1), ("starboard", 1)):
            def points(name):
                return pieces[names.index(name)]
            fin = points(f"{side}-dorsal-fin")
            arm = points(f"{side}-outrigger-arm")
            pod = points(f"{side}-pod")
            gaps = {
                "aboveArm": min(p[1] for p in fin) - max(p[1] for p in arm),
                "inboardOfPod": min(sign * p[0] for p in pod) - max(sign * p[0] for p in fin),
                "outboardOfHull": min(sign * p[0] for p in fin) - max(sign * p[0] for p in points("central-hull")),
            }
            millimetres = {name: gap * 1000 // 1 for name, gap in gaps.items()}
            require(millimetres == {"aboveArm": 799, "inboardOfPod": 849, "outboardOfHull": 1149},
                    "changed original Laggar fin separations")
            result[f"{lod}-{side}"] = millimetres
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
                        "changed Laggar original joint")
        checked.append(joint["name"])
    require(len(set(checked)) == len(checked), "duplicate Laggar original joint")
    return checked


def verify():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH085 proof input: {name}")

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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Laggar sweep")
        clearances[route["name"]] = distance
    require(len(clearances) == len(routes) == 7, "wrong Laggar route count")

    faces = list(struct.iter_unpack("<3I", derived["source-triangles.bin"]))
    geometry = (vertices, faces, membership, names, detail_vertices, detail_faces)
    shared_source = json.loads((INTEGRATION / "ORIGINAL-JOINTS.json").read_bytes())
    shared = prove_joints(shared_source, {"detailed", "native"}, *geometry)
    expected = {"canopy-to-hull", *(f"{side}-{joint}" for side in ("port", "starboard")
                for joint in ("arm-to-hull", "arm-to-pod", "nozzle-to-pod"))}
    require(set(shared) == expected, "changed Laggar original shared overlaps")
    detail_source = json.loads((INTEGRATION / "DETAILED-ONLY-JOINTS.json").read_bytes())
    detail = prove_joints(detail_source, {"detailed"}, *geometry)
    require(detail == ["canopy-ridge-to-canopy"], "changed Laggar detailed overlap")
    require(names.index("canopy-ridge") not in membership[detail_vertices:], "Laggar native gained a ridge")
    gaps = prove_fin_separation(vertices, membership, names, detail_vertices)
    return {"clearanceMillimetres": clearances, "sourceJointNames": shared, "detailedOnlyJointNames": detail,
            "originalFinSeparations": gaps}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
