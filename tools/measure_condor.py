"""Measure the original Condor mesh difference and rounded support fit."""

import argparse
from decimal import Decimal, localcontext
import json
from math import isqrt
import sys

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_condor import INTEGRATION, ORIGINAL, prepare, require
from verify_condor_collision import load_collision
from vulture_mk_i_exact_fit import TriangleIndex, bound_fan, full_upper, grid_index


def load():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH055 fit input: {name}")
    vertices = np.frombuffer(derived["source.vertices"], dtype="<f8", offset=16).reshape(-1, 3)
    faces = np.frombuffer(derived["source-triangles.bin"], dtype="<u4").reshape(-1, 3)
    membership = np.frombuffer(derived["source-piece-membership.bin"], dtype="u1")
    names = json.loads(derived["PIECES.json"])["pieces"]
    meshes = {
        "detailed": (0, trimesh.Trimesh(vertices[:2308], faces[:4504], process=False)),
        "native": (2308, trimesh.Trimesh(vertices[2308:], faces[4504:] - 2308, process=False)),
    }
    return vertices, meshes, {name: np.flatnonzero(membership == slot) for slot, name in enumerate(names)}


def measure_fit(vertices, meshes, groups, *, collision=False):
    fan_name = "COLLISION-FAN.json" if collision else "SUPPORT-FAN.json"
    fans = json.loads((INTEGRATION / fan_name).read_bytes())
    require(list(fans) == list(groups), "wrong Condor support fan order")
    wing_target = 3600 if collision else 6200
    targets = {
        "wings": {"detailed": wing_target, "native": wing_target},
        "otherPieces": {"detailed": 1500, "native": 2100},
    }
    indices = {name: grid_index(mesh) for name, (_, mesh) in meshes.items()}
    rows = {}
    for name, faces in fans.items():
        ids = groups[name]
        local = np.searchsorted(ids, faces)
        require(np.all(local < len(ids)) and np.array_equal(ids[local], faces), "fan crosses original SH055 groups")
        bound_fan(vertices[ids], local)
        limits = targets["wings" if "-wing" in name else "otherPieces"]
        rows[name] = {lod: full_upper(index, vertices[faces], limits[lod]) for lod, index in indices.items()}
    return {
        "nominalPaddingMetres": .02, "refinementTargetsMillimetres": targets,
        "fullUpperMillimetres": {lod: max(row[lod] for row in rows.values()) for lod in meshes},
        "pieces": rows,
    }


def prior_wing_stand_off(vertices, meshes, groups):
    exact, scale, triangles = fit.integer_source(vertices, meshes)
    witnesses = json.loads((INTEGRATION / "WING-STANDOFF-WITNESSES.json").read_bytes())
    original_fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    require(set(witnesses) == set(meshes), "missing Condor prior wing witness")
    certified = {}
    for lod, witness in witnesses.items():
        name, face = witness["wing"], witness["supportVertices"]
        weights = witness["weightsOverFive"]
        require(name.endswith("-wing") and face in original_fans[name]
                and set(face) <= set(groups[name]), "Condor prior witness leaves its wing support")
        require(len(weights) == len(face) == 3 and sum(weights) == 5 and min(weights) >= 0,
                "wrong Condor prior witness weights")
        coordinates = tuple(sum(w * exact[i][axis] for w, i in zip(weights, face, strict=True)) for axis in range(3))
        scaled = [tuple(tuple(5 * value for value in point) for point in tri) for tri in triangles[lod]]
        (num, den), nearest = TriangleIndex(scaled).nearest(coordinates)
        lower = isqrt(1_000_000 * num // (den * (5 * scale)**2)) - 1
        require(lower == witness["strictLowerMillimetres"] and nearest == witness["nearestTriangles"],
                "changed Condor prior wing witness")
        fit.verify_lower(coordinates, scaled, 5 * scale, lower)
        certified[lod] = lower
    return certified


def mesh_difference(vertices, meshes):
    exact, scale, triangles = fit.integer_source(vertices, meshes)
    split = meshes["native"][0]
    results = {}
    for target, source in (("detailed", range(split, len(vertices))), ("native", range(split))):
        index = TriangleIndex(triangles[target])
        best_num, best_den = -1, 1
        for vertex in source:
            (num, den), faces = index.nearest(exact[vertex])
            if num * best_den > best_num * den:
                best_num, best_den, chosen, nearest = num, den, vertex, faces
        denominator = best_den * scale**2
        lower = isqrt(best_num * 1_000_000 // denominator) - 1
        fit.verify_lower(exact[chosen], triangles[target], scale, lower)
        with localcontext() as context:
            context.prec = 50
            distance = float((Decimal(best_num) / Decimal(denominator)).sqrt())
        results[target] = {"vertex": chosen, "metres": distance, "faces": nearest, "strictLowerMillimetres": lower}
    return results


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--difference", action="store_true")
    mode.add_argument("--collision", action="store_true")
    args = parser.parse_args()
    vertices, meshes, groups = load()
    if args.collision:
        prior = prior_wing_stand_off(vertices, meshes, groups)
        _, _, _, collision_groups = load_collision()
        groups = {name: np.array(ids) for name, ids in collision_groups.items()}
    if args.difference:
        report = mesh_difference(vertices, meshes)
    else:
        report = measure_fit(vertices, meshes, groups, collision=args.collision)
        if args.collision:
            report["priorWingStrictLowerMillimetres"] = prior
    sys.stdout.buffer.write((json.dumps(report, indent=2) + "\n").encode())
