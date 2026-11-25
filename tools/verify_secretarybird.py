"""Replay Secretarybird's calibrated original joins and inspection routes."""

from collections import Counter
from fractions import Fraction
import json
import sys

from prepare_secretarybird import INTEGRATION, ORIGINAL, SCALE, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes

OVERLAPS = {(0, 5),
 (0, 6),
 (0, 9),
 (0, 11),
 (0, 14),
 (1, 6),
 (1, 14),
 (2, 5),
 (2, 11),
 (5, 11),
 (6, 14),
 (7, 17),
 (7, 18),
 (8, 19),
 (8, 20),
 (9, 10),
 (11, 12),
 (14, 15),
 (17, 18),
 (19, 20)}


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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Secretarybird route clearance")
        require(route["name"] not in guarded, "duplicate Secretarybird route")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", "ventral", "port-outboard", "starboard-outboard",
                            "under-pressure-envelope", "over-port-wing", "over-starboard-wing"},
            "missing Secretarybird route")
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
                "changed Secretarybird part order")
        require(set(before[2]) == set(after[2]), "changed Secretarybird part materials")
        for points, faces, _, _ in (before, after):
            edges = Counter((a,b) for x,y,z in faces for a,b in ((x,y),(y,z),(z,x)))
            require(len(points) - len(edges)//2 + len(faces) == 2
                    and all(count == edges[(b,a)] == 1 for (a,b),count in edges.items()),
                    "changed Secretarybird closed component")
        if index in wells:
            ids = wells[index]
            require(len(ids) == len(set(ids)) == 5 and all(0 <= i < 4 for i in ids[:3])
                    and 8 <= ids[3] < 12 and 4 <= ids[4] < 8 and len(after[1]) == 6,
                    "changed Secretarybird well source selection")
            require(after[0] == [before[0][i] for i in ids] and strictly_inside_tetra(after[0]),
                    "Secretarybird recessed vertex or backing changed")
            require(len(set(after[2][:3])) == 1 and after[2][0] == before[2][16]
                    and all(material == before[2][0] for material in after[2][3:]),
                    "Secretarybird well lost its recessed material")
        elif index in selections:
            ids = selections[index]
            require(sorted(set(ids)) == ids and 0 <= ids[0] <= ids[-1] < len(before[0])
                    and after[0] == [before[0][i] for i in ids],
                    "changed Secretarybird loft source selection")
        else:
            require(before == after, "changed Secretarybird fixed support")
            fixed += 1
    require(len(wells) == 2 and len(selections) == 6 and fixed == 13,
            "wrong Secretarybird part inventory")
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
                    "changed Secretarybird original overlap")
        require(pair not in contacts, "duplicate Secretarybird overlap")
        contacts.add(pair)
    require(contacts == OVERLAPS, "missing Secretarybird overlap")
    survey = json.loads((INTEGRATION / "CANDIDATE-TOPOLOGY-SURVEY.json").read_bytes())
    require(not survey["missingMeasuredOverlaps"] and not survey["additionalMeasuredOverlaps"],
            "changed Secretarybird overlap inventory")
    require({tuple(row["parts"]) for row in survey["candidateTopology"]["interiorOverlaps"]} == contacts,
            "inconsistent Secretarybird candidate survey")
    return sorted(contacts)


def verify():
    _, _, _, source_models, bindings = load_source()
    _, _, _, _, names, scaled = collision_inputs()
    streams = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == streams[name], f"changed SH165 proof input: {name}")
    fixed, wells = verify_candidate(*source_models[1:], *bindings[1:])
    contacts = verify_contacts(*(pair[0] for pair in scaled[1:]), *bindings[1:])
    clearance = verify_routes(names, scaled)
    return {"clearanceMillimetres": clearance, "originalOverlaps": contacts,
            "unchangedNativeParts": fixed, "recessedNativeWells": wells}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
