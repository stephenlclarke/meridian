"""Bound the exposed SH036 rounded union against both original LOD surfaces."""
import argparse
import hashlib
import json
import math
import struct

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_zebu_studio import ASSETS, COUNTS, NAMES, require

MANIFEST_SHA = '95a9e844052933b80738c342a6ce56eaf2eba238a72f3ac6a13c103000729a95'
REFINEMENT_DEPTH = 8


def load(folder=ASSETS):
    raw = (folder / 'SOURCE-GEOMETRY.json').read_bytes()
    require(hashlib.sha256(raw).hexdigest() == MANIFEST_SHA, 'Changed SH036 manifest')
    manifest = json.loads(raw)

    def read(name):
        data = (folder / name).read_bytes()
        pin = manifest['provenance']['derivedHashes'][name]
        require(len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], f'Changed SH036 fit input: {name}')
        return data

    source = read('source.vertices')
    dv, df = COUNTS[0][:2]
    nv, nf = COUNTS[1][:2]
    require(source[:16] == struct.pack('<8sII', b'MSH036V1', dv, nv), 'Wrong SH036 source stream')
    vertices = np.frombuffer(source, dtype='<f8', offset=16).reshape(-1, 3)
    faces = np.frombuffer(read('source-triangles.bin'), dtype='<u4').reshape(-1, 3)
    membership = np.frombuffer(read('source-piece-membership.bin'), dtype='u1')
    names = json.loads(read('PIECES.json'))['pieces']
    require(tuple(names) == NAMES and vertices.shape == (dv+nv,3) and faces.shape == (df+nf,3) and len(membership) == len(vertices) and set(membership) == set(range(len(names))), 'Wrong SH036 support or mesh counts')
    require(np.all(faces[:df] < dv) and np.all(faces[df:] >= dv) and np.all(membership[faces] == membership[faces[:, :1]]), 'Mixed geometry source or support')
    require(np.max(np.abs(vertices)) < 32, 'Coordinate range exceeds exact grid audit')
    groups = [np.flatnonzero(membership == slot) for slot in range(len(names))]
    meshes = {'detailed': (0, trimesh.Trimesh(vertices[:dv], faces[:df], process=False)), 'native': (dv, trimesh.Trimesh(vertices[dv:], faces[df:]-dv, process=False))}
    return vertices, names, groups, meshes


def build_pieces(vertices, names, groups):
    pieces = []
    for ids, name in zip(groups, names, strict=True):
        unique, first = np.unique(vertices[ids], axis=0, return_index=True)
        try:
            piece = fit.audit_fan(unique)
        except AssertionError as error:
            raise ValueError('Could not certify radial fan for ' + name) from error
        piece['sourceIds'] = ids[first]
        pieces.append(piece)
    return pieces


def refinement_targets(pieces, meshes):
    """Choose economical cutoffs from all unrounded fan samples, including hidden patches.

    Cutoff controls refinement only. It is never presented as a certificate.
    """
    estimate = {key: 0. for key in meshes}
    for piece in pieces:
        triangles = fit.split(fit.split(piece['triangles']))
        probes = np.concatenate((triangles.mean(axis=1), piece['vertices']))
        for key, (_, mesh) in meshes.items():
            _, distances, _ = fit.nearest(mesh, probes)
            estimate[key] = max(estimate[key], float(distances.max()))
    return {key: math.ceil(value * 1000) + 45 for key, value in estimate.items()}


def certified_zebu_upper(mesh, piece, target):
    # A tighter bound needs two further refinement levels on the large hump.
    # Keep the shared, hash-pinned SH008 certificate implementation unchanged.
    original_depth = fit.DEPTH
    try:
        fit.DEPTH = REFINEMENT_DEPTH
        return fit.certified_upper(mesh, piece, target)
    finally:
        fit.DEPTH = original_depth


def audit(samples_only=False):
    vertices, names, groups, meshes = load()
    pieces = build_pieces(vertices, names, groups)
    _, scale, exact = fit.integer_source(vertices, meshes)
    targets = refinement_targets(pieces, meshes)
    witnesses = {key: [] for key in meshes}
    rows = []
    for name, piece in zip(names, pieces, strict=True):
        samples = fit.exposed_samples(piece, pieces)
        comparisons = {}
        for key, (_, mesh) in meshes.items():
            row, index = fit.worst_sample(mesh, samples[0])
            if row is not None:
                witnesses[key].append((row, index, samples, piece))
            comparisons[key] = {'largestExposedSample': row}
            if not samples_only:
                comparisons[key]['fullPieceSurfaceUpperMillimetres'] = certified_zebu_upper(mesh, piece, targets[key])
        rows.append({'piece': name, 'exposedSamples': len(samples[0]), 'sourceComparisons': comparisons})
    summary = {}
    for key, candidates in witnesses.items():
        require(bool(candidates), f'No exposed SH036 sample for {key}')
        first = sorted(candidates, key=lambda item: -item[0]['metres'])[:2]
        for row, index, samples, piece in first:
            fit.certify_exposed_sample(row, index, samples, piece, pieces, (exact[key], scale))
        summary[key] = {'largestSample': first[0][0]}
        if not samples_only:
            summary[key]['fullUpperMillimetres'] = max(row['sourceComparisons'][key]['fullPieceSurfaceUpperMillimetres'] for row in rows)
    return {'hull': 'SH036', 'nominalPaddingMetres': .02, 'refinementDepth': REFINEMENT_DEPTH, 'refinementCutoffMillimetres': targets, 'samplesOnly': samples_only, 'summary': summary, 'pieces': rows}


def original_difference():
    """Report certified witness of LOD difference; not a global symmetric distance."""
    vertices, _, _, meshes = load()
    exact, scale, triangles = fit.integer_source(vertices, meshes)
    dv = COUNTS[0][0]
    result = {}
    for key, source in [('detailed', range(dv, len(vertices))), ('native', range(dv))]:
        ids = np.asarray(source)
        _, distances, faces = fit.nearest(meshes[key][1], vertices[ids])
        at = int(distances.argmax())
        vertex = int(ids[at])
        lower = int(distances[at] * 1000) - 1
        fit.verify_lower(exact[vertex], triangles[key], scale, lower)
        result[key] = {'mandatoryOppositeSourceVertex': vertex, 'nearestFace': int(faces[at]), 'sampledMetres': float(distances[at]), 'strictLowerMillimetres': lower}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--difference', action='store_true', help='Direct original-to-original vertex distance witness')
    parser.add_argument('--samples-only', action='store_true', help='Diagnostic exposed samples without upper bound')
    args = parser.parse_args()
    print(json.dumps(original_difference() if args.difference else audit(args.samples_only), indent=2))


if __name__ == '__main__':
    main()
