"""Bound SH033 exposed rounded-envelope fit against both original Scout meshes."""

import hashlib
import json
from pathlib import Path

import numpy as np
import scipy
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_panther_scout_studio import ASSETS, OUTPUT, authenticated, definition, require


TARGET_MM = {'detailed': 600, 'native': 2200}


def audit():
    """Independent source triangles for distances; exact support certificates for bounds."""
    source, _ = authenticated(ASSETS)
    package = definition()
    require(package == json.loads(OUTPUT.read_bytes()), 'Changed Scout package')
    detailed, native = source['design'], source['nativeDesign']
    original = np.asarray(detailed['vertices'] + native['vertices'], dtype=np.float64)
    meshes = {
        'detailed': (0, trimesh.Trimesh(detailed['vertices'], detailed['faces'], process=False)),
        'native': (len(detailed['vertices']), trimesh.Trimesh(native['vertices'], native['faces'], process=False)),
    }
    # The collision package may duplicate a seam index. Map each support point
    # to its first identical coordinate in the independent original input.
    ids = {}
    for index, point in enumerate(original):
        ids.setdefault(tuple(point), index)
    names = package['provenance']['groupNames']
    groups = [set() for _ in names]
    for mesh in (package['detail'], package['native']):
        for point, group in zip(mesh['vertices'], mesh['groups'], strict=True):
            groups[group].add(tuple(point))
    pieces = []
    for group, name in zip(groups, names, strict=True):
        unique = np.asarray(sorted(group))
        try:
            piece = fit.audit_fan(unique)
        except (AssertionError, ValueError) as error:
            raise ValueError('Unable to certify Scout source support ' + name) from error
        piece['sourceIds'] = np.asarray([ids[tuple(point)] for point in unique])
        pieces.append(piece)
    integer_points, scale, exact = fit.integer_source(original, meshes)
    witnesses = {key: [] for key in meshes}
    rows = []
    for name, piece in zip(names, pieces, strict=True):
        samples = fit.exposed_samples(piece, pieces)
        comparisons = {}
        for key, (_, mesh) in meshes.items():
            row, index = fit.worst_sample(mesh, samples[0])
            if row is not None:
                witnesses[key].append((row, index, samples, piece))
            comparisons[key] = {'largestExposedSample': row,
                                'fullRoundedPieceUpperMillimetres': fit.certified_upper(mesh, piece, TARGET_MM[key])}
        rows.append({'group': name, 'exposedSamples': len(samples[0]), 'comparisons': comparisons})
    summary = {}
    for key, candidates in witnesses.items():
        largest = sorted(candidates, key=lambda row: -row[0]['metres'])[:2]
        require(bool(largest), 'No exposed surface candidate')
        for row, index, samples, piece in largest:
            fit.certify_exposed_sample(row, index, samples, piece, pieces, (exact[key], scale))
        upper = max(row['comparisons'][key]['fullRoundedPieceUpperMillimetres'] for row in rows)
        summary[key] = {'largestSample': largest[0][0], 'fullUpperMillimetres': upper,
                        'refinementTargetMillimetres': TARGET_MM[key]}
    difference = {}
    for target, source_indices in [('detailed', range(len(detailed['vertices']), len(original))),
                                   ('native', range(len(detailed['vertices'])))]:
        indices = np.asarray(source_indices)
        _, distances, faces = fit.nearest(meshes[target][1], original[indices])
        at = int(np.argmax(distances))
        source_id = int(indices[at])
        lower = int(distances[at]*1000) - 1
        require(lower > 0, 'Missing original cross-source discrepancy')
        fit.verify_lower(integer_points[source_id], exact[target], scale, lower)
        difference[target] = {'oppositeSourceGlobalVertex': source_id, 'nearestOriginalFace': int(faces[at]),
                              'distanceMetres': float(distances[at]), 'strictLowerMillimetres': lower}
    return {'hull': 'SH033', 'packageSHA256': hashlib.sha256(OUTPUT.read_bytes()).hexdigest(),
            'nominalPaddingMetres': .02, 'groups': len(pieces), 'summary': summary, 'perGroup': rows,
            'originalCrossSourceDiscrepancy': difference,
            'originalSourceHashes': package['provenance']['sourceHashes'],
            'auditKernelSHA256': hashlib.sha256(Path(fit.__file__).read_bytes()).hexdigest(),
            'scientificVersions': {'numpy': np.__version__, 'scipy': scipy.__version__, 'trimesh': trimesh.__version__},
            'scope': 'Exact exposed rounded sample lower witnesses; full rounded piece upper bounds cover the union surface including overlaps, so they are conservative.'}


if __name__ == '__main__':
    print(json.dumps(audit(), indent=2))
