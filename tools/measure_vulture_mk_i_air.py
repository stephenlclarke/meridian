"""Replay the measured SH037 fit and direct original-to-original difference."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_vulture_mk_i_geometry import DETAIL, NATIVE, PIECES, require


ASSETS = Path(__file__).resolve().parents[1] / 'assets/fleet/sh037'


def load():
    manifest = json.loads((ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())

    def read(name):
        data = (ASSETS / name).read_bytes()
        pin = manifest['outputs'][name]
        require(len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'],
                'changed SH037 audit input: ' + name)
        return data

    vertices = np.frombuffer(read('source.vertices'), dtype='<f8', offset=20).reshape(-1, 3)
    faces = np.frombuffer(read('source-triangles.bin'), dtype='<u4').reshape(-1, 3)
    membership = np.frombuffer(read('source-piece-membership.bin'), dtype='u1')
    names = json.loads(read('PIECES.json'))['pieces']
    require(tuple(names) == PIECES and len(membership) == len(vertices)
            and set(membership) == set(range(len(PIECES))), 'wrong SH037 audit support stream')
    detailed, native = DETAIL[0], NATIVE[0]
    real_count = detailed + native
    groups = [real_count + np.flatnonzero(membership[real_count:] == slot) for slot in range(len(PIECES))]
    meshes = {
        'detailed': (0, trimesh.Trimesh(vertices[:detailed], faces[:DETAIL[1]], process=False)),
        'originalNative': (detailed, trimesh.Trimesh(vertices[detailed:real_count],
                            faces[DETAIL[1]:DETAIL[1] + NATIVE[1]] - detailed, process=False)),
    }
    return vertices, names, groups, meshes


def audit():
    vertices, names, groups, meshes = load()
    pieces = []
    for ids, name in zip(groups, names, strict=True):
        unique, first = np.unique(vertices[ids], axis=0, return_index=True)
        try:
            piece = fit.audit_fan(unique)
        except AssertionError as error:
            raise ValueError('geometry audit could not certify ' + name) from error
        piece['sourceIds'] = ids[first]
        pieces.append(piece)
    _, scale, exact = fit.integer_source(vertices, meshes)
    target = {'detailed': 955, 'originalNative': 960}
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
    return {'hull': 'SH037', 'nominalPaddingMetres': .02, 'summary': summary, 'pieces': rows}


def original_difference():
    vertices, _, _, meshes = load()
    exact, scale, triangles = fit.integer_source(vertices, meshes)
    result = {}
    for key, source in [('detailed', range(DETAIL[0], DETAIL[0] + NATIVE[0])),
                        ('originalNative', range(DETAIL[0]))]:
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
    report = original_difference() if args.difference else audit()
    sys.stdout.buffer.write((json.dumps(report, indent=2) + '\n').encode('utf-8'))


if __name__ == '__main__':
    main()
