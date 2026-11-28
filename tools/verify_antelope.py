"""Replay the exact SH089 arc, source contact, and guarded clearance evidence."""
from collections import Counter
from fractions import Fraction
import json
import sys

from prepare_antelope import APERTURE_X, BOW_GAP_Z, INTEGRATION, ORIGINAL, RING_SAMPLING, collision_inputs, encode_json, load_source, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def arc_topology(model, ring, major, minor):
    first = ring["faceStart"]
    count = ring["faceCount"]
    faces = model["faces"][first:first+count]
    edges = Counter((a,b) for face in faces for a,b in zip(face,face[1:]+face[:1],strict=True))
    require(all(n == edges[(b,a)] == 1 for (a,b),n in edges.items()), "Antelope arc lost closure or winding")
    used = {vertex for face in faces for vertex in face}
    require(len(used) == ring["vertexCount"], "Antelope arc lost source vertices")
    euler = len(used) - len(edges)//2 + len(faces)
    require(euler == 2, "Antelope open arc changed topology")
    rings = [(min(face)//minor,max(face)//minor) for face in faces]
    require(all(b in (a,a+1) for a,b in rings), "open arc gained a wrap face")
    end_shapes = [a for a,b in rings if a == b]
    require(Counter(end_shapes) == {0:minor-2,major-1:minor-2}, "open arc lost an original end cap")
    return euler


def verify():
    _, _, _, _, names, scaled = collision_inputs()
    _, _, _, _, part_sets = load_source()
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "collision-piece-membership.bin"):
        require((ORIGINAL/name).read_bytes() == derived[name], f"changed SH089 proof input: {name}")

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
        require(distance == route["clearanceMillimetres"] and distance > 100, "changed Antelope route clearance")
        guarded[route["name"]] = distance
    require(set(guarded) == {"bow-gap", *(f"{side}-aperture-{z:+}" for side in ("port","starboard") for z in (-4,0,4))},
            "missing Antelope aperture routes")

    contact = json.loads((INTEGRATION/"CANDIDATE-CONTACTS-AND-APERTURE.json").read_bytes())
    require(not contact["contacts"]["lost"] and not contact["contacts"]["unresolved"], "unresolved original Antelope join")
    original, candidate = [row[0] for row in scaled[1:]]
    contacts = set()
    for bond in contact["contacts"]["retained"]:
        a, b = bond["parts"]
        verified = []
        for model,parts,key in ((original,part_sets[1],"archiveOverlap"),
                                (candidate,part_sets[2],"candidateOverlap")):
            require([parts[i]["name"] for i in (a,b)] == bond["names"], "changed Antelope joint identity")
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
        require(all(verified), "changed Antelope original overlap")
        contacts.add((a,b))
    require(contacts == {(0,6),(0,7),(0,10),(0,13),(1,6),(1,7),(1,8),(1,10),(1,13),(8,9),(10,11),(13,14),(0,11),(0,14)},
            "missing Antelope original overlap witness")

    topology, aperture = {}, {}
    for lod,(model,_),parts,(major,minor) in zip(
        ("detailed","archivedNative","candidateNative"),scaled,part_sets,RING_SAMPLING,strict=True
    ):
        topology[lod] = arc_topology(model,parts[0],major,minor)
        ring = model["vertices"][:major*minor]
        clouds = [ring[i*minor:(i+2)*minor] for i in range(major-1)]
        for piece in parts[1:]:
            first = piece["vertexStart"]
            clouds.append(model["vertices"][first:first+piece["vertexCount"]])
        clouds = [[tuple(map(Fraction,p)) for p in cloud] for cloud in clouds]
        aperture[lod] = {}
        probes = [("bow-gap",0,BOW_GAP_Z)] + [
            (f"{side}-aperture-{z:+}",sign*APERTURE_X,z)
            for side,sign in (("port",-1),("starboard",1)) for z in (-4,0,4)
        ]
        for route,x,z in probes:
            proof = contact["aperture"][lod][route]
            tri = [[x-.035,-5,z],[x+.035,-5,z],[x,-5,z+.07]]
            swept = [tuple(map(Fraction,p)) for p in tri + [[a,9,c] for a,_,c in tri]]
            mm = separating_planes(clouds,swept,proof["axes"])
            require(mm == proof["clearanceMillimetres"] and mm > 100, "changed independent aperture proof")
            aperture[lod][route] = mm
    return {"clearanceMillimetres":guarded,"originalOverlaps":sorted(contacts),
            "openArcEulerCharacteristic":topology,"independentApertureMillimetres":aperture}


if __name__ == "__main__":
    sys.stdout.buffer.write(encode_json(verify()))
