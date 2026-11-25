"""Replay the exact SH090 source contact, and guarded clearance evidence."""
from fractions import Fraction
import json
import sys

from prepare_elephant import CANDIDATE_SHA, INTEGRATION, ORIGINAL, SOURCE_HASHES, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def verify():
    _, _, _, _, names, scaled = collision_inputs()
    _, _, _, _, part_sets = load_source()
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL/name).read_bytes() == derived[name], f"changed SH090 proof input: {name}")

    pieces = [[] for _ in names]
    for model, membership in scaled:
        for vertex, slots in zip(model["vertices"], membership, strict=True):
            for slot in slots:
                pieces[slot].append(tuple(map(Fraction, vertex)))

    route_path = INTEGRATION/"CONTACT-CANDIDATE.json"
    guarded = {}
    for route in json.loads(route_path.read_bytes()):
        hull = [tuple(map(Fraction,p)) for p in route["triangle"]]
        travel = tuple(map(Fraction,route["travel"]))
        swept = hull + [tuple(a-b for a,b in zip(p,travel,strict=True)) for p in hull]
        distance = separating_planes(pieces,swept,route["axes"])
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Elephant route clearance")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", "ventral", "between-aft-drives",
                            *(f"{side}-{route}" for side in ("port","starboard")
                              for route in ("outboard","forward-inner"))},
            "missing Elephant inspection routes")

    contact = json.loads((INTEGRATION/"CANDIDATE-CONTACTS-AND-CLEARANCE.json").read_bytes())
    require(not contact["contacts"]["lost"] and not contact["contacts"]["unresolved"],
            "unresolved original Elephant join")
    witnesses = json.loads((INTEGRATION/"EXACT-WITNESSES.json").read_bytes())
    require(len(witnesses["retainedOriginalWitnesses"]) == len(contact["contacts"]["retained"]) == 22,
            "missing original Elephant overlap")
    require(witnesses["candidateSHA256"] == CANDIDATE_SHA and witnesses["sourceMeshSHA256"] == SOURCE_HASHES["resource.toml"],
            "changed Elephant overlap evidence pin")
    original, candidate = [row[0] for row in scaled[1:]]
    contacts = set()
    for bond in witnesses["retainedOriginalWitnesses"]:
        a,b = bond["parts"]
        point = tuple(map(Fraction,bond["pointMetres"]))
        for model,parts,key in ((original,part_sets[1],"original"),
                                (candidate,part_sets[2],"candidate")):
            require([parts[i]["name"] for i in (a,b)] == bond["names"], "changed Elephant joint identity")
            parity = []
            for part in (parts[a],parts[b]):
                first = part["faceStart"]
                end = first + part["faceCount"]
                faces = [[tuple(map(Fraction,model["vertices"][i])) for i in row]
                         for row in model["faces"][first:end]]
                parity.append(list(inside_y(point,faces)))
            require(parity == bond["exactRayHits"][key], "changed original Elephant overlap")
        contacts.add((a,b))
    require(len(contacts)==22, "duplicate Elephant overlap witness")
    for routes in witnesses["routeClearanceMillimetres"].values():
        require(set(routes) == set(guarded) and all(type(mm) is int and mm>100 for mm in routes.values()),
                "changed Elephant independent clearance")
    return {"clearanceMillimetres":guarded,"originalOverlaps":sorted(contacts)}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
