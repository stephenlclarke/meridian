"""Measure the original Dzo Mk II mesh difference and rounded support fit."""

import argparse
from decimal import Decimal, localcontext
import json
from math import isqrt
import sys

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_dzo_mk_ii import INTEGRATION, ORIGINAL, prepare, require
from measure_banteng_geometry import adaptive_upper
from measure_lanner_geometry import make_fan
from dzo_collar_support import COLLARS, collar_spans
from vulture_mk_i_exact_fit import TriangleIndex, bound_fan, grid_index


def load():
    derived = prepare()
    for name in ("source.vertices", "source-triangles.bin", "source-piece-membership.bin"):
        require((ORIGINAL / name).read_bytes() == derived[name], f"changed SH075 fit input: {name}")
    vertices = np.frombuffer(derived["source.vertices"], dtype="<f8", offset=16).reshape(-1, 3)
    faces = np.frombuffer(derived["source-triangles.bin"], dtype="<u4").reshape(-1, 3)
    membership = np.frombuffer(derived["source-piece-membership.bin"], dtype="u1")
    names = json.loads(derived["PIECES.json"])["pieces"]
    meshes = {
        "detailed": (0, trimesh.Trimesh(vertices[:4862], faces[:9612], process=False)),
        "native": (4862, trimesh.Trimesh(vertices[4862:], faces[9612:] - 4862, process=False)),
    }
    return vertices, meshes, {name: np.flatnonzero(membership == slot) for slot, name in enumerate(names)}


def measure_fit(vertices, meshes, groups):
    fans = json.loads((INTEGRATION / "SUPPORT-FAN.json").read_bytes())
    targets = {"detailed": 3900, "native": 5100}
    indices = {name: grid_index(mesh) for name, (_, mesh) in meshes.items()}
    rows = {}
    for name, faces in fans.items():
        ids = groups[name]
        local = np.searchsorted(ids, faces)
        require(np.all(local < len(ids)) and np.array_equal(ids[local], faces), "fan crosses original SH075 groups")
        bound_fan(vertices[ids], local)
        rows[name] = {lod: adaptive_upper(index, vertices[faces], targets[lod]) for lod, index in indices.items()}
    return {
        "nominalPaddingMetres": .02, "minimumRefinementTargetsMillimetres": targets,
        "fullUpperMillimetres": {lod: max(row[lod] for row in rows.values()) for lod in meshes},
        "pieces": rows,
    }


def collar_options(vertices, meshes, groups):
    baseline = json.loads((INTEGRATION / "SURFACE-FIT.json").read_bytes())
    targets = baseline["minimumRefinementTargetsMillimetres"]
    indices = {lod: grid_index(mesh) for lod, (_, mesh) in meshes.items()}
    collars = COLLARS
    other = {
        lod: max(piece[lod] for name, piece in baseline["pieces"].items() if name not in collars)
        for lod in indices
    }
    options = {}
    for name in collars:
        archive = groups[name]
        points = (archive[archive < 4862], archive[archive >= 4862])
        options[name] = {"intactUpperMillimetres": baseline["pieces"][name]}
        for count in (4, 8):
            spans = collar_spans(archive, 4862, count)
            memberships = [set(span) for span in spans]
            for (offset, mesh), lod_points in zip(meshes.values(), points, strict=True):
                collars_faces = mesh.faces[np.isin(mesh.faces + offset, lod_points).all(1)]
                expected = 1536 if offset == 0 else 64
                require(len(collars_faces) == expected, "changed Dzo Mk II collar triangle count")
                require(all(any(set(face + offset) <= span for span in memberships) for face in collars_faces),
                        "uncovered Dzo Mk II collar triangle")
            fans = [make_fan(vertices, np.asarray(span)) for span in spans]
            options[name][f"{count}SpansUpperMillimetres"] = {
                lod: max(adaptive_upper(index, vertices[fan], targets[lod]) for fan in fans)
                for lod, index in indices.items()
            }
    return {"unchangedOtherSupportsUpperMillimetres": other, "collars": options}


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
        if lower >= 0:
            fit.verify_lower(exact[chosen], triangles[target], scale, lower)
        with localcontext() as context:
            context.prec = 50
            distance = float((Decimal(best_num) / Decimal(denominator)).sqrt())
        results[target] = {"vertex": chosen, "metres": distance, "faces": nearest, "strictLowerMillimetres": lower if lower >= 0 else None}
    return results


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    report_mode = parser.add_mutually_exclusive_group()
    report_mode.add_argument("--difference", action="store_true")
    report_mode.add_argument("--collar-options", action="store_true")
    args = parser.parse_args()
    vertices, meshes, groups = load()
    if args.difference:
        report = mesh_difference(vertices, meshes)
    else:
        report = (collar_options if args.collar_options else measure_fit)(vertices, meshes, groups)
    sys.stdout.buffer.write((json.dumps(report, indent=2) + "\n").encode())
