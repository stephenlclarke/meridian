"""Replay the exact SH087 source contact, and guarded clearance evidence."""
from fractions import Fraction
import json
import sys

from prepare_elk import INTEGRATION, ORIGINAL, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def verify():
    _, _, _, _, names, scaled = collision_inputs()
    _, _, _, _, part_sets = load_source()
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL/name).read_bytes() == derived[name], f"changed SH087 proof input: {name}")

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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Elk route clearance")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", "ventral", "between-aft-drives",
                            *(f"{side}-{route}" for side in ("port","starboard")
                              for route in ("outboard","forward-inner"))},
            "missing Elk inspection routes")

    contact = json.loads((INTEGRATION/"CANDIDATE-CONTACTS-AND-CLEARANCE.json").read_bytes())
    require(not contact["contacts"]["lost"] and not contact["contacts"]["unresolved"], "unresolved original Elk join")
    original, candidate = [row[0] for row in scaled[1:]]
    contacts = set()
    for bond in contact["contacts"]["retained"]:
        a, b = bond["parts"]
        verified = []
        for model,parts,key in ((original,part_sets[1],"archivedOverlap"),
                                (candidate,part_sets[2],"candidateOverlap")):
            require([parts[i]["name"] for i in (a,b)] == bond["names"], "changed Elk joint identity")
            witness = bond[key]
            point = tuple(map(Fraction,witness["pointMetres"]))
            parity = []
            for part in (parts[a],parts[b]):
                first = part["faceStart"]
                end = first + part["faceCount"]
                faces = [[tuple(map(Fraction,model["vertices"][i])) for i in row]
                         for row in model["faces"][first:end]]
                parity.append(list(inside_y(point,faces)))
            verified.append(parity == witness["hitCountsAboveBelow"])
        require(all(verified), "changed Elk original overlap")
        contacts.add((a,b))
    require(contacts == {(0,5),(0,8),(0,9),(0,11),(0,12),(5,6),(6,7),(8,9),(11,12)},
            "missing Elk original overlap witness")
    require(set(contact["routeClearanceMillimetres"]) == {"detailed","archivedNative","candidateNative"},
            "missing Elk independent route evidence")
    for routes in contact["routeClearanceMillimetres"].values():
        require(set(routes) == set(guarded) and all(isinstance(m,int) and m >= guarded[name] for name,m in routes.items()),
                "changed Elk independent clearance")
    return {"clearanceMillimetres":guarded,"originalOverlaps":sorted(contacts)}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
