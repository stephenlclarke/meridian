"""Measure SH136 local supports against the three calibrated meshes."""
import argparse
from fractions import Fraction
from functools import lru_cache
from heapq import heappop, heappush
from math import isqrt
from itertools import combinations
import json
import sys

import numpy as np
import trimesh

from measure_banteng_geometry import adaptive_upper, upper
from prepare_falcon_mk_i import INTEGRATION, RADIUS, SCALE, collision_inputs, load_source, require
from measure_goshawk_surface_fit import dyadic
from vulture_mk_i_exact_fit import TriangleIndex, bound_fan, grid_index, squared_distance


def load():
    _, _, _, _, names, raw = collision_inputs()
    vertices, groups, meshes = [], {name:[] for name in names}, {}
    for lod,(model,membership) in zip(("detailed","archivedNative","candidateNative"),raw,strict=True):
        offset = len(vertices)
        vertices.extend(model["vertices"])
        meshes[lod] = trimesh.Trimesh(model["vertices"],model["faces"],process=False)
        for index,slots in enumerate(membership):
            for slot in slots:
                groups[names[slot]].append(offset+index)
    return np.array(vertices), meshes, {name:np.array(ids) for name,ids in groups.items()}


def fit(vertices, meshes, groups):
    fans = json.loads((INTEGRATION/"SUPPORT-FAN.json").read_bytes())
    targets = {"detailed":2800,"archivedNative":3800,"candidateNative":4400}
    indices = {name:grid_index(mesh) for name,mesh in meshes.items()}
    per_group = {}
    for name,faces in fans.items():
        i = groups[name]
        local = np.searchsorted(i,faces)
        require(np.all(local < len(i)) and np.array_equal(i[local],faces), "Falcon Mk I support face spans groups")
        bound_fan(vertices[i],local)
        per_group[name] = {lod:adaptive_upper(index,vertices[faces],targets[lod]) for lod,index in indices.items()}
    return {"nominalPaddingMetres":RADIUS,"minimumRefinementTargetsMillimetres":targets,
            "fullUpperMillimetres":{lod:max(record[lod] for record in per_group.values()) for lod in meshes},
            "pieces":per_group}


def fidelity(meshes):
    indices = {name: grid_index(mesh) for name, mesh in meshes.items()}
    bounds = {}
    for first, second in combinations(meshes, 2):
        bounds[first + "To" + second[0].upper() + second[1:]] = upper(meshes[first], indices[second])
        bounds[second + "To" + first[0].upper() + first[1:]] = upper(meshes[second], indices[first])
    return {"continuousUpperMillimetres": bounds}


DEPTH=12
TOLERANCE_MM=25


def owner(parts,face):
    return next((part["index"],part["name"]) for part in parts
                if part["faceStart"]<=face<part["faceStart"]+part["faceCount"])


def box(vertices):
    return [[min(axis),max(axis)] for axis in zip(*vertices,strict=True)]


def directed(first,second):
    source, source_parts=first
    target, target_parts=second
    units=[tuple(x*float(SCALE) for x in v) for v in source["vertices"]+target["vertices"]]
    points,base=dyadic(units)
    scale=base*2**DEPTH
    points=[tuple(x*2**DEPTH for x in p) for p in points]
    offset=len(source["vertices"])
    target_tris=[tuple(points[offset+i] for i in face) for face in target["faces"]]
    index=TriangleIndex(target_tris)
    distance=lru_cache(maxsize=100000)(index.nearest)
    pending=[]
    best=Fraction(0)
    winner=None
    serial=0

    def examine(tri, face, depth):
        nonlocal best,winner,serial
        centre=tuple(sum(row)//3 for row in zip(*tri,strict=True))
        for point in tri:
            raw, ties=distance(point)
            value=Fraction(*raw)
            if winner is None or value>best:
                best=value
                winner=(face,point,ties)
        # Every convex patch is bounded by one reference triangle. Choosing it
        # with the lattice centroid is a heuristic; only the corner bound matters.
        _,ties=distance(centre)
        reference=target_tris[ties[0]]
        cap=max(Fraction(*squared_distance(point,reference)) for point in tri)
        serial+=1
        heappush(pending,(-cap,serial,face,depth,tri))

    for face, corners in enumerate(source["faces"]):
        examine(tuple(points[i] for i in corners), face, 0)
    tolerance=Fraction(TOLERANCE_MM*scale,1000)
    explored=0
    while pending:
        cap=-pending[0][0]
        # A floor on the lattice square root keeps the stopping inequality conservative.
        lower=isqrt(best.numerator//best.denominator)
        if cap <= (lower+tolerance)**2:
            break
        _,_,face,depth,tri=heappop(pending)
        assert depth<DEPTH, (face,depth)
        a,b,c=tri
        ab,bc,ac=(tuple((v+w)//2 for v,w in zip(p,q,strict=True)) for p,q in ((a,b),(b,c),(a,c)))
        for patch in ((a,ab,ac),(ab,b,bc),(ac,bc,c),(ab,bc,ac)):
            examine(patch,face,depth+1)
        explored+=1
    if winner is None or not pending:
        raise ValueError("Directed fit requires a measured source face")
    upper=-pending[0][0]
    face,point,ties=winner
    lower_mm=isqrt((best*1000000/(scale*scale))//1)
    scaled_upper=upper*1000000/(scale*scale)
    upper_mm=isqrt(scaled_upper//1)
    upper_mm+=upper_mm*upper_mm<scaled_upper
    target_ref=[[float(Fraction(x,scale)) for x in v] for v in target_tris[ties[0]]]
    return {
        "directedMaximumMillimetres":{"lowerFloor":lower_mm,"upperCeiling":upper_mm},
        "sourceWitness":{"face":face,"part":owner(source_parts,face),
                         "pointMetres":[float(Fraction(v,scale)) for v in point],
                         "exactPointMetres":[str(Fraction(v,scale)) for v in point],
                         "squaredDistanceMetres":str(best/(scale*scale))},
        "nearestTarget":{"faces":ties,"parts":[owner(target_parts,i) for i in ties],
                         "oneTriangleBoxMetres":box(target_ref)},
        "refinedPatches":explored,
    }


def directions(reference_only):
    _,_,_,raw,parts=load_source()
    detail,archive,candidate=zip(raw,parts,strict=True)
    if reference_only:
        return {"detailedToArchive":directed(detail,archive),"archiveToDetailed":directed(archive,detail)}
    return {"archiveToCandidate":directed(archive,candidate),"candidateToArchive":directed(candidate,archive),
            "detailedToCandidate":directed(detail,candidate),"candidateToDetailed":directed(candidate,detail),
            "axisExtremaMetres":{label:box([[v*float(SCALE) for v in p] for p in mesh["vertices"]])
                                 for label,(mesh,_) in (("detailed",detail),("archive",archive),("candidate",candidate))}}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--fidelity", action="store_true")
    mode.add_argument("--directions", action="store_true")
    mode.add_argument("--reference-directions", action="store_true")
    args = parser.parse_args()
    if args.directions or args.reference_directions:
        result = directions(args.reference_directions)
    else:
        data = load()
        result = fidelity(data[1]) if args.fidelity else fit(*data)
    sys.stdout.buffer.write((json.dumps(result, indent=2) + "\n").encode())
