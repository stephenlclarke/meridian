"""Replay Magpie's calibrated original joins and inspection routes."""

from collections import Counter
from fractions import Fraction
from hashlib import sha256
import json
import sys

from prepare_magpie import ARCHIVED_COLLISION_SHA, INTEGRATION, ORIGINAL, SCALE, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes

OVERLAPS = {(0, 5),
 (0, 6),
 (0, 19),
 (0, 21),
 (0, 24),
 (1, 24),
 (2, 21),
 (3, 5),
 (4, 6),
 (5, 7),
 (5, 11),
 (5, 17),
 (6, 12),
 (6, 15),
 (6, 18),
 (8, 17),
 (14, 18),
 (17, 21),
 (18, 24),
 (19, 20),
 (21, 22),
 (24, 25)}


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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Magpie route clearance")
        require(route["name"] not in guarded, "duplicate Magpie route")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", "ventral", "between-aft-drives",
                            *(f"{side}-{suffix}" for side in ("port", "starboard")
                              for suffix in ("outboard", "inboard-of-fin"))}, "missing Magpie route")
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
    proposed_bytes = (INTEGRATION / "ARCHIVED-COLLISION.json").read_bytes()
    require(sha256(proposed_bytes).hexdigest() == ARCHIVED_COLLISION_SHA, "changed Magpie archived proposal")
    proposed = json.loads(proposed_bytes)
    require(proposed["catalogueID"] == "SH107" and proposed["calibration"] == str(SCALE)
            and proposed["coordinateUnits"] == "metres", "wrong Magpie archived proposal")
    proposal, original_bindings = proposed["nativeDesign"], proposed["parts"]
    record = json.loads((ORIGINAL / "GENERATED-COLLISION.json").read_bytes())
    require(record["contactRepair"] == {"part": 19, "sourceVertexOut": 411, "sourceVertexIn": 377},
            "wrong Magpie glazing repair identity")
    require(record["baselineCandidateSource"] == "ARCHIVED-COLLISION.json"
            and proposed["originalSourceSHA256"] == record["originalSourceSHA256"],
            "wrong Magpie proposal ancestry")

    def topology(points, faces):
        edges = Counter((a, b) for x, y, z in faces for a, b in ((x, y), (y, z), (z, x)))
        require(len(points) - len(edges) // 2 + len(faces) == 2
                and all(count == edges[(b, a)] == 1 for (a, b), count in edges.items()),
                "changed Magpie closed-component topology")

    fixed = 0
    for old, base, new in zip(parts_old, original_bindings, parts_new, strict=True):
        before = section(original, old)
        archived = section(proposal, base)
        after = section(candidate, new)
        require((old["index"], old["name"]) == (base["index"], base["name"]) == (new["index"], new["name"]),
                "changed Magpie part order")
        ids = new["originalVertexIDs"]
        baseline_ids = base["originalVertexIDs"]
        require(len(ids) == len(set(ids)) == len(after[0])
                and all(old["vertexStart"] <= i < old["vertexStart"] + old["vertexCount"] for i in ids + baseline_ids),
                "changed Magpie creator part support")
        require([original["vertices"][i] for i in ids] == after[0]
                and [[axis * float(SCALE) for axis in original["vertices"][i]] for i in baseline_ids] == archived[0],
                "Magpie candidate vertex lacks original provenance")
        require(set(before[2]) == set(after[2]), "changed Magpie part materials")
        if old["index"] == 19:
            require(set(baseline_ids) - set(ids) == {411} and set(ids) - set(baseline_ids) == {377}
                    and len(archived[1]) == len(after[1]) == 30, "changed Magpie one-vertex repair")
        else:
            require(ids == baseline_ids and after[1:] == archived[1:],
                    "changed part outside Magpie glazing repair")
        topology(*before[:2])
        topology(*after[:2])
        fixed += before == after

    body = section(proposal, original_bindings[0])[0]
    glazing = section(proposal, original_bindings[19])[0]
    gap = min(Fraction(p[1]) for p in glazing) - max(Fraction(p[1]) for p in body)
    require(fixed == 19 and len(parts_new) == 27 and int(gap * 1000) == 117,
            "changed Magpie fixed parts or exact archived glazing gap")
    return fixed, int(gap * 1000)


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
                    "changed Magpie original overlap")
        require(pair not in contacts, "duplicate Magpie overlap")
        contacts.add(pair)
    require(contacts == OVERLAPS, "missing Magpie overlap")
    survey = json.loads((INTEGRATION / "CANDIDATE-TOPOLOGY-SURVEY.json").read_bytes())
    require(not survey["missingMeasuredOverlaps"] and not survey["additionalMeasuredOverlaps"],
            "changed Magpie overlap inventory")
    require({tuple(row["parts"]) for row in survey["candidateTopology"]["interiorOverlaps"]} == contacts,
            "inconsistent Magpie candidate survey")
    return sorted(contacts)


def verify():
    _, _, _, source_models, bindings = load_source()
    _, _, _, _, names, scaled = collision_inputs()
    streams = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == streams[name], f"changed SH107 proof input: {name}")
    fixed, gap = verify_candidate(*source_models[1:], *bindings[1:])
    contacts = verify_contacts(*(pair[0] for pair in scaled[1:]), *bindings[1:])
    clearance = verify_routes(names, scaled)
    return {"clearanceMillimetres": clearance, "originalOverlaps": contacts,
            "unchangedNativeParts": fixed, "archivedProposalGlazingGapMillimetres": gap}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
