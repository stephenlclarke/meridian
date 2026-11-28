"""Replay exact Gopher ring topology, original joints and guarded routes."""
from collections import Counter
from fractions import Fraction
import json
import sys

from import_goshawk_envelope_source import components
from prepare_gopher import INTEGRATION, ORIGINAL, collision_inputs, encode_json, prepare, require
from verify_eagle_cruiser_geometry import inside_y, separating_planes


def verify_roll_frame(model, span):
    lo, hi = span
    faces = model["faces"][lo:hi]
    used = {index for face in faces for index in face}
    edges = Counter((a,b) for face in faces for a,b in zip(face,face[1:]+face[:1],strict=True))
    require(all(n == edges[(b,a)] == 1 for (a,b),n in edges.items()), "Gopher roll frame lost closure or winding")
    euler = len(used) - len(edges)//2 + len(faces)
    require(euler == 0, "Gopher roll frame lost its opening")
    return euler


def verify():
    _, _, _, models, names, memberships = collision_inputs()
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "collision-piece-membership.bin"):
        require((ORIGINAL/name).read_bytes() == derived[name], f"changed Gopher proof input: {name}")
    clouds = [[] for _ in names]
    for model, rows in zip(models,memberships,strict=True):
        for vertex, slots in zip(model["vertices"],rows,strict=True):
            for slot in slots:
                clouds[slot].append(tuple(map(Fraction,vertex)))

    guarded = {}
    for route in json.loads((INTEGRATION/"CONTACT-CANDIDATE.json").read_bytes()):
        tri = [tuple(map(Fraction,p)) for p in route["triangle"]]
        motion = tuple(map(Fraction,route["travel"]))
        swept = tri + [tuple(a-b for a,b in zip(point,motion,strict=True)) for point in tri]
        mm = separating_planes(clouds,swept,route["axes"])
        require(mm == route["clearanceMillimetres"] and mm > 100, "changed Gopher roll-frame route")
        guarded[route["name"]] = mm
    require(set(guarded) == {"dorsal","above-hull-inside-frame","below-hull-inside-frame",
            "port-inside-frame","starboard-inside-frame","port-outboard","starboard-outboard"},
            "missing Gopher inspection route")

    shared = {
        "pressure-hull-to-bore-shaft","bore-shaft-to-bore-head","pressure-hull-to-canopy",
        *(f"{side}-frame-stay-to-{part}" for side in ("port","starboard") for part in ("pressure-hull","roll-frame")),
        "pressure-hull-to-port-nozzle","pressure-hull-to-starboard-nozzle",
    }
    bindings = json.loads((INTEGRATION/"ORIGINAL-JOINTS.json").read_bytes())
    topology, centre, verified = {}, {}, {}
    for lod,model,rows in zip(("detailed","native"),models,memberships,strict=True):
        name_map = {name: [i for i,n in enumerate(names) if n==name or name=="roll-frame" and n.startswith("roll-frame-")]
                    for name in {*names,"roll-frame"}}
        rings = set(components(model)[1])
        face_indices = [i for i,f in enumerate(model["faces"]) if f[0] in rings]
        require(face_indices == list(range(face_indices[0],face_indices[-1]+1)), "Gopher roll frame lost contiguous faces")
        topology[lod] = verify_roll_frame(model,(face_indices[0],face_indices[-1]+1))
        vertices = [tuple(map(Fraction,v)) for v in model["vertices"]]
        for proof in bindings[lod].values():
            point = tuple(map(Fraction,proof["pointMetres"]))
            require(len(proof["parts"])==2, "wrong Gopher joint")
            for name,part in proof["parts"].items():
                first,last = part["faceRange"]
                faces = model["faces"][first:last]
                require(bool(faces) and all(set(rows[i]).intersection(name_map[name])
                        for face in faces for i in face), "Gopher joint witness leaves its local part")
                parity = inside_y(point,[[vertices[i] for i in face] for face in faces])
                require(list(parity)==part["hitCountsAboveBelow"], "changed Gopher original contact")
        expected = shared | ({"canopy-to-canopy-ridge"} if lod=="detailed" else set())
        require(set(bindings[lod]) == expected, "missing Gopher original overlap")
        verified[lod] = list(bindings[lod])
        centre_point = tuple(map(Fraction,(0.137,0.232,0.013)))
        core = [[vertices[i] for i in face] for face in model["faces"]
                 if all(name_map["pressure-hull"][0] in rows[i] for i in face)]
        centre[lod] = list(inside_y(centre_point,core))
    return {"clearanceMillimetres":guarded,"rollFrameEulerCharacteristic":topology,
            "pressureHullCentreParity":centre,"originalOverlaps":verified}


if __name__=="__main__":
    sys.stdout.buffer.write(encode_json(verify()))
