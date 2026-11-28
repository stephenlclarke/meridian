"""Replay Wisent Mk II's exact original contacts and guarded inspection routes."""

from fractions import Fraction
import json
import sys

from prepare_wisent_mk_ii import INTEGRATION, ORIGINAL, SCALE, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes

RESAMPLED = {0, 5, 18, 21, 22, 24, 25}
OVERLAPS = {
    (0,10), (0,11), (0,12), (0,14), (0,16), (0,20), (0,21), (0,22), (1,10), (1,20),
    (5,10), (5,11), (5,13), (5,14), (5,17), (5,23), (5,24), (5,25), (7,10), (7,23),
    (10,20), (10,23), (12,14), (12,15), (12,16), (13,14), (13,15), (13,17),
    (14,16), (14,17), (15,16), (15,17), (18,19), (20,21), (23,24),
}


def section(model, owner):
    start = owner["vertexStart"]
    faces = slice(owner["faceStart"], owner["faceStart"] + owner["faceCount"])
    return (
        model["vertices"][start:start + owner["vertexCount"]],
        [[i - start for i in face] for face in model["faces"][faces]],
        model["faceMaterials"][faces],
        model["faceTextures"][faces],
    )


def verify_candidate(source, nominee, old_parts, new_parts):
    refs = [section(source, part) for part in old_parts]
    targets = [section(nominee, part) for part in new_parts]
    require(len(refs) == len(targets) == 26, "changed Wisent Mk II component inventory")
    changed = set()
    for index, (original, candidate) in enumerate(zip(refs, targets, strict=True)):
        require(old_parts[index]["name"] == new_parts[index]["name"], "changed Wisent Mk II part identity")
        require(set(original[2]) == set(candidate[2]), "changed Wisent Mk II original material inventory")
        if original != candidate:
            changed.add(index)
    require(changed == RESAMPLED, "changed fixed Wisent Mk II source part")
    for index, count in ((0, 5), (5, 5), (21, 2), (22, 2), (24, 2), (25, 2)):
        archived = {point[2] for point in refs[index][0]}
        candidate = {point[2] for point in targets[index][0]}
        require(len(archived) == count and candidate == archived, "changed Wisent Mk II longitudinal station")
    for models in (refs, targets):
        right_edge = min(Fraction(p[0]) for p in models[5][0])
        left_edge = max(Fraction(p[0]) for p in models[0][0])
        require(int((right_edge-left_edge) * SCALE * 1000) == 7760, "changed Wisent Mk II hull bay separation")
    return len(refs)-len(changed)


def verify_routes(names, scaled, independent):
    pieces = [[] for _ in names]
    for model, membership in scaled:
        for point, (slot,) in zip(model["vertices"], membership, strict=True):
            pieces[slot].append(tuple(map(Fraction, point)))
    guarded = {}
    for route in json.loads((INTEGRATION / "CONTACT-CANDIDATE.json").read_bytes()):
        triangle = [tuple(map(Fraction,p)) for p in route["triangle"]]
        travel = tuple(map(Fraction,route["travel"]))
        swept = triangle + [tuple(a-b for a,b in zip(p,travel,strict=True)) for p in triangle]
        distance = separating_planes(pieces,swept,route["axes"])
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Wisent Mk II route clearance")
        require(route["name"] not in guarded, "duplicate Wisent Mk II route")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", "ventral", "central-bay-lane", "between-aft-drive-walls",
                            "through-dock-aperture", "port-outboard", "starboard-outboard"}, "missing Wisent Mk II route")
    require(set(independent) == {"detailed","archivedNative","candidateNative"}, "missing Wisent Mk II independent route records")
    for routes in independent.values():
        require(set(routes) == set(guarded) and all(isinstance(v, int) and v >= guarded[k] for k,v in routes.items()),
                "changed Wisent Mk II independent route clearance")
    return guarded


def verify_contacts(original, candidate, old_parts, new_parts, report):
    faces = [
        [[[tuple(map(Fraction, model["vertices"][i])) for i in face]
          for face in model["faces"][part["faceStart"]:part["faceStart"] + part["faceCount"]]] for part in parts]
        for model, parts in ((original, old_parts), (candidate, new_parts))
    ]
    require(not report["lost"] and not report["unresolved"], "missing Wisent Mk II archived overlap")
    found = set()
    for bond in report["retained"]:
        pair = tuple(bond["parts"])
        require(bond["names"] == [old_parts[index]["name"] for index in pair], "changed Wisent Mk II overlap names")
        for patches, key in zip(faces, ("archivedOverlap", "candidateOverlap"), strict=True):
            witness = bond[key]
            point = tuple(map(Fraction, witness["pointMetres"]))
            parity = [list(inside_y(point, patches[index])) for index in pair]
            require(parity == witness["hitCountsAboveBelow"] and all(n % 2 for side in parity for n in side),
                    "changed Wisent Mk II contact witness")
        require(pair not in found, "duplicate Wisent Mk II contact")
        found.add(pair)
    require(found == OVERLAPS, "changed Wisent Mk II overlap inventory")
    survey = json.loads((INTEGRATION / "CANDIDATE-TOPOLOGY-SURVEY.json").read_bytes())
    require(not survey["additionalMeasuredOverlaps"] and not survey["missingMeasuredOverlaps"],
            "changed Wisent Mk II candidate overlap survey")
    require({tuple(row["parts"]) for row in survey["measuredCandidateOverlaps"]} == found,
            "inconsistent Wisent Mk II candidate overlap survey")
    return sorted(found)


def verify():
    _, _, _, originals, bindings = load_source()
    _, _, _, _, names, scaled = collision_inputs()
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH097 proof input: {name}")
    contact = json.loads((INTEGRATION / "CANDIDATE-CONTACTS-AND-CLEARANCE.json").read_bytes())
    return {
        "clearanceMillimetres": verify_routes(names, scaled, contact["routeClearanceMillimetres"]),
        "originalOverlaps": verify_contacts(*(row[0] for row in scaled[1:]), *bindings[1:], contact["contacts"]),
        "unchangedNativeParts": verify_candidate(*originals[1:], *bindings[1:]),
        "cargoBayBodyLateralSeparationMillimetres": 7760,
    }


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
