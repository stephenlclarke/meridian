"""Measure Cheetah contact, clearance and visual fit against the two original meshes."""
from fractions import Fraction
from itertools import combinations
from math import gcd, isqrt
import json

import numpy as np
from scipy.spatial import ConvexHull
import trimesh

from measure_banteng_geometry import section, solid, upper
from verify_eagle_cruiser_geometry import inside_y, separating_planes
from vulture_mk_i_exact_fit import grid_index

from prepare_cheetah_shield import ASSETS, inputs
from repair_cheetah_casings import repair

SCALE = Fraction(7500, 1081)


def load():
    resource, original, _, _ = inputs()
    native, parts = repair(ASSETS)
    return {
        "detailed": (resource["design"], original["design"]["parts"]),
        "archivedNative": (resource["nativeDesign"], original["nativeDesign"]["parts"]),
        "candidateNative": (native, parts),
    }


def as_mesh(vertices, faces):
    return trimesh.Trimesh(np.array(vertices) * float(SCALE), faces, process=False)


def as_parts(model, bindings):
    output = []
    for binding in bindings:
        vertices, faces, materials, textures = section(model, binding)
        mesh = as_mesh(vertices, faces)
        solid(mesh)
        output.append((mesh, binding, [vertices, faces, materials, textures]))
    return output


def integer_axis(normal):
    axis = tuple(int(v) for v in np.rint(normal * 2**20))
    return tuple(value // gcd(*axis) for value in axis)


def exact_cloud(points):
    return [tuple(map(Fraction, point)) for point in points]


def strict_gap(first, second, full_search=False):
    hulls = [ConvexHull(points) for points in (first, second)]
    normals = np.vstack([hull.equations[:, :3] for hull in hulls] + [np.eye(3)])
    if full_search:
        edges = []
        for hull, points in zip(hulls, (first, second), strict=True):
            edges.append(np.array([points[b] - points[a] for a,b in {tuple(sorted(edge))
                for face in hull.simplices for edge in combinations(face,2)}]))
        cross = np.cross(edges[0][:, None, :], edges[1][None, :, :]).reshape(-1, 3)
        norm = np.linalg.norm(cross, axis=1)
        normals = np.vstack((normals, cross[norm > 1e-12] / norm[norm > 1e-12, None]))
    normals = np.vstack([normals, -normals])
    gaps = (second @ normals.T).min(0) - (first @ normals.T).max(0)
    order = np.argsort(gaps)[::-1]
    points_a, points_b = exact_cloud(first), exact_cloud(second)
    for i in order:
        if gaps[i] <= 0:
            break
        axis = integer_axis(normals[i])
        def projection(p):
            return sum(a * b for a, b in zip(axis, p, strict=True))
        gap = min(map(projection, points_b)) - max(map(projection, points_a))
        if gap > 0:
            return {"axis": axis, "strictSeparationMillimetres": isqrt((gap * 1000)**2 // sum(a*a for a in axis))}
    return None


def witness(first, second):
    lower = np.maximum(first.bounds[0], second.bounds[0])
    upper_bound = np.minimum(first.bounds[1], second.bounds[1])
    if not np.all(upper_bound > lower):
        return None
    rng = np.random.default_rng(82)
    points = rng.uniform(lower, upper_bound, (2048, 3)).round(7)
    yes = first.contains(points) & second.contains(points)
    surfaces = [[exact_cloud(face) for face in mesh.triangles] for mesh in (first, second)]
    for sample in points[yes]:
        try:
            parity = [inside_y(tuple(map(Fraction, sample)), faces) for faces in surfaces]
        except ValueError:
            continue
        return {"pointMetres": sample.tolist(), "hitCountsAboveBelow": parity}
    return None


def contacts(archive, candidate):
    # Every original pressure, wing, cockpit, well, casing, and support relationship
    # involving a changed part. Fixed-to-fixed pairs are byte-identical.
    pairs = [(a,b) for a,b in combinations(range(len(archive)),2)
             if any(archive[i][2] != candidate[i][2] for i in (a,b))]
    lost, retained, unresolved = [], [], []
    for a,b in pairs:
        old_a, old_b = archive[a][0], archive[b][0]
        if np.any(old_a.bounds[1] <= old_b.bounds[0]) or np.any(old_b.bounds[1] <= old_a.bounds[0]):
            continue
        was_joined = witness(old_a, old_b)
        if was_joined is None:
            continue
        new_a, new_b = candidate[a][0],candidate[b][0]
        gap = strict_gap(new_a.vertices, new_b.vertices, full_search=True)
        record = {"parts": [a,b], "names": [archive[i][1]["name"] for i in (a,b)],
                  "archivedOverlap": was_joined}
        if gap:
            lost.append(dict(record, **gap))
            continue
        still_joined = witness(new_a, new_b)
        (retained if still_joined else unresolved).append(dict(record, candidateOverlap=still_joined))
    return {"lost": lost, "retained": retained, "unresolved": unresolved}


def silhouettes(reference, candidate):
    steps = ((1,0),(2,1),(1,1),(1,2),(0,1),(-1,2),(-1,1),(-2,1))
    directions = steps + tuple((-a,-b) for a,b in steps)
    source, target = [exact_cloud(m.vertices) for m in (reference, candidate)]
    result = {}
    for name, a, b in (("xy",0,1),("xz",0,2),("yz",1,2)):
        span = max((max(x*p[a]+y*p[b] for p in source) - max(x*p[a]+y*p[b] for p in target))**2
                   * 1_000_000 / (x*x+y*y) for x,y in directions)
        r = isqrt(span.numerator//span.denominator)
        result[name] = r + (r*r < span)
    return result


def clearances(parts):
    clouds = [mesh.vertices for mesh, _, _ in parts]
    kernels = [ConvexHull(p).equations[:, :3] for p in clouds]
    specs = [("dorsal",0,4.2,-15.5,15.5), ("ventral",0,-4.1,-15.5,15.5)]
    for side, sign in (("port",-1),("starboard",1)):
        specs += [(f"{side}-outboard",sign*8.2,0,-15.5,15.5),
                  (f"{side}-forward-inner",sign*.7,-2.25,6,14.9)]
    result = {}
    for name, x, y, start, end in specs:
        obstacle = np.array([[x-.03,y,start],[x+.03,y,start],[x,y+.06,start]])
        sweep = np.vstack((obstacle, obstacle + np.array([0,0,end-start])))
        axes = []
        for cloud, normals in zip(clouds, kernels, strict=True):
            directions = np.vstack((normals, np.eye(3), -np.eye(3)))
            gaps = (sweep @ directions.T).min(0) - (cloud @ directions.T).max(0)
            index = gaps.argmax()
            if gaps[index] <= 0:
                break
            axes.append(integer_axis(directions[index]))
        if len(axes) == len(clouds):
            result[name] = separating_planes([exact_cloud(p) for p in clouds], exact_cloud(sweep), axes)
        else:
            result[name] = {"noSeparatingPlaneFoundForPart": len(axes), "probe": sweep.tolist()}
    return result


def fidelity(models, parts):
    ref = {label: as_mesh(m["vertices"], m["faces"]) for label,(m,_) in models.items()}
    old, new = parts["archivedNative"], parts["candidateNative"]
    assert [r[1]["name"] for r in old] == [r[1]["name"] for r in new]
    index = {name: grid_index(mesh) for name, mesh in ref.items()}
    changes = []
    for o, n in zip(old, new, strict=True):
        assert set(o[2][2]) == set(n[2][2])
        if o[2] != n[2]:
            changes.append({"index": n[1]["index"], "name": n[1]["name"],
                            "candidateToArchivedUpperMm": upper(n[0], index["archivedNative"]),
                            "archivedToCandidateUpperMm": upper(o[0], index["candidateNative"])})
    return {"unchangedParts": len(old)-len(changes), "resampledParts": changes,
            "silhouetteOffsetsMm": {name: silhouettes(ref[name], ref["candidateNative"])
                for name in ("detailed", "archivedNative")},
            "wholeSurfaceUpperMm": {
                a+"To"+b[0].upper()+b[1:]: upper(ref[a], index[b])
                for a,b in (("archivedNative","candidateNative"), ("candidateNative","archivedNative"),
                            ("detailed","candidateNative"), ("candidateNative","detailed"),
                            ("detailed","archivedNative"), ("archivedNative","detailed"))}}


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", choices=("contacts", "fidelity"))
    args = parser.parse_args()
    models = load()
    parts = {name: as_parts(mesh, owner) for name, (mesh, owner) in models.items()}
    if args.report == "contacts":
        result = {"contacts": contacts(parts["archivedNative"], parts["candidateNative"]),
                  "clearances": {name: clearances(value) for name, value in parts.items()}}
    else:
        result = fidelity(models, parts)
    import sys
    sys.stdout.buffer.write((json.dumps(result, indent=2) + "\n").encode())
