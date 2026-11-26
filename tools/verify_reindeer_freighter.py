"""Replay Reindeer Freighter's calibrated original joins and inspection routes."""

from collections import Counter
from fractions import Fraction
import json
import sys

from prepare_reindeer_freighter import INTEGRATION, ORIGINAL, SCALE, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes

OVERLAPS = {(0, 5),
 (0, 8),
 (0, 11),
 (0, 14),
 (0, 17),
 (0, 20),
 (0, 23),
 (0, 26),
 (0, 29),
 (0, 32),
 (0, 35),
 (0, 38),
 (0, 41),
 (0, 43),
 (0, 46),
 (3, 5),
 (3, 11),
 (3, 43),
 (4, 8),
 (4, 14),
 (4, 46),
 (5, 6),
 (5, 7),
 (5, 8),
 (5, 43),
 (6, 7),
 (6, 43),
 (7, 43),
 (8, 9),
 (8, 10),
 (8, 46),
 (9, 10),
 (9, 46),
 (10, 46),
 (11, 12),
 (11, 13),
 (11, 14),
 (12, 13),
 (14, 15),
 (14, 16),
 (15, 16),
 (17, 18),
 (17, 19),
 (17, 20),
 (18, 19),
 (20, 21),
 (20, 22),
 (21, 22),
 (23, 24),
 (23, 25),
 (23, 26),
 (24, 25),
 (26, 27),
 (26, 28),
 (27, 28),
 (29, 30),
 (29, 31),
 (29, 32),
 (30, 31),
 (32, 33),
 (32, 34),
 (33, 34),
 (35, 36),
 (35, 37),
 (35, 38),
 (36, 37),
 (38, 39),
 (38, 40),
 (39, 40),
 (41, 42),
 (43, 44),
 (46, 47)}


def verify_routes(names, scaled):
    pieces = [[] for _ in names]
    for model, membership in scaled:
        for vertex, (slot,) in zip(model["vertices"], membership, strict=True):
            pieces[slot].append(tuple(map(Fraction, vertex)))
    guarded = {}
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        triangle = [tuple(map(Fraction, p)) for p in route["triangle"]]
        travel = tuple(map(Fraction, route["travel"]))
        swept = triangle + [tuple(a - b for a, b in zip(p, travel, strict=True)) for p in triangle]
        distance = separating_planes(pieces, swept, route["axes"])
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Reindeer Freighter route clearance")
        require(route["name"] not in guarded, "duplicate Reindeer Freighter route")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", "ventral", "port-outboard", "starboard-outboard",
                            "below-pressure-envelope", "above-port-panniers", "above-starboard-panniers"},
            "missing Reindeer Freighter route")
    return guarded


def section(model, owner):
    start = owner["vertexStart"]
    faces = slice(owner["faceStart"], owner["faceStart"] + owner["faceCount"])
    return (
        model["vertices"][start:start + owner["vertexCount"]],
        [[i - start for i in face] for face in model["faces"][faces]],
        model["faceMaterials"][faces],
        model["faceTextures"][faces],
    )


def strictly_inside_tetra(points):
    a, b, c, d, inner = [tuple(map(Fraction, p)) for p in points]

    def determinant(u, v, w):
        return u[0] * (v[1]*w[2] - v[2]*w[1]) - u[1] * (v[0]*w[2] - v[2]*w[0]) + u[2] * (v[0]*w[1] - v[1]*w[0])

    def relative(point):
        return tuple(x-y for x, y in zip(point, a, strict=True))

    u, v, w, x = map(relative, (b, c, d, inner))
    volume = determinant(u, v, w)
    if not volume:
        return False
    weights = (determinant(x, v, w) / volume, determinant(u, x, w) / volume,
               determinant(u, v, x) / volume)
    return all(weight > 0 for weight in weights) and sum(weights) < 1


def verify_extended_well(before, after, ids):
    require(len(ids) == len(set(ids)) == 7 and ids[:4] == [0, 1, 2, 3]
            and all(8 <= idx < 12 for idx in ids[4:6]) and 4 <= ids[6] < 8,
            "wrong four-corner Reindeer well source selection")
    vertices, faces, _, _ = after
    require(len(faces) == 10 and all(triangle[2] == 6 for triangle in faces[:4])
            and {tuple(sorted(triangle[:2])) for triangle in faces[:4]} == {(0, 1), (1, 2), (2, 3), (0, 3)},
            "wrong recessed opening or backing topology")
    points = [tuple(map(Fraction, point)) for point in vertices]

    def side_of_plane(face, index):
        a, b, c = (points[idx] for idx in face)
        u, v, w = [tuple(x-y for x, y in zip(point, a, strict=True)) for point in (b, c, points[index])]
        return (u[0] * (v[1]*w[2] - v[2]*w[1]) - u[1] * (v[0]*w[2] - v[2]*w[0])
                + u[2] * (v[0]*w[1] - v[1]*w[0]))

    require(all(side_of_plane(face, 6) < 0 for face in faces[4:])
            and all(side_of_plane((0, 1, 2), idx) * side_of_plane((0, 1, 2), 6) > 0 for idx in (4, 5)),
            "recess leaves its original opening or backing")
    require(vertices == [before[0][idx] for idx in ids], "changed Reindeer well source vertices")


def verify_candidate(original, candidate, parts_old, parts_new):
    record = json.loads((ORIGINAL / "GENERATED-COLLISION.json").read_bytes())
    selections = record["selectedSourceVertexIndices"]
    wells = record["recessedWellSelections"]
    fixed = 0
    for old, new in zip(parts_old, parts_new, strict=True):
        index = str(old["index"])
        before = section(original, old)
        after = section(candidate, new)
        require((old["name"], old["creatorMethod"]) == (new["name"], new["creatorMethod"]),
                "changed Reindeer Freighter part order")
        require(set(before[2]) == set(after[2]), "changed Reindeer Freighter part materials")
        for points, faces, _, _ in (before, after):
            edges = Counter((a,b) for x,y,z in faces for a,b in ((x,y),(y,z),(z,x)))
            require(len(points) - len(edges)//2 + len(faces) == 2
                    and all(count == edges[(b,a)] == 1 for (a,b),count in edges.items()),
                    "changed Reindeer Freighter closed component")
        if old["name"] == "box_detail structural envelope":
            require(index not in selections and index not in wells and before == after,
                    "Reindeer cargo-box envelope must be exact")
        if index in wells:
            ids = wells[index]
            rim_faces = 3 if len(ids) == 5 else 4
            if rim_faces == 3:
                require(len(ids) == len(set(ids)) == 5 and all(0 <= idx < 4 for idx in ids[:3])
                        and 8 <= ids[3] < 12 and 4 <= ids[4] < 8 and len(after[1]) == 6
                        and after[0] == [before[0][idx] for idx in ids] and strictly_inside_tetra(after[0]),
                        "changed Reindeer six-face well source or backing")
            else:
                verify_extended_well(before, after, ids)
            require(all(material == before[2][16] for material in after[2][:rim_faces])
                    and all(material == before[2][0] for material in after[2][rim_faces:]),
                    "Reindeer well lost its recessed material")
        elif index in selections:
            ids = selections[index]
            require(sorted(set(ids)) == ids and 0 <= ids[0] <= ids[-1] < len(before[0])
                    and after[0] == [before[0][i] for i in ids],
                    "changed Reindeer Freighter loft source selection")
        else:
            require(before == after, "changed Reindeer Freighter fixed support")
            fixed += 1
    require(len(wells) == 16 and len(selections) == 21 and fixed == 12,
            "wrong Reindeer Freighter part inventory")
    return fixed, len(wells)


def verify_contacts(original, candidate, parts_old, parts_new):
    original_parts, candidate_parts = [
        [[[tuple(map(Fraction, model["vertices"][i])) for i in face]
          for face in model["faces"][part["faceStart"]:part["faceStart"] + part["faceCount"]]] for part in parts]
        for model, parts in ((original, parts_old), (candidate, parts_new))
    ]
    contacts = set()
    for row in json.loads((INTEGRATION / "RETAINED-OVERLAPS.json").read_bytes()):
        pair = tuple(row["parts"])
        point = tuple(map(Fraction, row["pointMetres"]))
        for faces, key in ((original_parts, "hitCountsAboveBelow"), (candidate_parts, "candidateHitCountsAboveBelow")):
            hit_counts = [list(inside_y(point, faces[i])) for i in pair]
            require(hit_counts == row[key] and all(n % 2 for hit in hit_counts for n in hit),
                    "changed Reindeer Freighter original overlap")
        require(pair not in contacts, "duplicate Reindeer Freighter overlap")
        contacts.add(pair)
    require(contacts == OVERLAPS, "missing Reindeer Freighter overlap")
    survey = json.loads((INTEGRATION / "CANDIDATE-TOPOLOGY-SURVEY.json").read_bytes())
    require(not survey["missingMeasuredOverlaps"] and not survey["additionalMeasuredOverlaps"],
            "changed Reindeer Freighter overlap inventory")
    require({tuple(row["parts"]) for row in survey["candidateTopology"]["interiorOverlaps"]} == contacts,
            "inconsistent Reindeer Freighter candidate survey")
    return sorted(contacts)


def verify():
    _, _, _, source_models, bindings = load_source()
    _, _, _, _, names, scaled = collision_inputs()
    streams = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == streams[name], f"changed SH150 proof input: {name}")
    fixed, wells = verify_candidate(*source_models[1:], *bindings[1:])
    contacts = verify_contacts(*(pair[0] for pair in scaled[1:]), *bindings[1:])
    clearance = verify_routes(names, scaled)
    return {"clearanceMillimetres": clearance, "originalOverlaps": contacts,
            "unchangedNativeParts": fixed, "recessedNativeWells": wells}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
