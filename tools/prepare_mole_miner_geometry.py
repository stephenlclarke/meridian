"""Authenticate both original SH016 meshes and package the controlled source shield."""
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
FOLDER = ROOT / 'assets/fleet/sh016'
DETAIL = (6760, 12836, 179)
NATIVE = (616, 1100, 33)
PIECE_COUNT = 203
MAGIC = b'MSH016V1'
VISUAL_MAGIC = b'MOLEVIS1'
SCALE = Fraction(11500, 687)
PINS = {
    'resource.toml': 'b23f6d0c214e199e19e6eaf71647cdbe45cc2a1c6b25935d30c1299335c5a2aa',
    'native-export.json': '9efa31451cebc03f9e00ef976d3c2ecd81e9bab8fdb7699f8b0b6aea00db60eb',
    'engineering.json': '9e8d0af0f1eca9b6de1307c33324ac96b0fb4f4652a2477b66fbf1d95a43421d',
    'contract.toml': 'd1bf21e0ad41a1fab71976310b850c67c28de10666f331b9b09e99363967d366',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
# Each 22.5-degree sector ends on a head vertex or an octagonal face normal.
COLLAR_STEPS = (3,) * 16
APPEARANCE = ('vertices', 'faces', 'faceMaterials', 'materials', 'faceTextures', 'textures')
NATIVE_DETAIL = {
    0: 0, 5: 51, 6: 52, 7: 53, 8: 54, 9: 55, 10: 56, 11: 57, 16: 108,
    21: 159, 22: 160, 23: 165, 24: 166, 25: 171, 26: 172, 27: 173, 28: 174,
    29: 175, 30: 176, 31: 177, 32: 178,
}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def document(data):
    return (json.dumps(data, indent=2, allow_nan=False) + '\n').encode()


def authenticate(folder):
    raw = {name: (folder / name).read_bytes() for name in PINS}
    for name, encoded in raw.items():
        require(hashlib.sha256(encoded).hexdigest() == PINS[name], 'changed SH016 original: ' + name)
    source = tomllib.loads(raw['resource.toml'].decode())
    archive = json.loads(raw['native-export.json'])
    engineering = json.loads(raw['engineering.json'])
    contract = tomllib.loads(raw['contract.toml'].decode())
    require(source['id'] == archive['id'] == engineering['id'] == contract['id'] == 'SH016',
            'wrong original identity')
    require(source['name'] == contract['name'] == 'Mole Miner', 'wrong SH016 name')
    require(engineering['contractSHA256'] == PINS['contract.toml']
            and contract['source']['geometrySHA256'] == archive['sourceSHA256']
            == engineering['detailedSourceSHA256'], 'wrong original lineage')
    require(Fraction(engineering['metresPerSourceUnitExact']) == SCALE
            and engineering['metresPerSourceUnit'] == float(SCALE)
            and engineering['targetDetailedLengthMetres'] == 46.0
            and engineering['sourceOriginUnchanged'], 'wrong SH016 calibration')
    require(contract['geometry']['nativeFaceLimit'] == 512 < len(archive['nativeDesign']['faces']),
            'changed native face discrepancy')
    detail, native = source['design'], source['nativeDesign']
    require(all(native[key] == archive['nativeDesign'][key] for key in APPEARANCE),
            'archived native mesh differs from the active original')
    return detail, native, engineering


def metric(source, counts):
    coords, faces = source['vertices'], source['faces']
    require((len(coords), len(faces)) == counts[:2], 'wrong source mesh dimensions')
    require(source['paletteName'] == 'slate-orange' and source['license'] == source['appearanceLicense'] == 'MIT',
            'wrong source appearance')
    require([item['asset'] for item in source['textures']] == TEXTURES, 'wrong source texture assets')
    require(all(len(p) == 3 and all(math.isfinite(v) for v in p) for p in coords),
            'invalid source coordinates')
    require(all(len(face) == len(set(face)) == 3 and
                all(type(i) is int and 0 <= i < len(coords) for i in face) for face in faces),
            'invalid original triangle')
    return {**source, 'vertices': [[value * float(SCALE) for value in point] for point in coords]}


def collar_sectors(ids):
    require(ids == list(range(ids[0], ids[0] + 576)), 'changed collar angular grid')
    start = 0
    for width in COLLAR_STEPS:
        rings = [angle % 48 for angle in range(start, start + width + 1)]
        yield [ids[12 * angle + i] for angle in rings for i in range(12)]
        start += width


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
        if name == 'boring head collar':
            for sector, part in enumerate(collar_sectors(ids)):
                names.append(f'boring-head-collar-{index}-sector-{sector}')
                points.append(part)
        elif '/recess-service-strut-' in name:
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


def verify_face_coverage(faces, pieces, length):
    index = [set() for _ in range(length)]
    for slot, ids in enumerate(pieces):
        require(len(ids) == len(set(ids)) and len(ids) >= 4, 'invalid support membership')
        for i in ids:
            index[i].add(slot)
    require(all(index), 'source vertex missing from the local supports')
    require(all(index[a] & index[b] & index[c] for a, b, c in faces),
            'a source triangle crosses a support seam')


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
    require(all(math.isclose(spans[i + 1][2] - spans[i][2], 46.0, rel_tol=0, abs_tol=1e-12)
                for i in (0, 2)), 'wrong metric hull length')
    names, groups = partition(detail, native, engineering)
    shifted = [[i + DETAIL[0] for i in f] for f in native['faces']]
    verify_face_coverage(detail['faces'] + shifted, groups, DETAIL[0] + NATIVE[0])
    d_points, d_faces = pack_model(detail, DETAIL, extra=NATIVE[0])
    n_points, n_faces = pack_model(native, NATIVE)
    output = {
        'source.vertices': d_points + n_points[16:],
        'source-triangles.bin': d_faces + b''.join(struct.pack('<3I', *f) for f in shifted),
        'source-piece-membership.bin': b''.join(struct.pack(f'<{1 + len(ids)}H', len(ids), *ids)
                                               for ids in groups),
        'PIECES.json': document({'pieces': names, 'roundingMetres': .02}),
        'visual.bin': encode_model(detail, d_points, d_faces, MAGIC, (DETAIL[0], NATIVE[0], DETAIL[1]), VISUAL_MAGIC),
        'native-visual.bin': encode_model(native, n_points, n_faces, MAGIC, (NATIVE[0], 0, NATIVE[1]), VISUAL_MAGIC),
    }
    output['SOURCE-GEOMETRY.json'] = document({
        'id': 'SH016', 'name': 'Mole Miner', 'legacyRevision': 'a11fe16924d87d06021816f9037952afcbac08db',
        'metresPerSourceUnit': str(SCALE), 'sourceOriginUnchanged': True,
        'detail': dict(zip(('vertices', 'faces', 'components'), DETAIL, strict=True), bounds=spans[:2]),
        'native': dict(zip(('vertices', 'faces', 'components'), NATIVE, strict=True), bounds=spans[2:],
                       faceLimit=512, faceLimitStatus='ORIGINAL_EXCEEDS_LIMIT'),
        'shapePieces': PIECE_COUNT, 'membershipEncoding': 'u16 little endian count, then source ids per support',
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
            require(path.read_bytes() == data, 'changed SH016 encoding: ' + name)
        else:
            path.write_bytes(data)


if __name__ == '__main__':
    main()
