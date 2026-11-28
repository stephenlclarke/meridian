"""Replay Anoa's calibrated original joins and inspection routes."""

from collections import Counter
from fractions import Fraction
import json
import sys

from prepare_anoa import INTEGRATION, ORIGINAL, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes

OVERLAPS = {
    (0, 5), (0, 6), (0, 7), (0, 18), (0, 20), (0, 24),
    (0, 25), (0, 26), (0, 27), (3, 6), (4, 18), (5, 11),
    (5, 23), (6, 11), (6, 12), (6, 14), (6, 24), (6, 25),
    (6, 28), (6, 29), (6, 30), (10, 12), (11, 17), (12, 17),
    (12, 18), (12, 21), (12, 24), (12, 25), (13, 18), (17, 23),
    (18, 23), (18, 24), (18, 25), (18, 31), (18, 32), (18, 33),
    (25, 26), (26, 27), (28, 29), (31, 32),
}
COLLARS = (24, 25)

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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Anoa route clearance")
        require(route["name"] not in guarded, "duplicate Anoa route")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", "ventral", "port-outboard", "starboard-outboard",
                            "through-aft-collar", "through-forward-collar", "upper-diagonal"},
            "missing Anoa route")
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


def collar_indices(nominee):
    record = nominee["collars"]
    major, minor = record["majorIndices"], record["minorIndices"]
    require(major == list(range(0, 24, 3)) and minor == [0, 3, 5], "changed Anoa collar source sampling")
    return [m * 8 + n for m in major for n in minor]


def verify_topology(section_data, collar):
    points, faces, _, _ = section_data
    edges = Counter((a, b) for x, y, z in faces for a, b in ((x, y), (y, z), (z, x)))
    require(all(count == edges[(b, a)] == 1 for (a, b), count in edges.items()),
            "open or inconsistent Anoa part")
    require(len(points) - len(edges) // 2 + len(faces) == (0 if collar else 2),
            "changed Anoa genus")


def verify_candidate(original, candidate, parts_old, parts_new):
    nominee = json.loads((ORIGINAL / "GENERATED-COLLISION.json").read_bytes())
    selections = nominee["selectedSourceVertexIndices"]
    collar_selection = collar_indices(nominee)
    source_parts = [section(original, binding) for binding in parts_old]
    derived_parts = [section(candidate, binding) for binding in parts_new]
    fixed = 0
    for index, (original_part, derived_part) in enumerate(zip(source_parts, derived_parts, strict=True)):
        frozen_vertices, _, frozen_materials, _ = original_part
        candidate_vertices, _, candidate_materials, _ = derived_part
        require(parts_old[index]["name"] == parts_new[index]["name"], "changed Anoa part order")
        require(set(frozen_materials) == set(candidate_materials), "changed Anoa part material inventory")
        if index in COLLARS:
            require(len(original_part[1]) == 384 and len(derived_part[1]) == 48,
                    "changed Anoa original or candidate collar faces")
            require([frozen_vertices[i] for i in collar_selection] == candidate_vertices,
                    "changed Anoa original collar vertex selection")
        elif str(index) in selections:
            indices = selections[str(index)]
            require(sorted(set(indices)) == indices and 0 <= indices[0] <= indices[-1] < len(frozen_vertices),
                    "changed Anoa source vertex selection")
            require([frozen_vertices[i] for i in indices] == candidate_vertices, "candidate vertex lacks Anoa provenance")
        else:
            require(original_part == derived_part, "changed fixed Anoa part")
            fixed += 1
        verify_topology(original_part, index in COLLARS)
        verify_topology(derived_part, index in COLLARS)
    require(fixed == 7 and len(selections) == 25, "changed Anoa candidate inventory")
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
                    "changed Anoa original overlap")
        require(pair not in contacts, "duplicate Anoa overlap")
        contacts.add(pair)
    require(contacts == OVERLAPS, "missing Anoa overlap")
    survey = json.loads((INTEGRATION / "CANDIDATE-TOPOLOGY-SURVEY.json").read_bytes())
    require(not survey["missingMeasuredOverlaps"] and not survey["additionalMeasuredOverlaps"],
            "changed Anoa overlap inventory")
    require({tuple(row["parts"]) for row in survey["candidateTopology"]["interiorOverlaps"]} == contacts,
            "inconsistent Anoa candidate survey")
    return sorted(contacts)


def verify():
    _, _, _, source_models, bindings = load_source()
    detailed = [section(source_models[0], part) for part in bindings[0]
                if part["name"] == "radial structural collar"]
    require(len(detailed) == 2 and all(len(row[1]) == 1152 for row in detailed),
            "changed detailed Anoa collar source")
    for collar in detailed:
        verify_topology(collar, collar=True)
    _, _, _, _, names, scaled = collision_inputs()
    streams = prepare()
    for name in ("source.vertices", "source-triangles.bin", "collision-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == streams[name], f"changed SH098 proof input: {name}")
    fixed = verify_candidate(*source_models[1:], *bindings[1:])
    contacts = verify_contacts(*(pair[0] for pair in scaled[1:]), *bindings[1:])
    clearance = verify_routes(names, scaled)
    return {"clearanceMillimetres": clearance, "originalOverlaps": contacts,
            "unchangedNativeParts": fixed, "closedCollars": 2, "candidateCollarFaces": [48, 48]}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
