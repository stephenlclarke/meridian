"""Recheck the original SH032 fit witnesses and complete support bounds on every platform."""
import argparse
from fractions import Fraction
import hashlib
import json
from math import isqrt
import sys

import numpy as np

import measure_goshawk_surface_fit as fit
from measure_panther_explorer_air import ASSETS, load
from prepare_panther_explorer_geometry import DETAIL, NATIVE, SUPPORT_VERTICES, require
from prepare_panther_explorer_studio import OUTPUT
from verify_panther_explorer_geometry import verify as verify_contacts
from vulture_mk_i_exact_fit import TriangleIndex, bound_fan, full_upper, grid_index


INTEGRATION = OUTPUT.parent
FAN_SHA256 = '6cada83aa00b416a378f10886393d5e210170dbe30783f5539818b9f2f142e65'
SOURCE_SHA256 = '16ececf28c3263984ce990bd44534bc60821388307226aa08a0bd116577fc4c7'
REPORTS = {'split': ('SURFACE-FIT.json', '58912c92a5317c881dc014160e65be4b58a9a7564a86b1f1e7a1cc77f8674da4'),
           'whole': ('UNSPLIT-FIT.json', 'cfcdd453110367032a672a209ebe841beb4d1a096065677da378406d59c0a71c')}


def geometry(mode):
    require(hashlib.sha256((ASSETS / 'SOURCE-GEOMETRY.json').read_bytes()).hexdigest() == SOURCE_SHA256,
            'Changed Explorer source manifest')
    vertices, names, ids, meshes = load()
    require(np.abs(vertices).max() < 32, 'Coordinate exceeds the fixed grid audit range')
    groups = {}
    for name, group in zip(names, ids, strict=True):
        if mode == 'whole' and '-curved-support-' in name:
            name = name.split('-curved-support-', 1)[0] + '-curved-support'
        groups.setdefault(name, []).extend(group.tolist())
    return vertices, {name: np.asarray(sorted(points)) for name, points in groups.items()}, meshes


def pinned_json(name, digest):
    raw = (INTEGRATION / name).read_bytes()
    require(hashlib.sha256(raw).hexdigest() == digest, f"Changed SH032 certificate input {name}")
    return json.loads(raw)


def certified_pieces(vertices, groups, fans):
    require(list(fans) == list(groups), "Incomplete SH032 support fan registry")
    pieces = {}
    for name, ids in groups.items():
        faces = np.asarray(fans[name], dtype=np.int64)
        local = np.searchsorted(ids, faces)
        require(len(local) > 0 and np.all(local < len(ids)) and np.array_equal(ids[local], faces),
                f"Wrong source group in SH032 fan {name}")
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
            "Witness facet crosses SH032 groups")
    support = piece["vertices"][ids]
    cloud, _ = fit.dyadic(np.vstack((support, piece["vertices"])))
    normal = fit.cross(fit.sub(cloud[1], cloud[0]), fit.sub(cloud[2], cloud[0]))
    require(any(normal) and all(fit.dot(normal, fit.sub(p, cloud[0])) <= 0 for p in cloud[3:]),
            "Witness facet does not support SH032")
    require(min(weights) >= 0 and sum(weights) == 12, "Invalid SH032 witness weights")
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


def surface_fit(mode):
    vertices, groups, meshes = geometry(mode)
    fans = pinned_json("SUPPORT-FAN.json", FAN_SHA256)[mode]
    pieces = certified_pieces(vertices, groups, fans)
    original = pinned_json(*REPORTS[mode])["summary"]
    _, scale, triangles = fit.integer_source(vertices, meshes)
    result = {}
    for lod, (_, mesh) in meshes.items():
        target = original[lod]["fullUpperMillimetres"]
        index = grid_index(mesh)
        upper = {name: piece_upper(index, piece["triangles"], target) for name, piece in pieces.items()}
        require(max(upper.values()) <= target, f"Uncertified SH032 {lod} upper bound")
        result[lod] = {"strictExposedLowerMillimetres": exposed_lower(original[lod], pieces, triangles[lod], scale),
                       "completeUpperMillimetres": max(upper.values()), "pieceUpperMillimetres": upper}
    return result


def original_difference():
    vertices, _, meshes = geometry("split")
    exact, scale, triangles = fit.integer_source(vertices, meshes)
    split = meshes["native"][0]
    result = {}
    for target, ids in (("detailed", range(split, split + NATIVE[0])), ("native", range(DETAIL[0]))):
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
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--difference", action="store_true")
    modes.add_argument("--unsplit", action="store_true")
    modes.add_argument("--contact", action="store_true")
    args = parser.parse_args()
    if args.contact:
        report = verify_contacts()
    elif args.difference:
        report = original_difference()
    else:
        report = surface_fit("whole" if args.unsplit else "split")
    sys.stdout.buffer.write((json.dumps(report, indent=2) + "\n").encode("utf-8"))
