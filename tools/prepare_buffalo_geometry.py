"""Authenticate and prepare Buffalo's corrected SH013 source and detailed visual."""
import argparse
import hashlib
import json
from math import isfinite, sqrt
from pathlib import Path
import struct
import tomllib

from import_goshawk_envelope_source import components


ROOT = Path(__file__).resolve().parents[1]
FOLDER = ROOT / 'assets/fleet/sh013'
DETAIL = (2056, 3460, 163)
NATIVE = (280, 500, 15)
PIECE_COUNT = 57
TEXTURES = ['l337-patrol-diffuse', 'l337-strike-diffuse', 'l337-service-details']
PINS = {
    'resource.toml': '44fd4d58e0368443bf82e6f4701e63e11132feb4a8687da38cb72df8081603d7',
    'pre-correction-native-export.json': '8928c74d6e117ce65b1bdccc31ed075a3c3777a86706f4e6296faf25975a071a',
    'contract.toml': '84abd93c7ee81072ef52d70a95e7054bf30d778e988fd05452cb6bf959a37271',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def pin(raw):
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def document(value):
    return (json.dumps(value, indent=2, allow_nan=False) + '\n').encode()


def authenticate(folder):
    data = {name: (folder / name).read_bytes() for name in PINS}
    for name, raw in data.items():
        require(pin(raw)['sha256'] == PINS[name], 'changed SH013 original: ' + name)
    current = tomllib.loads(data['resource.toml'].decode())
    contract = tomllib.loads(data['contract.toml'].decode())
    archive = json.loads(data['pre-correction-native-export.json'])
    require(current['id'] == contract['id'] == archive['id'] == 'SH013', 'wrong original identity')
    require(current['name'] == contract['name'] == 'Buffalo', 'wrong Buffalo name')
    require(contract['source']['geometrySHA256'] == archive['sourceSHA256'],
            'wrong pre-correction native provenance')
    require(len(archive['nativeDesign']['faces']) == 928, 'wrong archived native model')
    require(contract['geometry']['nativeFaceLimit'] == 512, 'wrong native face limit')
    require('23.75metres/sourceunit' in current['design']['source']
            and '23.75metres/sourceunit' in current['nativeDesign']['source'],
            'missing corrected metre calibration')
    return current['design'], current['nativeDesign']


def dimensions(source, expected):
    points, triangles = source['vertices'], source['faces']
    require((len(points), len(triangles)) == expected[:2], 'wrong corrected source counts')
    require(source['paletteName'] == 'cream-maroon', 'wrong original material palette')
    require(source['license'] == source['appearanceLicense'] == 'MIT', 'wrong original licence')
    require([item['asset'] for item in source['textures']] == TEXTURES, 'wrong original maps')
    require(len(source['faceMaterials']) == len(source['faceTextures']) == len(triangles),
            'source face appearance incomplete')
    require(all(len(point) == 3 and all(isfinite(x) for x in point) for point in points),
            'invalid source coordinates')
    require(all(len(f) == 3 and len(set(f)) == 3 and all(
        type(v) is int and 0 <= v < len(points) for v in f) for f in triangles),
        'invalid triangle indices')
    return [[take(axis) for axis in zip(*points, strict=True)] for take in (min, max)]


class Partition:
    def __init__(self):
        self.names = []
        self.detail = {}
        self.native = {}

    def piece(self, name, detail=None, native=None):
        slot = len(self.names)
        self.names.append(name)
        for mapping, limits in ((self.detail, detail), (self.native, native)):
            if limits is not None:
                for index in range(*limits):
                    require(index not in mapping, 'duplicate source assignment')
                    mapping[index] = slot


def partition_models():
    result = Partition()
    for name, start, native in [('left', 0, 0), ('middle', 568, 52), ('right', 1136, 92)]:
        result.piece(f'{name}-drum', (start, start + 120), (native, native + 40))
        for i in range(12):
            well = start + 120 + 36 * i
            result.piece(f'{name}-detailed-well-{i}', (well, well + 36))
        for i in range(2):
            strut = start + 552 + 8 * i
            result.piece(f'{name}-strut-{i}', (strut, strut + 8))
    result.piece('left-native-well', native=(40, 52))
    result.piece('right-native-well', native=(132, 144))
    result.piece('command-deck', (1704, 1712), (144, 152))
    result.piece('short-cabin', (1712, 1832), (152, 192))
    result.piece('cabin-front-frame', (1832, 1840), (192, 200))
    for name, d, n in [('left', 1840, 200), ('right', 1944, 236)]:
        result.piece(f'{name}-thruster-bracket', (d, d + 8), (n, n + 8))
        result.piece(f'{name}-thruster-case', (d + 8, d + 56), (n + 8, n + 24))
        result.piece(f'{name}-thruster-recess', (d + 56, d + 104), (n + 24, n + 36))
    result.piece('short-opaque-plinth', (2048, 2056), (272, 280))
    require(len(result.names) == PIECE_COUNT, 'wrong local support count')
    return result


def assignments(source, mapping, expected):
    count = len(source['vertices'])
    require(set(mapping) == set(range(count)), 'incomplete source assignment')
    groups = components(source)
    require(len(groups) == expected, 'changed original connectivity')
    for points in groups:
        require(all(mapping[i] == mapping[points[0]] for i in points), 'split source component')
    for a, b, c in source['faces']:
        require(mapping[a] == mapping[b] == mapping[c], 'source triangle crossed shields')
    return bytes(mapping[i] for i in range(count))


def visual(model):
    data = bytearray(struct.pack('<8sII', b'BUFLVIZ1', DETAIL[1] * 3, 56))
    for face, tint, mapping in zip(model['faces'], model['faceMaterials'], model['faceTextures'], strict=True):
        points = [model['vertices'][i] for i in face]
        x, y, z = (b - a for a, b in zip(points[0], points[1], strict=True))
        u, v, w = (b - a for a, b in zip(points[0], points[2], strict=True))
        normal = (y * w - z * v, z * u - x * w, x * v - y * u)
        length = sqrt(sum(d * d for d in normal))
        require(length > 0 and isfinite(length) and 0 <= mapping['texture'] < len(TEXTURES),
                'invalid source visual triangle')
        material = model['materials'][tint]
        rgba = [(material['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
        for position, uv in zip(points, mapping['uv'], strict=True):
            data.extend(struct.pack('<12fII', *position, *(d / length for d in normal), *rgba, *uv,
                                    mapping['texture'], int(material['emissive'])))
    return bytes(data)


def prepare(folder):
    detailed, native = authenticate(folder)
    detail_bounds = dimensions(detailed, DETAIL)
    native_bounds = dimensions(native, NATIVE)
    require(detail_bounds[1][2] - detail_bounds[0][2] == 76.0
            and native_bounds[1][2] - native_bounds[0][2] == 76.0, 'not the corrected metre geometry')
    part = partition_models()
    membership = assignments(detailed, part.detail, DETAIL[2]) + assignments(native, part.native, NATIVE[2])
    coordinates = detailed['vertices'] + native['vertices']
    vertices = struct.pack('<8sII', b'MSH013V1', DETAIL[0], NATIVE[0])
    vertices += b''.join(struct.pack('<3d', *point) for point in coordinates)
    faces = b''.join(struct.pack('<3I', *face) for face in detailed['faces'])
    faces += b''.join(struct.pack('<3I', *(i + DETAIL[0] for i in face)) for face in native['faces'])
    output = {
        'source.vertices': vertices, 'source-triangles.bin': faces,
        'source-piece-membership.bin': membership, 'visual.bin': visual(detailed),
        'PIECES.json': document({'pieces': part.names, 'roundingMetres': .02}),
    }
    output['SOURCE-GEOMETRY.json'] = document({
        'id': 'SH013', 'name': 'Buffalo', 'legacyRevision': 'a11fe16924d87d06021816f9037952afcbac08db',
        'source': 'Corrected original resource with short opaque plinth and lowered bridge.',
        'authoringMetresPerOriginalUnit': 23.75, 'runtimeScaleForBundledGeometry': 1,
        'details': dict(zip(('vertices', 'faces', 'components'), DETAIL, strict=True), bounds=detail_bounds),
        'native': dict(zip(('vertices', 'faces', 'components'), NATIVE, strict=True), bounds=native_bounds, faceLimit=512),
        'archivedNative': {'file': 'pre-correction-native-export.json', 'faces': 928, 'role': 'provenance'},
        'textures': TEXTURES, 'pins': PINS, 'outputs': {k: pin(v) for k, v in output.items()},
    })
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=FOLDER)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, data in prepare(args.assets).items():
        output = args.assets / name
        if args.check:
            require(output.read_bytes() == data, 'changed SH013 encoding: ' + name)
        else:
            output.write_bytes(data)
    print('SH013 prepared: corrected 3,960 source triangles, 57 local supports and cream-maroon visual.')


if __name__ == '__main__':
    main()
