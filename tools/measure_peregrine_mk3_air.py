"""Replay the measured SH023 fit and direct original-to-original difference."""
import argparse
import hashlib
import json
from collections import Counter
from itertools import combinations, pairwise
from pathlib import Path

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_peregrine_mk3_geometry import DETAIL, NATIVE, PIECE_COUNT, require


ASSETS = Path(__file__).resolve().parents[1] / 'assets/fleet/sh023'


def load():
    manifest = json.loads((ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())

    def read(name):
        data = (ASSETS / name).read_bytes()
        pin = manifest['outputs'][name]
        require(len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'],
                'changed SH023 audit input: ' + name)
        return data

    vertices = np.frombuffer(read('source.vertices'), dtype='<f8', offset=16).reshape(-1, 3)
    faces = np.frombuffer(read('source-triangles.bin'), dtype='<u4').reshape(-1, 3)
    membership = np.frombuffer(read('source-piece-membership.bin'), dtype='u1')
    names = json.loads(read('PIECES.json'))['pieces']
    require(len(names) == PIECE_COUNT and len(membership) == len(vertices)
            and set(membership) == set(range(PIECE_COUNT)), 'wrong SH023 audit support stream')
    groups = [np.flatnonzero(membership == slot) for slot in range(PIECE_COUNT)]
    points, tris = DETAIL[:2]
    meshes = {
        'detailed': (0, trimesh.Trimesh(vertices[:points], faces[:tris], process=False)),
        'native': (points, trimesh.Trimesh(vertices[points:], faces[tris:] - points, process=False)),
    }
    return vertices, names, groups, meshes


def exact_small_fan(vertices):
    # The two tilted eight-point engine brackets have nearly coplanar face pairs.
    # Enumerate its exact binary64 support facets and verify their closed outward fan.
    assert len(vertices) == 8
    points, _ = fit.dyadic(vertices)
    center = tuple(map(sum, zip(*points)))
    faces, normals = [], []
    edges = Counter()
    for a, b, c in combinations(range(len(points)), 3):
        normal = fit.cross(fit.sub(points[b], points[a]), fit.sub(points[c], points[a]))
        h = fit.dot(normal, points[a])
        offset = h * len(points) - fit.dot(normal, center)
        if not offset:
            continue
        if offset < 0:
            b, c = c, b
            normal = tuple(-x for x in normal)
            h = -h
        if max(fit.dot(normal, point) for point in points) != h:
            continue
        unit = np.asarray(normal, dtype=float)
        normals.append(unit / np.linalg.norm(unit))
        faces.append((a, b, c))
        for edge in pairwise((a, b, c, a)):
            edges[edge] += 1
    assert faces and all(count == edges[(b, a)] == 1 for (a, b), count in edges.items())
    normals = np.array(normals)
    return {
        'vertices': vertices,
        'triangles': vertices[faces],
        'supportFaces': list(zip(faces, normals, strict=True)),
        'normals': normals,
        'offsets': (vertices @ normals.T).max(axis=0),
        'min': vertices.min(axis=0) - fit.RADIUS,
        'max': vertices.max(axis=0) + fit.RADIUS,
    }

def audit():
    vertices, names, groups, meshes = load()
    pieces = []
    for ids, name in zip(groups, names, strict=True):
        unique, first = np.unique(vertices[ids], axis=0, return_index=True)
        try:
            if name in ('159:engine_pair/negative side/connected support',
                        '162:engine_pair/positive side/connected support'):
                piece = exact_small_fan(unique)
            else:
                piece = fit.audit_fan(unique)
        except AssertionError as error:
            raise ValueError('geometry audit could not certify ' + name) from error
        piece['sourceIds'] = ids[first]
        pieces.append(piece)
    _, scale, exact = fit.integer_source(vertices, meshes)
    target = {'detailed': 825, 'native': 1500}
    witnesses = {key: [] for key in meshes}
    rows = []
    for name, piece in zip(names, pieces, strict=True):
        samples = fit.exposed_samples(piece, pieces)
        comparisons = {}
        for key, (_, mesh) in meshes.items():
            row, index = fit.worst_sample(mesh, samples[0])
            if row is not None:
                witnesses[key].append((row, index, samples, piece))
            comparisons[key] = {
                'largestExposedSample': row,
                'fullPieceSurfaceUpperMillimetres': fit.certified_upper(mesh, piece, target[key]),
            }
        rows.append({'piece': name, 'exposedSamples': len(samples[0]), 'sourceComparisons': comparisons})
    summary = {}
    for key, candidates in witnesses.items():
        first = sorted(candidates, key=lambda row: -row[0]['metres'])[:2]
        for row, index, samples, piece in first:
            fit.certify_exposed_sample(row, index, samples, piece, pieces, (exact[key], scale))
        summary[key] = {
            'largestSample': first[0][0],
            'fullUpperMillimetres': max(row['sourceComparisons'][key]['fullPieceSurfaceUpperMillimetres']
                                       for row in rows),
        }
    return {'hull': 'SH023', 'nominalPaddingMetres': .02, 'summary': summary, 'pieces': rows}


def original_difference():
    vertices, _, _, meshes = load()
    exact, scale, triangles = fit.integer_source(vertices, meshes)
    result = {}
    for key, source in [('detailed', range(DETAIL[0], DETAIL[0] + NATIVE[0])),
                        ('native', range(DETAIL[0]))]:
        ids = np.asarray(source)
        _, distances, faces = fit.nearest(meshes[key][1], vertices[ids])
        at = distances.argmax()
        vertex = int(ids[at])
        lower = int(distances[at] * 1000) - 1
        fit.verify_lower(exact[vertex], triangles[key], scale, lower)
        result[key] = {'mandatoryOppositeSourceVertex': vertex, 'nearestFace': int(faces[at]),
                       'sampledMetres': float(distances[at]), 'strictLowerMillimetres': lower}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--difference', action='store_true')
    args = parser.parse_args()
    print(json.dumps(original_difference() if args.difference else audit(), indent=2))


if __name__ == '__main__':
    main()
