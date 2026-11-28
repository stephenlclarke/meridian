"""Replay Shikra Interdictor's calibrated original joins and inspection routes."""

from fractions import Fraction
import json
import sys

from prepare_shikra_interdictor import INTEGRATION, ORIGINAL, SCALE, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes

OVERLAPS = {(0, 5), (0, 11), (0, 17), (0, 23), (0, 29), (0, 31), (0, 34), (1, 34), (2, 31),
            (5, 6), (11, 12), (17, 18), (23, 24), (29, 30), (31, 32), (34, 35), (37, 38), (39, 40)}


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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Shikra Interdictor route clearance")
        require(route["name"] not in guarded, "duplicate Shikra Interdictor route")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", "ventral", "between-aft-drives",
                            *(f"{side}-{suffix}" for side in ("port", "starboard")
                              for suffix in ("outboard", "between-diagonal-spars"))}, "missing Shikra Interdictor route")
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
    nominee = json.loads((ORIGINAL / "GENERATED-COLLISION.json").read_bytes())
    selections = nominee["selectedSourceVertexIndices"]
    source_parts = [section(original, binding) for binding in parts_old]
    derived_parts = [section(candidate, binding) for binding in parts_new]
    fixed = 0
    for index, (original_part, derived_part) in enumerate(zip(source_parts, derived_parts, strict=True)):
        frozen_vertices, _, frozen_materials, _ = original_part
        candidate_vertices, _, candidate_materials, _ = derived_part
        require(parts_old[index]["name"] == parts_new[index]["name"], "changed Shikra Interdictor part order")
        require(set(frozen_materials) == set(candidate_materials), "changed Shikra Interdictor part material inventory")
        if str(index) in selections:
            indices = selections[str(index)]
            require(sorted(set(indices)) == indices and 0 <= indices[0] <= indices[-1] < len(frozen_vertices),
                    "changed Shikra Interdictor source vertex selection")
            require([frozen_vertices[i] for i in indices] == candidate_vertices, "candidate vertex lacks Shikra Interdictor provenance")
        else:
            require(original_part == derived_part, "changed fixed Shikra Interdictor part")
            fixed += 1
    require(fixed == 11 and len(selections) == 30, "changed Shikra Interdictor candidate inventory")
    for first, second in zip(zip(*original["vertices"]), zip(*candidate["vertices"]), strict=True):
        require((min(first), max(first)) == (min(second), max(second)), "changed Shikra Interdictor native extrema")
    for a,b in ((32, 33), (35, 36)):
        for parts in (source_parts, derived_parts):
            gap = (Fraction(min(v[2] for v in parts[a][0])) -
                   Fraction(max(v[2] for v in parts[b][0]))) * SCALE
            require(int(gap * 1000) == 46, "changed Shikra Interdictor casing-to-recess gap")
    return fixed


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
                    "changed Shikra Interdictor original overlap")
        require(pair not in contacts, "duplicate Shikra Interdictor overlap")
        contacts.add(pair)
    require(contacts == OVERLAPS, "missing Shikra Interdictor overlap")
    survey = json.loads((INTEGRATION / "CANDIDATE-TOPOLOGY-SURVEY.json").read_bytes())
    require(not survey["missingMeasuredOverlaps"] and not survey["additionalMeasuredOverlaps"],
            "changed Shikra Interdictor overlap inventory")
    require({tuple(row["parts"]) for row in survey["candidateTopology"]["interiorOverlaps"]} == contacts,
            "inconsistent Shikra Interdictor candidate survey")
    return sorted(contacts)


def verify():
    _, _, _, source_models, bindings = load_source()
    _, _, _, _, names, scaled = collision_inputs()
    streams = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == streams[name], f"changed SH095 proof input: {name}")
    fixed = verify_candidate(*source_models[1:], *bindings[1:])
    contacts = verify_contacts(*(pair[0] for pair in scaled[1:]), *bindings[1:])
    clearance = verify_routes(names, scaled)
    return {"clearanceMillimetres": clearance, "originalOverlaps": contacts,
            "unchangedNativeParts": fixed, "casingRecessLongitudinalGapMillimetres": 46}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
