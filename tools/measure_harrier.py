"""Recheck the original SH026 fit witnesses and complete support bounds on every platform."""
import argparse
from fractions import Fraction
import hashlib
import json
from math import isqrt
import sys

import numpy as np

import measure_goshawk_surface_fit as fit
from measure_harrier_air import ASSETS, load
from prepare_harrier_geometry import require
from verify_harrier_geometry import INTEGRATION, verify as verify_contacts
from vulture_mk_i_exact_fit import TriangleIndex, bound_fan, full_upper, grid_index


FAN_SHA256 = "e130f5b62c2c30b478f5e244f605bd64a2223ba1a52445eb8f4fdcd3b355ff82"
AUTHOR_FIT_SHA256 = "d606d3e0b4fb71f735e63949b8fcff77b2a9f514f4c1473a994acbf4aae1a69c"


def source():
    manifest = (ASSETS / 'SOURCE-GEOMETRY.json').read_bytes()
    require(hashlib.sha256(manifest).hexdigest() == '4a1f8182079611f3fa29e66766cdd9774db87000e1ee6ea916f2df300c456aca', 'Changed Harrier source manifest')
    vertices, names, groups, meshes = load()
    require(np.abs(vertices).max() < 32, 'Coordinates exceed the certified grid range')
    return vertices, names, groups, meshes


def pinned_json(name, digest):
    raw = (INTEGRATION / name).read_bytes()
    require(hashlib.sha256(raw).hexdigest() == digest, f"Changed SH026 certificate input {name}")
    return json.loads(raw)


def certified_pieces(vertices, names, groups, fans):
    require(list(fans) == list(names), "Incomplete SH026 support fan registry")
    pieces = {}
    for name, ids in zip(names, groups, strict=True):
        faces = np.asarray(fans[name], dtype=np.int64)
        local = np.searchsorted(ids, faces)
        require(len(local) > 0 and np.all(local < len(ids)) and np.array_equal(ids[local], faces),
                f"Wrong source group in SH026 fan {name}")
        bound_fan(vertices[ids], local)
        triangles = vertices[faces]
        normals = np.cross(triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 0])
        normals /= np.linalg.norm(normals, axis=1)[:, None]
        pieces[name] = {"vertices": vertices[ids], "sourceIds": ids, "normals": normals,
                        "triangles": triangles}
    return pieces


def exposed_lower(summary, pieces, triangles, scale):
    # Replay one authored barycentric witness. The exact lower proof tests every
    # original triangle and strict separation from every other rounded piece.
    authored = summary["largestSample"]
    vertex = authored["supportFacetGlobalVertices"][0]
    (piece,) = (group for group in pieces.values() if vertex in group["sourceIds"])
    ids = np.searchsorted(piece["sourceIds"], authored["supportFacetGlobalVertices"])
    weights = authored["supportFacetWeightNumeratorsOver12"]
    require(np.all(ids < len(piece["sourceIds"])) and
            piece["sourceIds"][ids].tolist() == authored["supportFacetGlobalVertices"],
            "Witness facet crosses SH026 groups")
    support = piece["vertices"][ids]
    cloud, _ = fit.dyadic(np.vstack((support, piece["vertices"])))
    normal = fit.cross(fit.sub(cloud[1], cloud[0]), fit.sub(cloud[2], cloud[0]))
    require(any(normal) and all(fit.dot(normal, fit.sub(p, cloud[0])) <= 0 for p in cloud[3:]),
            "Witness facet does not support SH026")
    require(min(weights) >= 0 and sum(weights) == 12, "Invalid SH026 witness weights")
    samples = np.asarray([authored["point"]]), np.asarray([ids]), np.asarray([weights])
    result = {"metres": authored["metres"]}
    fit.certify_exposed_sample(result, 0, samples, piece, pieces.values(), (triangles, scale))
    return result["certifiedLowerMillimetres"]


def piece_upper(index, triangles, target):
    bound = full_upper(index, triangles, target)
    if bound <= target:
        return bound
    # Split twice before the standard integer refinement, as in the depth-eight
    # author audit. All coordinates are below 32 m. A further millimetre
    # covers both binary64 midpoint roundings before the usual grid allowance.
    refined = fit.split(fit.split(triangles))
    return min(bound, full_upper(index, refined, target - 1) + 1)


def surface_fit():
    vertices, names, groups, meshes = source()
    fans = pinned_json("SUPPORT-FAN.json", FAN_SHA256)
    pieces = certified_pieces(vertices, names, groups, fans)
    original = pinned_json("SURFACE-FIT.json", AUTHOR_FIT_SHA256)["summary"]
    _, scale, triangles = fit.integer_source(vertices, meshes)
    result = {}
    for lod, (_, mesh) in meshes.items():
        target = original[lod]["fullUpperMillimetres"]
        index = grid_index(mesh)
        upper = {name: piece_upper(index, piece["triangles"], target) for name, piece in pieces.items()}
        require(max(upper.values()) <= target, f"Uncertified SH026 {lod} upper bound")
        result[lod] = {"strictExposedLowerMillimetres": exposed_lower(original[lod], pieces, triangles[lod], scale),
                       "completeUpperMillimetres": max(upper.values()), "pieceUpperMillimetres": upper}
    return result


def original_difference():
    vertices, _, _, meshes = source()
    exact, scale, triangles = fit.integer_source(vertices, meshes)
    split = meshes["native"][0]
    result = {}
    for target, ids in (("detailed", range(split, len(vertices))), ("native", range(split))):
        index = TriangleIndex(triangles[target])
        def candidate(vertex):
            distance, faces = index.nearest(exact[vertex])
            return Fraction(*distance), vertex, faces
        distance, vertex, faces = max(map(candidate, ids), key=lambda row: row[0])
        metres_squared = distance / scale**2
        lower = isqrt(metres_squared.numerator * 1_000_000 // metres_squared.denominator) - 1
        fit.verify_lower(exact[vertex], triangles[target], scale, lower)
        result[target] = {"oppositeSourceVertex": vertex, "nearestFaces": faces,
                          "strictLowerMillimetres": lower}
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--difference", action="store_true")
    mode.add_argument("--contact", action="store_true")
    args = parser.parse_args()
    if args.contact:
        report = verify_contacts()
    elif args.difference:
        report = original_difference()
    else:
        report = surface_fit()
    sys.stdout.buffer.write((json.dumps(report, indent=2) + "\n").encode("utf-8"))
