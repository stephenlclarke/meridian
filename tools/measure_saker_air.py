"""Certify Saker against both original meshes using the frozen local support fans."""

import argparse
from decimal import Decimal, localcontext
import hashlib
import json
from math import isqrt
from pathlib import Path
import sys

import numpy as np
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_saker_geometry import DETAIL, NATIVE, PIECE_COUNT, require
from vulture_mk_i_exact_fit import TriangleIndex, bound_fan, full_upper, grid_index


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/fleet/sh042'
FAN = ROOT / 'integration/sh042-saker/SUPPORT-FAN.json'


def load():
    manifest = json.loads((ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())

    def read(name):
        raw = (ASSETS / name).read_bytes()
        expected = manifest['outputs'][name]
        require(len(raw) == expected['bytes'] and hashlib.sha256(raw).hexdigest() == expected['sha256'],
                'changed SH042 audit input: ' + name)
        return raw

    vertices = np.frombuffer(read('source.vertices'), dtype='<f8', offset=16).reshape(-1, 3)
    faces = np.frombuffer(read('source-triangles.bin'), dtype='<u4').reshape(-1, 3)
    membership = np.frombuffer(read('source-piece-membership.bin'), dtype='u1')
    names = json.loads(read('PIECES.json'))['pieces']
    require(len(names) == PIECE_COUNT and len(membership) == len(vertices)
            and set(membership) == set(range(PIECE_COUNT)), 'wrong Saker audit support stream')
    groups = [np.flatnonzero(membership == slot) for slot in range(PIECE_COUNT)]
    meshes = {
        'detailed': (0, trimesh.Trimesh(vertices[:DETAIL[0]], faces[:DETAIL[1]], process=False)),
        'native': (DETAIL[0], trimesh.Trimesh(vertices[DETAIL[0]:], faces[DETAIL[1]:] - DETAIL[0], process=False)),
    }
    return vertices, names, groups, meshes


def audit():
    vertices, names, groups, meshes = load()
    fans = json.loads(FAN.read_bytes())
    require(list(fans) == names, 'wrong Saker fan registry')
    targets = {'detailed': 450, 'native': 1600}
    indices = {name: grid_index(mesh) for name, (_, mesh) in meshes.items()}
    rows = []
    for name, ids in zip(names, groups, strict=True):
        faces = np.asarray(fans[name], dtype=np.int64)
        local = np.searchsorted(ids, faces)
        require(np.all(local < len(ids)) and np.array_equal(ids[local], faces), 'Saker fan crosses its local support')
        bound_fan(vertices[ids], local)
        rows.append({'piece': name, 'fanTriangles': len(faces), 'upperMillimetres': {
            key: full_upper(index, vertices[faces], targets[key]) for key, index in indices.items()
        }})
    return {'hull': 'SH042', 'nominalPaddingMetres': .02, 'refinementTargetsMillimetres': targets,
            'fullUpperMillimetres': {k: max(row['upperMillimetres'][k] for row in rows) for k in meshes},
            'pieces': rows}


def original_difference():
    vertices, _, _, meshes = load()
    exact, scale, triangles = fit.integer_source(vertices, meshes)
    result = {}
    for key, source in [('detailed', range(DETAIL[0], DETAIL[0] + NATIVE[0])),
                        ('native', range(DETAIL[0]))]:
        index = TriangleIndex(triangles[key])
        best = None
        for vertex in source:
            (num, den), faces = index.nearest(exact[vertex])
            if best is None or num * best[1] > best[0] * den:
                best = num, den, vertex, faces
        num, den, vertex, faces = best
        denominator = den * scale**2
        lower = isqrt(num * 1_000_000 // denominator) - 1
        fit.verify_lower(exact[vertex], triangles[key], scale, lower)
        with localcontext() as context:
            context.prec = 50
            distance = float((Decimal(num) / Decimal(denominator)).sqrt())
        result[key] = {'mandatoryOppositeSourceVertex': vertex, 'nearestFace': faces[0],
                       'exactNearestFaces': faces, 'sampledMetres': distance,
                       'strictLowerMillimetres': lower}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--difference', action='store_true')
    args = parser.parse_args()
    report = original_difference() if args.difference else audit()
    sys.stdout.buffer.write((json.dumps(report, indent=2) + '\n').encode('utf-8'))


if __name__ == '__main__':
    main()
