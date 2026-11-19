"""Prepare the original metre-authored SH042 collision and visual streams."""
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
ASSETS = ROOT / 'assets/fleet/sh042'
DETAIL = (1996, 3864, 32)
NATIVE = (132, 220, 11)
PIECE_COUNT = 26
MAGIC = b'MSH042V1'
VISUAL_MAGIC = b'SAKRVIZ1'
UNMAPPED = 0xffff_ffff
LEGACY = 'a11fe16924d87d06021816f9037952afcbac08db'
NATIVE_PATH = 'fleet-completion-20261019/batch-02-native-lod/SH042-design.toml'
SOURCE_SHA = '8b64ebb1529abe86018b7b2553c45f961c06f0e0a9182321ec1b528cb0a716c8'
PINS = {
    'resource.toml': '94707ec289d9b4b1147158ded2c69fc3fe1225a7686ce53a6b4968b3d16c3b1f',
    'native-export.json': 'ecf4c8d379c676ef7c309ebfcb1b52870d773f6eabf0d739c0aa99a1a276f689',
    'contract.toml': '36a903484eb884b1774baf0c18270d99916c54cc1cfa55aad32253284faf4293',
    'engineering.json': 'f113d1c0b723760cb7755ae51204d89e125bb5c3f8d1cf1f0070eaf96d54fa75',
    'finite-dry-source.json': '0ba38f2ab61682ba99c62696c2e22120131ab560018d785ee34f8924ad85358c',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
DETAIL_NAMES = (
    'forward-barrel',
    'upper-shroud', 'upper-thrust-brace', *('upper-drive',) * 3,
    'left-shroud', 'left-thrust-brace', *('left-drive',) * 3,
    'right-shroud', 'right-thrust-brace', *('right-drive',) * 3,
    'pressure-canopy', 'canopy-centre-brace',
    'left-service-pipe', 'left-instrument-housing', *(f'left-louvre-{i}' for i in range(5)),
    'right-service-pipe', 'right-instrument-housing', *(f'right-louvre-{i}' for i in range(5)),
)
NATIVE_DETAIL = (0, 1, 2, 3, 6, 7, 8, 11, 12, 13, 16)


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def document(data):
    return (json.dumps(data, indent=2, allow_nan=False) + '\n').encode()


def authenticate(folder):
    raw = {name: (folder / name).read_bytes() for name in PINS}
    for name, data in raw.items():
        require(hashlib.sha256(data).hexdigest() == PINS[name], 'changed SH042 original: ' + name)
    resource = tomllib.loads(raw['resource.toml'].decode())
    contract = tomllib.loads(raw['contract.toml'].decode())
    export = json.loads(raw['native-export.json'])
    require(resource['id'] == contract['id'] == export['id'] == 'SH042'
            and resource['name'] == contract['name'] == 'Saker', 'wrong original identity')
    require(export['sourcePath'] == contract['source']['geometryFile'] == NATIVE_PATH
            and export['sourceSHA256'] == contract['source']['geometrySHA256'] == SOURCE_SHA
            and export['coordinateUnits'] == 'metres', 'wrong approved Saker metre source')
    engineering = json.loads(raw['engineering.json'])
    finite = json.loads(raw['finite-dry-source.json'])
    require(engineering['id'] == finite['id'] == 'SH042'
            and engineering['coordinateUnits'] == finite['coordinateUnits'] == 'metres'
            and engineering['metresPerSourceUnit'] == finite['metresPerSourceUnit'] == 1
            and finite['r3SHA256'] == engineering['contractSHA256'] == PINS['contract.toml']
            and finite['engineeringSHA256'] == PINS['engineering.json']
            and finite['resourceSHA256'] == PINS['resource.toml']
            and finite['nativeSourceSHA256'] == SOURCE_SHA
            and engineering['detailedSourceSHA256'] == '1bb0ed2db6f14323e84a5073f57f75aef7aea86049fcd608b195146611780547',
            'wrong Saker engineering provenance')
    require(finite['nativeVertices'] == NATIVE[0] and finite['nativeTriangles'] == NATIVE[1]
            and finite['emptyStateOnly'] and not finite['operatingAuthorityGranted']
            and not finite['nativeEvidenceQualified'] and not finite['peopleOrFuelCargoEquipmentGranted']
            and finite['emptyDryReference']['massKg'] == engineering['dryMassKilograms'] == 26588,
            'wrong exact finite empty-dry reference')
    detail, native = resource['design'], resource['nativeDesign']
    require(native == export['nativeDesign'], 'resource native does not match approved archived mesh')
    require(NATIVE[1] <= contract['geometry']['nativeFaceLimit'] == 512, 'wrong native face limit')
    return detail, native


def source_bounds(mesh, counts):
    coords, faces = mesh['vertices'], mesh['faces']
    require((len(coords), len(faces)) == counts[:2], 'wrong original source counts')
    require(mesh['license'] == mesh['appearanceLicense'] == 'MIT'
            and mesh['paletteName'] == 'forest-ivory', 'wrong appearance provenance')
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
                'invalid Saker visual face')
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
    require(detail_bounds == native_bounds == [[-8.0681778, -5.506, -20.09], [8.0681778, 9.006, 21.5]],
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
        'id': 'SH042', 'name': 'Saker', 'legacyRevision': LEGACY,
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
            require(out.read_bytes() == data, 'changed SH042 encoding: ' + name)
        else:
            out.write_bytes(data)


if __name__ == '__main__':
    main()
