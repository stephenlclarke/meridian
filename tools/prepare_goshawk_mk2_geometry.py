"""Authenticate and encode only the original SH009 detailed and native-candidate geometry."""
import argparse
from decimal import Decimal
from fractions import Fraction
from functools import partial
import hashlib
import json
from math import isfinite
from pathlib import Path
import struct
import tomllib

from import_goshawk_envelope_source import components


ASSETS = Path(__file__).resolve().parents[1] / 'assets/fleet/sh009'
LEGACY_REVISION = 'a11fe16924d87d06021816f9037952afcbac08db'
PINS = {
    'resource.toml': (
        'Sources/L337Core/Resources/Ships/SH009.toml',
        '4fc835b6f40c1b7751ff8c03940c0501ce7c445b1bd0be95f15651a174c67552',
    ),
    'native-source.json': (
        'Sources/L337Core/Resources/OriginalFleetShowroomModels/SH009.json',
        'bb4b2f9bf1b5ad045feceb3692fabfa81334d1cd559bfd54447cd5e625af7abe',
    ),
    'native-export-manifest.json': (
        'Sources/L337Core/Resources/OriginalFleetShowroomModels/manifest.json',
        '118b05757dc54096faa15dd79801f38999b87fe9424f1b3513133ead94f71971',
    ),
    'contract.toml': (
        'docs/fleet-design-v2/ships/SH009.toml',
        '8487d3b24cff75389b9451eb6a3e9f041c907df16caf696a67cd31f46feec670',
    ),
    'LICENSE': (
        'LICENSE',
        '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
    ),
}
NATIVE_SCALE = Fraction(30_000_000, 3_186_991)
SOURCE_PROPOSAL = 'replacement-hulls/SH009-design.toml'
SOURCE_PROPOSAL_SHA = '41cda75d40e41e4de1a745de9f5c8770618fe5de2f0df0568a49a8c72ba92722'


def check(test, message):
    if not test:
        raise ValueError(message)


def pin(blob):
    return {'bytes': len(blob), 'sha256': hashlib.sha256(blob).hexdigest()}


def json_bytes(value):
    return (json.dumps(value, indent=2, allow_nan=False) + '\n').encode()


def authenticate(folder):
    raw = {}
    for filename, (_, expected) in PINS.items():
        blob = (folder / filename).read_bytes()
        check(pin(blob)['sha256'] == expected, f'changed SH009 source: {filename}')
        raw[filename] = blob

    staged = tomllib.loads(raw['resource.toml'].decode(), parse_float=Decimal)
    contract = tomllib.loads(raw['contract.toml'].decode(), parse_float=Decimal)
    exported = json.loads(raw['native-source.json'], parse_float=Decimal)
    manifest = json.loads(raw['native-export-manifest.json'])
    check(staged['id'] == contract['id'] == exported['id'] == 'SH009', 'wrong source identity')
    check(staged['name'] == contract['name'] == 'Goshawk Mk II', 'wrong hull name')
    check('nativeDesign' not in staged, 'staged display unexpectedly contains an active native mesh')
    check(contract['source']['geometryFile'] == exported['sourcePath'] == SOURCE_PROPOSAL,
          'native original source path mismatch')
    check(contract['source']['geometrySHA256'] == exported['sourceSHA256'] == SOURCE_PROPOSAL_SHA,
          'native original source digest mismatch')
    rows = [row for row in manifest['models'] if row['id'] == 'SH009']
    expected_export = dict(pin(raw['native-source.json']), id='SH009', file='SH009.json',
                           sourcePath=SOURCE_PROPOSAL, sourceSHA256=SOURCE_PROPOSAL_SHA)
    check(rows == [expected_export], 'native export manifest did not bind the original')
    check('30000000/3186991' in staged['geometryContext'], 'different adopted metre calibration')
    return raw, staged['design'], exported['nativeDesign'], contract['geometry']['nativeFaceLimit']


def encode_model(model, scale, expected, vertex_offset):
    vertex_count, face_count, component_count = expected
    source_vertices, faces = model['vertices'], model['faces']
    check((len(source_vertices), len(faces)) == (vertex_count, face_count), 'unexpected original mesh size')
    check(model['license'] == model['appearanceLicense'] == 'MIT', 'wrong original notice')
    check(model['paletteName'] == 'forest-silver', 'wrong original palette')
    check(len(model['faceMaterials']) == len(model['faceTextures']) == face_count, 'incomplete face appearance')
    verts = [tuple(float(Fraction(v) * scale) for v in point) for point in source_vertices]
    check(all(len(point) == 3 and all(isfinite(v) for v in point) for point in verts), 'invalid vertex')
    check(all(len(face) == 3 and len(set(face)) == 3 and all(
        type(i) is int and 0 <= i < vertex_count for i in face) for face in faces), 'invalid source faces')
    groups = components(model)
    check(len(groups) == component_count, 'original connectivity changed')
    ownership = {}
    rows = []
    for slot, ids in enumerate(groups):
        check(ids == list(range(ids[0], ids[-1] + 1)), 'noncontiguous original component')
        ownership.update((i, slot) for i in ids)
        rows.append({'startVertex': ids[0], 'vertexCount': len(ids), 'sourceFaces': []})
    for i, face in enumerate(faces):
        owners = {ownership[v] for v in face}
        check(len(owners) == 1, 'face crosses original component boundary')
        rows[owners.pop()]['sourceFaces'].append(i)
    for row in rows:
        ids = row.pop('sourceFaces')
        check(bool(ids) and ids == list(range(ids[0], ids[-1] + 1)), 'noncontiguous component faces')
        row.update(startFace=ids[0], faceCount=len(ids))
    bounds = [[take(axis) for axis in zip(*verts, strict=True)] for take in (min, max)]
    return (b''.join(struct.pack('<3d', *v) for v in verts),
            b''.join(struct.pack('<3I', *(i + vertex_offset for i in face)) for face in faces),
            rows, {'vertices': vertex_count, 'faces': face_count, 'components': component_count,
                   'boundsMetres': bounds, 'palette': model['paletteName'],
                   'textureIds': [row['asset'] for row in model['textures']]})


def register_piece(names, detail_slots, native_slots, name, d=None, n=None):
    slot = len(names)
    names.append(name)
    for mapping, limits in ((detail_slots, d), (native_slots, n)):
        if limits is not None:
            for index in range(*limits):
                check(index not in mapping, 'duplicate piece assignment')
                mapping[index] = slot


def partition(details, native):
    # Keep well mouths local. Pair the major assemblies across source versions.
    # The four Mk II bracket/runner boxes stay independent of their neighbours.
    detail_slots = {}
    native_slots = {}
    names = []
    piece = partial(register_piece, names, detail_slots, native_slots)

    for side, d, n in [('left', 0, 0), ('right', 576, 116)]:
        piece(f'{side}-rail', (d, d + 8), (n, n + 8))
        piece(f'{side}-drive-arm', (d + 8, d + 128), (n + 8, n + 68))
        for i in range(12):
            begin = d + 128 + i * 36
            piece(f'{side}-detailed-well-{i}', (begin, begin + 36))
        for i in range(4):
            begin = n + 68 + i * 12
            piece(f'{side}-native-well-{i}', n=(begin, begin + 12))
        for i in range(2):
            begin = d + 560 + i * 8
            piece(f'{side}-strut-{i}', (begin, begin + 8))
    piece('fore-crossbrace', (1152, 1160), (232, 240))
    piece('aft-crossbrace', (1160, 1168), (240, 248))
    piece('primary-body', (1168, 1288), (248, 308))
    for i in range(12):
        begin = 1288 + i * 36
        piece(f'detailed-body-well-{i}', (begin, begin + 36))
    for i in range(4):
        begin = 308 + i * 12
        piece(f'native-body-well-{i}', n=(begin, begin + 12))
    piece('upper-body-strut', (1720, 1728))
    piece('lower-body-strut', (1728, 1736))
    piece('cockpit', (1736, 1856), (356, 416))
    piece('cockpit-crossmember', (1856, 1864), (416, 424))
    for side, d, n in [('left', 1864, 424), ('right', 1968, 480)]:
        piece(f'{side}-engine-support', (d, d + 8), (n, n + 8))
        piece(f'{side}-engine-casing', (d + 8, d + 56), (n + 8, n + 32))
        piece(f'{side}-engine-recess', (d + 56, d + 104), (n + 32, n + 56))
    for side, d, n in [('left', 2072, 536), ('right', 2088, 552)]:
        piece(f'{side}-ventral-bracket', (d, d + 8), (n, n + 8))
        piece(f'{side}-ventral-runner', (d + 8, d + 16), (n + 8, n + 16))

    check(len(names) == 73, 'unexpected SH009 piece count')
    for model, mapping, vertex_count in [(details, detail_slots, 2104), (native, native_slots, 568)]:
        check(set(mapping) == set(range(vertex_count)), 'incomplete SH009 piece assignment')
        for component in model:
            start = component['startVertex']
            ids = range(start, start + component['vertexCount'])
            check(all(mapping[i] == mapping[start] for i in ids), 'SH009 component was split')
    membership = bytes(detail_slots[i] for i in range(2104)) + bytes(native_slots[i] for i in range(568))
    return names, membership


def prepare(folder):
    raw, detailed, native, face_limit = authenticate(folder)
    wants = (2104, 3532, 169), (568, 1012, 31)
    dv, df, dc, details = encode_model(detailed, Fraction(1), wants[0], 0)
    nv, nf, nc, candidate = encode_model(native, NATIVE_SCALE, wants[1], wants[0][0])
    check(face_limit == 512 and candidate['faces'] > face_limit, 'wrong native qualification condition')
    check(details['boundsMetres'] == candidate['boundsMetres'], 'calibrated extent disagreement')
    extent = [b - a for a, b in zip(*details['boundsMetres'], strict=True)]
    check(abs(extent[2] - 30.0) < 1e-12, 'staged detailed length differs from the authorized target')
    pieces, membership = partition(dc, nc)
    encoded = {
        'source.vertices': struct.pack('<8sII', b'MSH009V1', wants[0][0], wants[1][0]) + dv + nv,
        'source-triangles.bin': df + nf,
        'COMPONENTS.json': json_bytes({'detailed': dc, 'nativeCandidate': nc}),
    }
    encoded['SOURCE-GEOMETRY.json'] = json_bytes({
        'id': 'SH009', 'name': 'Goshawk Mk II', 'legacyRevision': LEGACY_REVISION,
        'mode': 'SOURCE_PREPARATION_ONLY;RUNTIME_ADMISSION_PENDING',
        'calibration': {'detailed': 'Already calibrated by original stage, retained as supplied',
                        'native': '30000000/3186991 m per original source unit, rounded once to binary64',
                        'orientation': '+Z forward, +Y up', 'extentMetres': extent},
        'detailed': details, 'nativeCandidate': dict(candidate, exactNativeFaceLimit=face_limit),
        'legacyInputs': {name: dict(pin(raw[name]), path=source_path)
                         for name, (source_path, _) in PINS.items()},
        'originalNativeProposal': {'path': SOURCE_PROPOSAL, 'sha256': SOURCE_PROPOSAL_SHA},
        'encodings': {name: pin(blob) for name, blob in encoded.items()},
    })
    encoded['SHIELD-PARTITION.json'] = json_bytes({
        'id': 'SH009', 'pieces': pieces, 'roundingMetres': 0.02,
        'originalGeometry': pin(encoded['SOURCE-GEOMETRY.json']), 'membership': pin(membership),
    })
    encoded['source-piece-membership.bin'] = membership
    return encoded


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ASSETS)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, blob in prepare(args.assets).items():
        path = args.assets / name
        if args.check:
            check(path.read_bytes() == blob, f'changed SH009 geometry encoding: {name}')
        else:
            path.write_bytes(blob)
    print('SH009 checked: 2 calibrated sources, 4,544 original faces, 200 index-connected components, 73 rounded supports.')


if __name__ == '__main__':
    main()
