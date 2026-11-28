"""Measure the original Hobby Fighter mesh difference and rounded support fit."""

import argparse
from decimal import Decimal, localcontext
import json
from math import isqrt
import sys

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_hobby_fighter import INTEGRATION, ORIGINAL, prepare, require
from vulture_mk_i_exact_fit import TriangleIndex, bound_fan, full_upper, grid_index


def load():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH054 fit input: {name}")
    vertices = np.frombuffer(derived["source.vertices"], dtype="<f8", offset=16).reshape(-1, 3)
    faces = np.frombuffer(derived["source-triangles.bin"], dtype="<u4").reshape(-1, 3)
    membership = np.frombuffer(derived["source-piece-membership.bin"], dtype="u1")
    names = json.loads(derived["PIECES.json"])["pieces"]
    meshes = {
        "detailed": (0, trimesh.Trimesh(vertices[:2374], faces[:4620], process=False)),
        "native": (2374, trimesh.Trimesh(vertices[2374:], faces[4620:] - 2374, process=False)),
    }
    return vertices, meshes, {name: np.flatnonzero(membership == slot) for slot, name in enumerate(names)}


def measure_fit(vertices, meshes, groups):
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    targets = {"detailed": 1600, "native": 2700}
    indices = {name: grid_index(mesh) for name, (_, mesh) in meshes.items()}
    rows = {}
    for name, faces in fans.items():
        ids = groups[name]
        local = np.searchsorted(ids, faces)
        require(np.all(local < len(ids)) and np.array_equal(ids[local], faces), "fan crosses original SH054 groups")
        bound_fan(vertices[ids], local)
        rows[name] = {lod: full_upper(index, vertices[faces], targets[lod]) for lod, index in indices.items()}
    return {
        "nominalPaddingMetres": .02, "refinementTargetsMillimetres": targets,
        "fullUpperMillimetres": {lod: max(row[lod] for row in rows.values()) for lod in meshes},
        "pieces": rows,
    }


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
    parser.add_argument("--difference", action="store_true")
    args = parser.parse_args()
    vertices, meshes, groups = load()
    report = mesh_difference(vertices, meshes) if args.difference else measure_fit(vertices, meshes, groups)
    sys.stdout.buffer.write((json.dumps(report, indent=2) + "\n").encode())
