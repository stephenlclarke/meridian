"""Replay the exact SH086 shield, source contact, and guarded clearance evidence."""
from collections import Counter
from fractions import Fraction
import json
import sys

from prepare_alpaca import INTEGRATION, ORIGINAL, RING_SAMPLING, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def shield_topology(model, ring):
    first = ring["faceStart"]
    count = ring["faceCount"]
    faces = model["faces"][first:first+count]
    edges = Counter((a,b) for face in faces for a,b in zip(face,face[1:]+face[:1],strict=True))
    require(all(n == edges[(b,a)] == 1 for (a,b),n in edges.items()), "Alpaca shield lost closure or winding")
    used = {vertex for face in faces for vertex in face}
    require(len(used) == ring["vertexCount"], "Alpaca shield lost source vertices")
    euler = len(used) - len(edges)//2 + len(faces)
    require(euler == 0, "Alpaca shield lost its handle")
    return euler


def verify():
    _, _, _, _, names, scaled = collision_inputs()
    _, _, _, _, part_sets = load_source()
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "collision-piece-membership.bin"):
        require((ORIGINAL/name).read_bytes() == derived[name], f"changed SH086 proof input: {name}")

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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Alpaca route clearance")
        guarded[route["name"]] = distance
    require(set(guarded) == {"dorsal", *(f"{side}-aperture-{z:+}" for side in ("port","starboard") for z in (-4,0,4))},
            "missing Alpaca aperture routes")

    contact = json.loads((INTEGRATION/"CANDIDATE-CONTACTS-AND-APERTURE.json").read_bytes())
    require(not contact["contacts"]["lost"] and not contact["contacts"]["unresolved"], "unresolved original Alpaca join")
    original, candidate = [row[0] for row in scaled[1:]]
    contacts = set()
    for bond in contact["contacts"]["retained"]:
        a, b = bond["parts"]
        verified = []
        for model,parts,key in ((original,part_sets[1],"archiveOverlap"),
                                (candidate,part_sets[2],"candidateOverlap")):
            require([parts[i]["name"] for i in (a,b)] == bond["names"], "changed Alpaca joint identity")
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
        require(all(verified), "changed Alpaca original overlap")
        contacts.add((a,b))
    require(contacts == {(0,8),(0,9),(0,11),(0,12),(1,6),(1,8),(1,11),(6,7),(8,9),(11,12)},
            "missing Alpaca original overlap witness")

    topology, aperture = {}, {}
    for lod,(model,_),parts,(major,minor) in zip(
        ("detailed","archivedNative","candidateNative"),scaled,part_sets,RING_SAMPLING,strict=True
    ):
        topology[lod] = shield_topology(model,parts[0])
        ring = model["vertices"][:major*minor]
        if any((max(face)//minor-min(face)//minor) not in (1,major-1)
               for face in model["faces"][:parts[0]["faceCount"]]):
            raise ValueError("an Alpaca shield face crosses authored sectors")
        ring += ring[:minor]
        clouds = [ring[i*minor:(i+2)*minor] for i in range(major)]
        for piece in parts[1:]:
            first = piece["vertexStart"]
            clouds.append(model["vertices"][first:first+piece["vertexCount"]])
        clouds = [[tuple(map(Fraction,p)) for p in cloud] for cloud in clouds]
        aperture[lod] = {}
        for side,sign in (("port",-1),("starboard",1)):
            x = sign*10.0
            for z in (-4,0,4):
                route = f"{side}-aperture-{z:+}"
                proof = contact["aperture"][lod][route]
                tri = [[x-.035,-5,z],[x+.035,-5,z],[x,-5,z+.07]]
                swept = [tuple(map(Fraction,p)) for p in tri + [[a,9,c] for a,_,c in tri]]
                mm = separating_planes(clouds,swept,proof["axes"])
                require(mm == proof["clearanceMillimetres"] and mm > 100, "changed Alpaca independent aperture proof")
                aperture[lod][route] = mm
    return {"clearanceMillimetres":guarded,"originalOverlaps":sorted(contacts),
            "closedLoopEulerCharacteristic":topology,"independentApertureMillimetres":aperture}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
