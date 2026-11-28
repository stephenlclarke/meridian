"""Replay Boar Transport's calibrated original joins and inspection routes."""

from collections import Counter
from fractions import Fraction
import json
import sys

from prepare_boar_transport import INTEGRATION, ORIGINAL, SCALE, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes

OVERLAPS = {(0, 6),
 (0, 8),
 (0, 10),
 (0, 11),
 (0, 13),
 (0, 16),
 (3, 5),
 (3, 6),
 (3, 13),
 (4, 5),
 (4, 6),
 (4, 16),
 (5, 6),
 (5, 13),
 (5, 16),
 (6, 13),
 (6, 16),
 (7, 8),
 (9, 10),
 (11, 12),
 (13, 14),
 (16, 17)}


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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Boar Transport route clearance")
        require(route["name"] not in guarded, "duplicate Boar Transport route")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", "ventral", "port-outboard", "starboard-outboard",
                            "between-aft-drives", "over-port-cargo-beds", "over-starboard-cargo-beds"},
            "missing Boar Transport route")
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


def verify_candidate(original, candidate, parts_old, parts_new):
    record = json.loads((ORIGINAL / "GENERATED-COLLISION.json").read_bytes())
    selections = record["selectedSourceVertexIndices"]
    fixed = 0
    wells = 0
    for old, new in zip(parts_old, parts_new, strict=True):
        index = str(old["index"])
        before = section(original, old)
        after = section(candidate, new)
        require((old["name"], old["creatorMethod"]) == (new["name"], new["creatorMethod"]),
                "changed Boar Transport part order")
        require(set(before[2]) == set(after[2]), "changed Boar Transport part materials")
        for points, faces, _, _ in (before, after):
            edges = Counter((a,b) for x,y,z in faces for a,b in ((x,y),(y,z),(z,x)))
            require(len(points) - len(edges)//2 + len(faces) == 2
                    and all(count == edges[(b,a)] == 1 for (a,b),count in edges.items()),
                    "changed Boar Transport closed component")
        if old["creatorMethod"] == "Hull.loft / Hull.solid":
            ids = selections[index]
            require(sorted(set(ids)) == ids and 0 <= ids[0] <= ids[-1] < len(before[0])
                    and after[0] == [before[0][i] for i in ids],
                    "changed Boar Transport loft source selection")
        else:
            require(before == after, "changed Boar Transport fixed part")
            fixed += 1
            wells += old["creatorMethod"] == "Mesh.well"
    require(len(selections) == 6 and fixed == 13 and wells == 7, "wrong Boar Transport candidate inventory")
    return fixed, wells


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
                    "changed Boar Transport original overlap")
        require(pair not in contacts, "duplicate Boar Transport overlap")
        contacts.add(pair)
    require(contacts == OVERLAPS, "missing Boar Transport overlap")
    survey = json.loads((INTEGRATION / "CANDIDATE-TOPOLOGY-SURVEY.json").read_bytes())
    require(not survey["missingMeasuredOverlaps"] and not survey["additionalMeasuredOverlaps"],
            "changed Boar Transport overlap inventory")
    require({tuple(row["parts"]) for row in survey["candidateTopology"]["interiorOverlaps"]} == contacts,
            "inconsistent Boar Transport candidate survey")
    return sorted(contacts)


def verify():
    _, _, _, source_models, bindings = load_source()
    _, _, _, _, names, scaled = collision_inputs()
    streams = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == streams[name], f"changed SH128 proof input: {name}")
    fixed, wells = verify_candidate(*source_models[1:], *bindings[1:])
    contacts = verify_contacts(*(pair[0] for pair in scaled[1:]), *bindings[1:])
    clearance = verify_routes(names, scaled)
    return {"clearanceMillimetres": clearance, "originalOverlaps": contacts,
            "unchangedNativeParts": fixed, "unchangedNativeWells": wells}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
