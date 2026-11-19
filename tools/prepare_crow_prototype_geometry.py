"""Authenticate both original SH020 meshes and package the controlled source shield."""
import argparse
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib

from encode_goshawk_visual import TEXTURES, encode_model
from import_goshawk_envelope_source import components, pin


ROOT = Path(__file__).resolve().parents[1]
FOLDER = ROOT / 'assets/fleet/sh020'
DETAIL = (2064, 3472, 164)
NATIVE = (528, 952, 26)
PIECE_COUNT = 68
MAGIC = b'MSH020V1'
VISUAL_MAGIC = b'CROWVIZ1'
SCALE = Fraction(20000, 2533)
PINS = {
    'resource.toml': 'be4983ac1ac1d194e77cb67c8f83d51f9bfbbaa125e844733ec355455d6dec69',
    'native-export.json': 'c260bc59bef8e46dddf81056070a5b58712ad07255a57ed3f2f51ce3b4d29e9b',
    'engineering.json': 'f2c3b08d17db3f1fba450c93f699e6c81bdc32acd12e78d870b5fcd01c6f2711',
    'contract.toml': 'a4578fb4a6fc9bddd3bbda55fe802ed80e153c0389610e21bf98c5e5521062c5',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
APPEARANCE = ('vertices', 'faces', 'faceMaterials', 'materials', 'faceTextures', 'textures')
NATIVE_DETAIL = {
    0: 0, 5: 51, 10: 102, 11: 103, 12: 104, 13: 105, 18: 156, 19: 157,
    20: 158, 21: 159, 22: 160, 23: 161, 24: 162, 25: 163,
}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def document(data):
    return (json.dumps(data, indent=2, allow_nan=False) + '\n').encode()


def authenticate(folder):
    raw = {name: (folder / name).read_bytes() for name in PINS}
    for name, encoded in raw.items():
        require(hashlib.sha256(encoded).hexdigest() == PINS[name], 'changed SH020 original: ' + name)
    source = tomllib.loads(raw['resource.toml'].decode())
    archive = json.loads(raw['native-export.json'])
    engineering = json.loads(raw['engineering.json'])
    contract = tomllib.loads(raw['contract.toml'].decode())
    require(source['id'] == archive['id'] == engineering['id'] == contract['id'] == 'SH020',
            'wrong original identity')
    require(source['name'] == contract['name'] == 'Crow Prototype', 'wrong SH020 name')
    require(engineering['contractSHA256'] == PINS['contract.toml']
            and contract['source']['geometrySHA256'] == archive['sourceSHA256']
            == engineering['detailedSourceSHA256'], 'wrong original lineage')
    require(Fraction(engineering['metresPerSourceUnitExact']) == SCALE
            and engineering['metresPerSourceUnit'] == float(SCALE)
            and engineering['targetDetailedLengthMetres'] == 30.0
            and engineering['sourceOriginUnchanged'], 'wrong SH020 calibration')
    require(contract['geometry']['nativeFaceLimit'] == 512 < len(archive['nativeDesign']['faces']),
            'changed native face discrepancy')
    detail, native = source['design'], source['nativeDesign']
    require(all(native[key] == archive['nativeDesign'][key] for key in APPEARANCE),
            'archived native mesh differs from the active original')
    return detail, native, engineering


def metric(source, counts):
    coords, faces = source['vertices'], source['faces']
    require((len(coords), len(faces)) == counts[:2], 'wrong source mesh dimensions')
    require(source['paletteName'] == 'navy-copper' and source['license'] == source['appearanceLicense'] == 'MIT',
            'wrong source appearance')
    require([item['asset'] for item in source['textures']] == TEXTURES, 'wrong source texture assets')
    require(all(len(p) == 3 and all(math.isfinite(v) for v in p) for p in coords),
            'invalid source coordinates')
    require(all(len(face) == len(set(face)) == 3 and
                all(type(i) is int and 0 <= i < len(coords) for i in face) for face in faces),
            'invalid original triangle')
    return {**source, 'vertices': [[value * float(SCALE) for value in point] for point in coords]}


def partition(detail, native, engineering):
    details, natives = components(detail), components(native)
    require((len(details), len(natives)) == (DETAIL[2], NATIVE[2]), 'changed source connectivity')
    names = []
    points = []
    by_name = {}
    primary = {}
    stock = engineering['stockComponents']
    require(len(stock) == len(details), 'missing detailed engineering component records')
    next_face = 0
    for index, (ids, record) in enumerate(zip(details, stock, strict=True)):
        start, count = record['faceStart'], record['faceCount']
        require(record['index'] == index and start == next_face and
                set(ids) == {i for face in detail['faces'][start:start + count] for i in face},
                'engineering component face range changed')
        next_face += count
        name = record['name']
        if '/recess-service-strut-' in name:
            parent = name.rsplit('/', 1)[0]
            points[by_name[parent]].extend(ids)
        else:
            by_name[name] = primary[index] = len(points)
            names.append(f'{index}:{name}')
            points.append(ids.copy())
    require(next_face == DETAIL[1], 'engineering components left source faces unbound')
    for index, ids in enumerate(natives):
        if index in NATIVE_DETAIL:
            equivalent = NATIVE_DETAIL[index]
            d = {tuple(detail['vertices'][i]) for i in details[equivalent]}
            require(all(tuple(native['vertices'][i]) in d for i in ids), 'native coarse source changed')
            slot = primary[equivalent]
        else:
            slot = len(points)
            names.append(f'native-service-well-{index}')
            points.append([])
        points[slot].extend(i + DETAIL[0] for i in ids)
    require(len(points) == PIECE_COUNT, 'wrong support count')
    return names, [sorted(ids) for ids in points]


def membership(faces, groups, length):
    slots = [None] * length
    for slot, ids in enumerate(groups):
        for i in ids:
            require(slots[i] is None, 'duplicate vertex assignment')
            slots[i] = slot
    require(None not in slots, 'source vertex missing from local supports')
    require(all(slots[a] == slots[b] == slots[c] for a, b, c in faces),
            'a source triangle crosses a support')
    return bytes(slots)


def pack_model(model, counts, extra=0):
    header = struct.pack('<8sII', MAGIC, counts[0], extra)
    vertices = header + b''.join(struct.pack('<3d', *point) for point in model['vertices'])
    triangles = b''.join(struct.pack('<3I', *face) for face in model['faces'])
    return vertices, triangles


def prepare(folder):
    detail, native, engineering = authenticate(folder)
    detail, native = metric(detail, DETAIL), metric(native, NATIVE)
    spans = [[take(axis) for axis in zip(*model['vertices'], strict=True)]
             for model in (detail, native) for take in (min, max)]
    require(all(math.isclose(spans[i + 1][2] - spans[i][2], 30.0, rel_tol=0, abs_tol=1e-12)
                for i in (0, 2)), 'wrong metric hull length')
    names, groups = partition(detail, native, engineering)
    shifted = [[i + DETAIL[0] for i in f] for f in native['faces']]
    owners = membership(detail['faces'] + shifted, groups, DETAIL[0] + NATIVE[0])
    d_points, d_faces = pack_model(detail, DETAIL, extra=NATIVE[0])
    n_points, n_faces = pack_model(native, NATIVE)
    output = {
        'source.vertices': d_points + n_points[16:],
        'source-triangles.bin': d_faces + b''.join(struct.pack('<3I', *f) for f in shifted),
        'source-piece-membership.bin': owners,
        'PIECES.json': document({'pieces': names, 'roundingMetres': .02}),
        'visual.bin': encode_model(detail, d_points, d_faces, MAGIC, (DETAIL[0], NATIVE[0], DETAIL[1]), VISUAL_MAGIC),
        'native-visual.bin': encode_model(native, n_points, n_faces, MAGIC, (NATIVE[0], 0, NATIVE[1]), VISUAL_MAGIC),
    }
    output['SOURCE-GEOMETRY.json'] = document({
        'id': 'SH020', 'name': 'Crow Prototype', 'legacyRevision': 'a11fe16924d87d06021816f9037952afcbac08db',
        'metresPerSourceUnit': str(SCALE), 'sourceOriginUnchanged': True,
        'detail': dict(zip(('vertices', 'faces', 'components'), DETAIL, strict=True), bounds=spans[:2]),
        'native': dict(zip(('vertices', 'faces', 'components'), NATIVE, strict=True), bounds=spans[2:],
                       faceLimit=512, faceLimitStatus='ORIGINAL_EXCEEDS_LIMIT'),
        'shapePieces': PIECE_COUNT, 'membershipEncoding': 'one u8 support index per source vertex',
        'textures': TEXTURES, 'pins': PINS, 'outputs': {name: pin(raw) for name, raw in output.items()},
    })
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=FOLDER)
    parser.add_argument('--check', action='store_true')
    config = parser.parse_args()
    for name, data in prepare(config.assets).items():
        path = config.assets / name
        if config.check:
            require(path.read_bytes() == data, 'changed SH020 encoding: ' + name)
        else:
            path.write_bytes(data)


if __name__ == '__main__':
    main()
