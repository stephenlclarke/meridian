"""Replay Buzzard Mk II's calibrated original joins and inspection routes."""

from collections import Counter
from fractions import Fraction
import json
import sys

from prepare_buzzard_mk_ii import INTEGRATION, ORIGINAL, SCALE, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes

OVERLAPS = {(1, 2),
 (1, 7),
 (1, 8),
 (1, 15),
 (1, 16),
 (1, 17),
 (1, 18),
 (7, 8),
 (9, 10),
 (12, 13),
 (15, 16),
 (17, 18)}


def verify_routes(names, scaled):
    pieces = [[] for _ in names]
    for model, membership in scaled:
        for vertex, slots in zip(model["vertices"], membership, strict=True):
            point = tuple(map(Fraction, vertex))
            for slot in slots:
                pieces[slot].append(point)
    guarded = {}
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        triangle = [tuple(map(Fraction, p)) for p in route["triangle"]]
        travel = tuple(map(Fraction, route["travel"]))
        swept = triangle + [tuple(a - b for a, b in zip(p, travel, strict=True)) for p in triangle]
        distance = separating_planes(pieces, swept, route["axes"])
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Buzzard Mk II route clearance")
        require(route["name"] not in guarded, "duplicate Buzzard Mk II route")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", "ventral", "port-outboard", "starboard-outboard",
                            "through-aft-hoop", "port-through-hoops", "starboard-through-hoops"},
            "missing Buzzard Mk II route")
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
    recipe = json.loads((INTEGRATION / "ARCHIVED-RECIPE-PROVENANCE.json").read_bytes())
    generated = set(recipe["sampling"]["changedOriginalPartIndices"])
    fixed = wells = hoops = 0
    for index, (old, new) in enumerate(zip(parts_old, parts_new, strict=True)):
        before, after = section(original, old), section(candidate, new)
        require(old["index"] == new["index"] == index and old["name"] == new["name"],
                "changed Buzzard Mk II creator part order")
        require(set(before[2]) == set(after[2]) == set(new["originalMaterialIndices"]),
                "changed Buzzard Mk II native part material indices")
        expected_euler = 0 if index < 2 else 2
        for points, faces, _, _ in (before, after):
            directed = Counter((a, b) for x, y, z in faces for a, b in ((x, y), (y, z), (z, x)))
            require(len(points) - len(directed) // 2 + len(faces) == expected_euler and
                    all(count == directed[(b, a)] == 1 for (a, b), count in directed.items()),
                    "changed Buzzard Mk II closed component topology")
        require((before != after) == (index in generated), "changed Buzzard Mk II recipe part inventory")
        fixed += before == after
        wells += old["creatorMethod"] == "Mesh.well"
        hoops += expected_euler == 0
    require((fixed, wells, hoops) == (11, 4, 2), "wrong Buzzard Mk II candidate inventory")
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
                    "changed Buzzard Mk II original overlap")
        require(pair not in contacts, "duplicate Buzzard Mk II overlap")
        contacts.add(pair)
    require(contacts == OVERLAPS, "missing Buzzard Mk II overlap")
    survey = json.loads((INTEGRATION / "CANDIDATE-TOPOLOGY-SURVEY.json").read_bytes())
    require(not survey["missingMeasuredOverlaps"] and not survey["additionalMeasuredOverlaps"],
            "changed Buzzard Mk II overlap inventory")
    require({tuple(row["parts"]) for row in survey["candidateTopology"]["interiorOverlaps"]} == contacts,
            "inconsistent Buzzard Mk II candidate survey")
    return sorted(contacts)


def verify():
    _, _, _, source_models, bindings = load_source()
    _, _, _, _, names, scaled = collision_inputs()
    streams = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == streams[name], f"changed SH142 proof input: {name}")
    fixed, wells = verify_candidate(*source_models[1:], *bindings[1:])
    contacts = verify_contacts(*(pair[0] for pair in scaled[1:]), *bindings[1:])
    clearance = verify_routes(names, scaled)
    return {"clearanceMillimetres": clearance, "originalOverlaps": contacts,
            "unchangedNativeParts": fixed, "unchangedNativeWells": wells}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
