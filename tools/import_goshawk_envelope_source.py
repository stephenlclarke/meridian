"""Authenticate and encode the SH008 staged display and original candidate native models."""
import argparse
from decimal import Decimal, localcontext
import hashlib
import json
import re
from pathlib import Path
import struct
import tomllib

if not __debug__:
    raise RuntimeError('Goshawk source verification requires Python assertions. Run without -O or -OO.')

ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / 'assets/fleet/sh008'
RESOURCE = 'resource.toml'
ORIGINAL_DESIGN = 'original-design.toml'
PINS = {
    RESOURCE: 'aafcb5dfec2cb5d141316756daae267888bf3541383f4f155c2561a4d7dd0bb7',
    ORIGINAL_DESIGN: '9a6b3e1403a5cbb45691592f2db2e1dac79bfc7c0d328a4b86a234487943e127',
    'contract.toml': '6815e92d5480692d18576ab5ed5e98cd1f99b1bff622c3c7eb346ed59bcf8d42',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
SCALE = (10_000_000, 1_028_273)


def pin(data):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def components(model):
    """Index-connected original records, without spatial welding or remeshing."""
    parents = list(range(len(model['vertices'])))

    def root(i):
        while parents[i] != i:
            i = parents[i]
        return i

    for a, b, c in model['faces']:
        anchor = root(a)
        parents[root(b)] = anchor
        parents[root(c)] = anchor
    groups = {}
    for i in range(len(parents)):
        groups.setdefault(root(i), []).append(i)
    return list(groups.values())


def assign_range(table, interval, slot):
    if interval is None:
        return
    for index in range(*interval):
        assert index not in table
        table[index] = slot


def partition_ranges():
    """Source-local assemblies; matching dense/coarse structures share a piece."""
    detail = {}
    native = {}
    names = []

    def piece(name, d=None, n=None):
        slot = len(names)
        names.append(name)
        assign_range(detail, d, slot)
        assign_range(native, n, slot)

    for side, d, n in [('left', 0, 0), ('right', 576, 116)]:
        piece(side + '-rail', (d, d + 8), (n, n + 8))
        piece(side + '-drive-arm', (d + 8, d + 128), (n + 8, n + 68))
        for well in range(12):
            lo = d + 128 + well * 36
            piece(f'{side}-detailed-well-{well}', (lo, lo + 36))
        for well in range(4):
            lo = n + 68 + well * 12
            piece(f'{side}-candidate-well-{well}', n=(lo, lo + 12))
        for strut in range(2):
            lo = d + 560 + strut * 8
            piece(f'{side}-strut-{strut}', (lo, lo + 8))
    piece('fore-crossbrace', (1152, 1160), (232, 240))
    piece('aft-crossbrace', (1160, 1168), (240, 248))
    piece('primary-body', (1168, 1288), (248, 308))
    for well in range(12):
        lo = 1288 + well * 36
        piece(f'detailed-body-well-{well}', (lo, lo + 36))
    for well in range(4):
        lo = 308 + well * 12
        piece(f'candidate-body-well-{well}', n=(lo, lo + 12))
    piece('upper-body-strut', (1720, 1728))
    piece('lower-body-strut', (1728, 1736))
    piece('cockpit', (1736, 1856), (356, 416))
    piece('cockpit-crossmember', (1856, 1864), (416, 424))
    for side, d, n in [('left', 1864, 424), ('right', 1968, 480)]:
        piece(side + '-engine-support', (d, d + 8), (n, n + 8))
        piece(side + '-engine-casing', (d + 8, d + 56), (n + 8, n + 32))
        piece(side + '-engine-recess', (d + 56, d + 104), (n + 32, n + 56))
    assert len(names) == 69
    return detail, native, names


def calibrated(raw_point):
    with localcontext() as context:
        context.prec = 70
        return tuple(float(Decimal(c) * SCALE[0] / SCALE[1]) for c in raw_point)


def encode(inputs):
    for name, expected in PINS.items():
        assert pin(inputs[name])['sha256'] == expected, name
    resource = tomllib.loads(inputs[RESOURCE].decode(), parse_float=Decimal)
    original = tomllib.loads(inputs[ORIGINAL_DESIGN].decode(), parse_float=Decimal)
    detailed, native = original['design'], original['nativeDesign']
    staged = resource['design']
    assert {k: v for k, v in staged.items() if k != 'vertices'} == {
        k: v for k, v in detailed.items() if k != 'vertices'
    }
    display = [tuple(map(float, p)) for p in staged['vertices']]
    assert display == [calibrated(p) for p in detailed['vertices']]
    assert 'nativeDesign' not in resource
    assert (len(display), len(detailed['faces']), len(native['vertices']), len(native['faces'])) == (2072, 3484, 536, 964)

    detail_slots, native_slots, names = partition_ranges()
    vertices = bytearray(b'MSH008V1') + struct.pack('<II', 2072, 536)
    faces = bytearray()
    membership = bytearray()
    ownership = {}
    for variant, source, points, slots, offset, component_count in [
        ('detailed', detailed, display, detail_slots, 0, 165),
        ('nativeCandidate', native, list(map(calibrated, native['vertices'])), native_slots, 2072, 27),
    ]:
        assert set(slots) == set(range(len(points)))
        assert source['license'] == source['appearanceLicense'] == 'MIT'
        for i, point in enumerate(points):
            vertices.extend(struct.pack('<3d', *point))
            membership.append(slots[i])
        for face in source['faces']:
            a, b, c = face
            assert slots[a] == slots[b] == slots[c]
            faces.extend(struct.pack('<3I', *(index + offset for index in face)))
        ownership[variant] = []
        groups = components(source)
        assert len(groups) == component_count
        for ids in groups:
            assert ids == list(range(ids[0], ids[-1] + 1))
            assert all(slots[ids[0]] == slots[i] for i in ids), ids
            ownership[variant].append({'startVertex': ids[0], 'vertexCount': len(ids), 'piece': slots[ids[0]]})

    outputs = {**inputs, 'source.vertices': bytes(vertices), 'source-triangles.bin': bytes(faces),
               'source-piece-membership.bin': bytes(membership),
               'COMPONENT-PIECES.json': (json.dumps({'pieces': names, **ownership}, indent=2) + '\n').encode()}
    manifest = {'id': 'SH008', 'sourceRevision': 'a11fe16924d87d06021816f9037952afcbac08db',
                'calibration': '10000000/1028273 m per source unit; exact staged display f64 and original native candidate rounded once',
                'detailed': {'vertices': 2072, 'faces': 3484}, 'nativeCandidate': {'vertices': 536, 'faces': 964},
                'envelope': '69 source-local convex supports plus a 0.02 m sphere; candidate only',
                'authority': 'Detailed asset staged. Native candidate and engineering/runtime qualification pending.',
                'files': {name: pin(data) for name, data in outputs.items()}}
    outputs['MANIFEST.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--snapshot', type=Path)
    parser.add_argument('--proposal', type=Path)
    parser.add_argument('--check', action='store_true')
    options = parser.parse_args()
    if options.check:
        inputs = {name: (ASSET / name).read_bytes() for name in PINS}
    else:
        if options.snapshot is None or options.proposal is None:
            parser.error('production requires --snapshot and --proposal')
        base = options.snapshot
        inputs = {
            RESOURCE: (base / 'Sources/L337Core/Resources/Ships/SH008.toml').read_bytes(),
            ORIGINAL_DESIGN: (options.proposal / 'replacement-hulls/SH008-design.toml').read_bytes(),
            'contract.toml': (base / 'docs/fleet-design-v2/ships/SH008.toml').read_bytes(),
            'LICENSE': (base / 'LICENSE').read_bytes(),
        }
    contract = tomllib.loads(inputs['contract.toml'].decode(), parse_float=Decimal)
    core = (ROOT / 'crates/meridian-core/src/goshawk.rs').read_text()
    speed = re.search(r'^pub\(crate\) const GOSHAWK_MAX_SPEED_UNITS: f32 = (\d+\.\d+);$', core, re.M)
    assert speed is not None and Decimal(speed[1]) == contract['performance']['maxSpeed']
    for name, data in encode(inputs).items():
        path = ASSET / name
        if options.check:
            assert path.read_bytes() == data, name
        else:
            path.write_bytes(data)
    print('SH008 source binding verified: 4,448 triangles, 192 components and 69 pieces. Geometry execution unavailable in source lane.')


if __name__ == '__main__':
    main()
