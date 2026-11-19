"""Prepare the original metre-authored SH031 collision and visual streams."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib

from encode_goshawk_visual import TEXTURES
from import_goshawk_envelope_source import components, pin


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/fleet/sh031'
DETAIL = (1612, 3108, 29)
NATIVE = (96, 160, 8)
PIECE_COUNT = 25
MAGIC = b'MSH031V1'
VISUAL_MAGIC = b'KITEVIZ1'
UNMAPPED = 0xffff_ffff
LEGACY = 'a11fe16924d87d06021816f9037952afcbac08db'
NATIVE_PATH = 'fleet-completion-20261019/batch-02-native-lod/SH031-design.toml'
SOURCE_SHA = '099246e4cea539c64042a0749ba1873e3fdbe2dfba92c53bc7370c03719dea56'
PINS = {
    'resource.toml': '13e4d6b94a801c84dd89b9f4af9c8ba21151a33782b7e9a98c03ef503302772f',
    'native-export.json': '861c137f8b8881a5569ce518175de5e11397fcfefaf7d59e25cfab295693deff',
    'contract.toml': 'cedbde09b76675e9fb3f6022d9c78330524be816ec3d3878f600c0b5244fc8b0',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
DETAIL_NAMES = (
    'central-fuselage',
    'left-blade', 'left-nacelle', 'left-cross-brace',
    'left-drive', 'left-drive', 'left-drive',
    'right-blade', 'right-nacelle', 'right-cross-brace',
    'right-drive', 'right-drive', 'right-drive',
    'glazing', 'canopy-crest',
    'left-pipe', 'left-pipe-mount', *(f'left-pipe-clamp-{i}' for i in range(5)),
    'right-pipe', 'right-pipe-mount', *(f'right-pipe-clamp-{i}' for i in range(5)),
)
NATIVE_DETAIL = (0, 1, 2, 4, 7, 8, 10, 13)


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def document(data):
    return (json.dumps(data, indent=2, allow_nan=False) + '\n').encode()


def authenticate(folder):
    raw = {name: (folder / name).read_bytes() for name in PINS}
    for name, data in raw.items():
        require(hashlib.sha256(data).hexdigest() == PINS[name], 'changed SH031 original: ' + name)
    resource = tomllib.loads(raw['resource.toml'].decode())
    contract = tomllib.loads(raw['contract.toml'].decode())
    export = json.loads(raw['native-export.json'])
    require(resource['id'] == contract['id'] == export['id'] == 'SH031'
            and resource['name'] == contract['name'] == 'Kite Interceptor', 'wrong original identity')
    require(export['sourcePath'] == contract['source']['geometryFile'] == NATIVE_PATH
            and export['sourceSHA256'] == contract['source']['geometrySHA256'] == SOURCE_SHA
            and export['coordinateUnits'] == 'metres', 'wrong approved Kite Interceptor metre source')
    detail, native = resource['design'], resource['nativeDesign']
    require(native == export['nativeDesign'], 'resource native does not match approved archived mesh')
    require(NATIVE[1] <= contract['geometry']['nativeFaceLimit'] == 512, 'wrong native face limit')
    return detail, native


def source_bounds(mesh, counts):
    coords, faces = mesh['vertices'], mesh['faces']
    require((len(coords), len(faces)) == counts[:2], 'wrong original source counts')
    require(mesh['license'] == mesh['appearanceLicense'] == 'MIT'
            and mesh['paletteName'] == 'navy-copper', 'wrong appearance provenance')
    require([item['asset'] for item in mesh['textures']] == TEXTURES
            and all(t['repeat'] and t['license'] == 'MIT' for t in mesh['textures']),
            'wrong original shared maps')
    require(all(len(p) == 3 and all(math.isfinite(v) for v in p) for p in coords),
            'invalid original coordinates')
    require(all(len(f) == len(set(f)) == 3 and all(type(i) is int and 0 <= i < len(coords) for i in f)
                for f in faces), 'invalid original triangle')
    return [[func(axis) for axis in zip(*coords, strict=True)] for func in (min, max)]


def owners(mesh, labels, counts, slots):
    groups = components(mesh)
    require(len(groups) == counts[2], 'wrong original component count')
    assigned = [None] * counts[0]
    for group, label in zip(groups, labels, strict=True):
        for index in group:
            assigned[index] = slots[label]
    require(all(assigned[a] == assigned[b] == assigned[c] for a, b, c in mesh['faces']),
            'source triangle crosses local supports')
    return bytes(assigned)


def encode_visual(mesh):
    faces = mesh['faces']
    data = bytearray(struct.pack('<8sII', VISUAL_MAGIC, 3 * len(faces), 56))
    for face, material_number, mapping in zip(faces, mesh['faceMaterials'], mesh['faceTextures'], strict=True):
        points = [mesh['vertices'][i] for i in face]
        ax, ay, az = (b - a for a, b in zip(points[0], points[1], strict=True))
        bx, by, bz = (b - a for a, b in zip(points[0], points[2], strict=True))
        normal = (ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx)
        length = math.sqrt(sum(v * v for v in normal))
        slot = mapping['texture']
        require(length > 0 and math.isfinite(length) and -1 <= slot < len(TEXTURES),
                'invalid Kite Interceptor visual face')
        material = mesh['materials'][material_number]
        rgba = ((material['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0))
        color = list(rgba)
        n = [v / length for v in normal]
        for p, uv in zip(points, mapping['uv'], strict=True):
            data.extend(struct.pack('<12fII', *p, *n, *color, *uv, slot & UNMAPPED, int(material['emissive'])))
    return bytes(data)


def prepare(folder):
    detail, native = authenticate(folder)
    detail_bounds, native_bounds = source_bounds(detail, DETAIL), source_bounds(native, NATIVE)
    require(detail_bounds == native_bounds == [[-6.67, -6.5, -19.5], [6.67, 6.5, 19.5]],
            'changed authored metre envelope')
    names = list(dict.fromkeys(DETAIL_NAMES))
    require(len(names) == PIECE_COUNT, 'wrong local support count')
    slots = {name: i for i, name in enumerate(names)}
    memberships = owners(detail, DETAIL_NAMES, DETAIL, slots)
    memberships += owners(native, [DETAIL_NAMES[i] for i in NATIVE_DETAIL], NATIVE, slots)
    points = detail['vertices'] + native['vertices']
    vertices = struct.pack('<8sII', MAGIC, DETAIL[0], NATIVE[0])
    vertices += b''.join(struct.pack('<3d', *point) for point in points)
    faces = detail['faces'] + [[i + DETAIL[0] for i in face] for face in native['faces']]
    output = {
        'source.vertices': vertices,
        'source-triangles.bin': b''.join(struct.pack('<3I', *face) for face in faces),
        'source-piece-membership.bin': memberships,
        'PIECES.json': document({'pieces': names, 'roundingMetres': .02}),
        'visual.bin': encode_visual(detail),
        'native-visual.bin': encode_visual(native),
    }
    output['SOURCE-GEOMETRY.json'] = document({
        'id': 'SH031', 'name': 'Kite Interceptor', 'legacyRevision': LEGACY,
        'sourceUnits': 'metres', 'metresPerSourceUnit': 1,
        'detail': dict(zip(('vertices', 'faces', 'components'), DETAIL, strict=True), bounds=detail_bounds),
        'native': dict(zip(('vertices', 'faces', 'components'), NATIVE, strict=True), bounds=native_bounds,
                       faceLimit=512, approvedSourcePath=NATIVE_PATH, approvedSourceSha256=SOURCE_SHA),
        'shapePieces': PIECE_COUNT, 'textures': TEXTURES, 'unmappedMaterialSlot': UNMAPPED,
        'pins': PINS, 'outputs': {name: pin(data) for name, data in output.items()},
    })
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ASSETS)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, data in prepare(args.assets).items():
        out = args.assets / name
        if args.check:
            require(out.read_bytes() == data, 'changed SH031 encoding: ' + name)
        else:
            out.write_bytes(data)


if __name__ == '__main__':
    main()
