"""Recheck the Scout support fans and source distance witnesses in original metres."""

from fractions import Fraction
import hashlib
import json
from math import isqrt
import sys

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_panther_scout_studio import ASSETS, OUTPUT, authenticated, definition, require
from vulture_mk_i_exact_fit import TriangleIndex, bound_fan, full_upper, grid_index


def geometry():
    resource, _ = authenticated(ASSETS)
    package = definition()
    require(package == json.loads(OUTPUT.read_bytes()), 'Changed Scout support package')
    detail, native = resource['design'], resource['nativeDesign']
    vertices = np.asarray(detail['vertices'] + native['vertices'])
    require(np.abs(vertices).max() < 32, 'Coordinate exceeds the grid certificate range')
    meshes = {lod: (offset, trimesh.Trimesh(model['vertices'], model['faces'], process=False))
              for lod, offset, model in [('detailed', 0, detail), ('native', len(detail['vertices']), native)]}
    first = {}
    for i, point in enumerate(vertices):
        first.setdefault(tuple(point), i)
    # Repeated arch seam points can belong to multiple groups. Keep that membership.
    names = package['provenance']['groupNames']
    groups = [set() for _ in names]
    for mesh in (package['detail'], package['native']):
        for point, group in zip(mesh['vertices'], mesh['groups'], strict=True):
            groups[group].add(first[tuple(point)])
    return vertices, meshes, {name: np.array(sorted(ids)) for name, ids in zip(names, groups, strict=True)}


def receipt(name, sha):
    raw = OUTPUT.with_name(name).read_bytes()
    require(hashlib.sha256(raw).hexdigest() == sha, f'Changed Scout certificate input {name}')
    return json.loads(raw)


def pieces_from_fans(vertices, groups, fans):
    require(fans.keys() == groups.keys(), 'Incomplete Scout support groups')
    pieces = {}
    for name, ids in groups.items():
        faces = np.asarray(fans[name], dtype=np.int64)
        local = np.searchsorted(ids, faces)
        require(np.all(local < len(ids)) and np.array_equal(ids[local], faces), 'Wrong Scout fan membership')
        bound_fan(vertices[ids], local)
        triangles = vertices[faces]
        normals = np.cross(triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 0])
        normals /= np.linalg.norm(normals, axis=1)[:, None]
        pieces[name] = {'sourceIds': ids, 'vertices': vertices[ids], 'triangles': triangles, 'normals': normals}
    return pieces


def exposed_lower(lod, archive, pieces, exact_faces, scale):
    witness = archive['summary'][lod]['largestSample']
    (name,) = (row['group'] for row in archive['perGroup']
               if row['comparisons'][lod]['largestExposedSample'] == witness)
    piece = pieces[name]
    source_ids = witness['supportFacetGlobalVertices']
    indices = np.searchsorted(piece['sourceIds'], source_ids)
    require(np.all(indices < len(piece['sourceIds'])) and piece['sourceIds'][indices].tolist() == source_ids,
            'Wrong Scout witness membership')
    weights = witness['supportFacetWeightNumeratorsOver12']
    require(len(weights) == 3 and min(weights) >= 0 and sum(weights) == 12, 'Invalid witness weights')
    cloud, _ = fit.dyadic(np.vstack((piece['vertices'][indices], piece['vertices'])))
    normal = fit.cross(fit.sub(cloud[1], cloud[0]), fit.sub(cloud[2], cloud[0]))
    require(any(normal) and all(fit.dot(normal, fit.sub(p, cloud[0])) <= 0 for p in cloud[3:]),
            'Witness lies outside its source support plane')
    samples = np.array([witness['point']]), np.array([indices]), np.array([weights])
    row = {'metres': witness['metres']}
    fit.certify_exposed_sample(row, 0, samples, piece, pieces.values(), (exact_faces, scale))
    return {'group': name, 'millimetres': row['certifiedLowerMillimetres']}


def audit():
    vertices, meshes, groups = geometry()
    fans = receipt('SUPPORT-FAN.json', 'dfd7669c7d16685c42a73f57956b31045b8a9d27b2951a4cc870bd3a6fdf8b54')
    archive = receipt('SURFACE-FIT.json', 'c68ceb9c8fa9c2140f4fc7f4f09cca1332ca056214c1e98d22b4c36fdf61ec6d')
    pieces = pieces_from_fans(vertices, groups, fans)
    exact, scale, triangles = fit.integer_source(vertices, meshes)
    result = {}
    for lod, (_, mesh) in meshes.items():
        upper = archive['summary'][lod]['fullUpperMillimetres']
        grid = grid_index(mesh)
        bounds = {name: full_upper(grid, piece['triangles'], upper) for name, piece in pieces.items()}
        require(max(bounds.values()) <= upper, f'Scout {lod} exceeds the archived upper bound')
        source = archive['originalCrossSourceDiscrepancy'][lod]['oppositeSourceGlobalVertex']
        require(0 <= source < len(vertices) and (source < meshes['native'][0]) == (lod == 'native'),
                'Wrong original opposite-mesh witness')
        squared, faces = TriangleIndex(triangles[lod]).nearest(exact[source])
        distance = Fraction(*squared) / scale**2
        lower = isqrt(distance.numerator * 1_000_000 // distance.denominator) - 1
        fit.verify_lower(exact[source], triangles[lod], scale, lower)
        result[lod] = {'exposedLower': exposed_lower(lod, archive, pieces, triangles[lod], scale),
                       'completeUpperMillimetres': max(bounds.values()), 'pieceUpperMillimetres': bounds,
                       'oppositeMeshWitness': {'sourceVertex': source, 'nearestFaces': faces,
                                               'strictLowerMillimetres': lower}}
    return result


if __name__ == '__main__':
    sys.stdout.buffer.write((json.dumps(audit(), indent=2) + '\n').encode())
