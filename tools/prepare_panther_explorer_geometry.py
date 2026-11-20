"""Prepare the original metre-authored SH032 collision and visual streams."""
import argparse
from bisect import bisect_right
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib

from encode_goshawk_visual import TEXTURES
from import_goshawk_envelope_source import components, pin


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/fleet/sh032'
DETAIL = (2866, 5596, 34)
NATIVE = (206, 352, 15)
PIECE_COUNT = 36
SUPPORT_VERTICES = (2938, 230)
MAGIC = b'MSH032V1'
VISUAL_MAGIC = b'PNTXVIZ1'
UNMAPPED = 0xffff_ffff
LEGACY = 'a11fe16924d87d06021816f9037952afcbac08db'
NATIVE_PATH = 'fleet-completion-20261019/batch-01-native-lod/SH032-design.toml'
SOURCE_SHA = 'd87094362d608526b5e257173478f305b0b2f786d573abdb51f7ef613a4bb131'
PINS = {
    'resource.toml': '0023556a6c7a00481cdc10461eaf5b3ef4b9477e0c4f1919442e108fe0a94d9f',
    'native-export.json': '135ed9ca8b424fc27f4c5a65e043254b78ca8d48108fafe2099ae51060573a4f',
    'contract.toml': '9bbbecb8af20c7c20d0d95e0a1d82aeb27950d4a6243329af3a0e6848ecd3a6f',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
DETAIL_NAMES = (
    'equipment-keel', 'pressure-cabin',
    'left-curved-support', 'left-sled', 'left-cross-brace',
    'left-drive', 'left-drive', 'left-drive',
    'right-curved-support', 'right-sled', 'right-cross-brace',
    'right-drive', 'right-drive', 'right-drive',
    'panoramic-glazing', 'canopy-crest',
    'left-opaque-sponson', 'left-side-window', 'right-opaque-sponson', 'right-side-window',
    'left-pipe', 'left-pipe-mount', *(f'left-pipe-clamp-{i}' for i in range(5)),
    'right-pipe', 'right-pipe-mount', *(f'right-pipe-clamp-{i}' for i in range(5)),
)
NATIVE_DETAIL = (0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 14, 16, 17, 18, 19)
CURVE_BANDS = ('fore', 'fore-mid', 'aft-mid', 'aft')
# Each record gives the first original vertex and triangle of one longitudinal arch.
DETAIL_CURVES = ((388, 768), (1124, 2216))
NATIVE_CURVES = ((36, 64), (92, 160))


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def document(data):
    return (json.dumps(data, indent=2, allow_nan=False) + '\n').encode()


def authenticate(folder):
    raw = {name: (folder / name).read_bytes() for name in PINS}
    for name, data in raw.items():
        require(hashlib.sha256(data).hexdigest() == PINS[name], 'changed SH032 original: ' + name)
    resource = tomllib.loads(raw['resource.toml'].decode())
    contract = tomllib.loads(raw['contract.toml'].decode())
    export = json.loads(raw['native-export.json'])
    require(resource['id'] == contract['id'] == export['id'] == 'SH032'
            and resource['name'] == contract['name'] == 'Panther Explorer', 'wrong original identity')
    require(export['sourcePath'] == contract['source']['geometryFile'] == NATIVE_PATH
            and export['sourceSHA256'] == contract['source']['geometrySHA256'] == SOURCE_SHA
            and export['coordinateUnits'] == 'metres', 'wrong approved Panther Explorer metre source')
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


def curve_bands(mesh, curves, rings, stride, cuts):
    bands = {}
    face_count = (rings - 1) * 2 * stride + 2 * (stride - 2)
    for side, (first_vertex, first_face) in zip(('left', 'right'), curves, strict=True):
        for number in range(first_face, first_face + face_count):
            rings_in_face = sorted({(i - first_vertex) // stride for i in mesh['faces'][number]})
            require(0 <= rings_in_face[0] <= rings_in_face[-1] < rings, 'wrong curve ownership')
            if len(rings_in_face) == 1:
                require(rings_in_face[0] in (0, rings - 1), 'unexpected middle curve cap')
            else:
                require(len(rings_in_face) == 2 and rings_in_face[1] == rings_in_face[0] + 1,
                        'curve triangle crosses more than one span')
            band = bisect_right(cuts, rings_in_face[0])
            bands[number] = f'{side}-curved-support-{CURVE_BANDS[band]}'
    return bands


def support_mesh(mesh, labels, counts, slots, curves):
    groups = components(mesh)
    require(len(groups) == counts[2], 'wrong original component count')
    source_labels = [None] * counts[0]
    for group, label in zip(groups, labels, strict=True):
        for index in group:
            source_labels[index] = label
    vertices = list(mesh['vertices'])
    membership = [None] * len(vertices)
    duplicates = {}
    faces = []
    for number, face in enumerate(mesh['faces']):
        label = source_labels[face[0]]
        require(all(source_labels[i] == label for i in face), 'triangle leaves its source component')
        slot = slots[curves.get(number, label)]
        remapped = []
        for index in face:
            if membership[index] is None:
                membership[index] = slot
            if membership[index] != slot:
                key = (index, slot)
                if key not in duplicates:
                    duplicates[key] = len(vertices)
                    vertices.append(vertices[index])
                    membership.append(slot)
                index = duplicates[key]
            remapped.append(index)
        faces.append(remapped)
    return vertices, faces, bytes(membership)


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
                'invalid Panther Explorer visual face')
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
    require(detail_bounds == [[-9.829, -6.2, -20.09], [9.829, 7.56, 17.0]]
            and native_bounds == [[-9.829, -6.2, -20.09], [9.829, 7.48, 17.0]],
            'changed authored metre envelope')
    names = []
    for name in dict.fromkeys(DETAIL_NAMES):
        if name.endswith('-curved-support'):
            names.extend(f'{name}-{band}' for band in CURVE_BANDS)
        else:
            names.append(name)
    require(len(names) == PIECE_COUNT, 'wrong local support count')
    slots = {name: i for i, name in enumerate(names)}
    detail_points, detail_faces, detail_membership = support_mesh(
        detail, DETAIL_NAMES, DETAIL, slots, curve_bands(detail, DETAIL_CURVES, 28, 12, (5, 14, 22)))
    native_points, native_faces, native_membership = support_mesh(
        native, [DETAIL_NAMES[i] for i in NATIVE_DETAIL], NATIVE, slots,
        curve_bands(native, NATIVE_CURVES, 5, 4, (1, 2, 3)))
    require((len(detail_points), len(native_points)) == SUPPORT_VERTICES, 'wrong split arch vertex count')
    vertices = struct.pack('<8sII', MAGIC, *SUPPORT_VERTICES)
    vertices += b''.join(struct.pack('<3d', *point) for point in detail_points + native_points)
    faces = detail_faces + [[i + SUPPORT_VERTICES[0] for i in face] for face in native_faces]
    output = {
        'source.vertices': vertices,
        'source-triangles.bin': b''.join(struct.pack('<3I', *face) for face in faces),
        'source-piece-membership.bin': detail_membership + native_membership,
        'PIECES.json': document({'pieces': names, 'roundingMetres': .02}),
        'visual.bin': encode_visual(detail),
        'native-visual.bin': encode_visual(native),
    }
    output['SOURCE-GEOMETRY.json'] = document({
        'id': 'SH032', 'name': 'Panther Explorer', 'legacyRevision': LEGACY,
        'sourceUnits': 'metres', 'metresPerSourceUnit': 1,
        'detail': dict(zip(('vertices', 'faces', 'components'), DETAIL, strict=True), bounds=detail_bounds),
        'native': dict(zip(('vertices', 'faces', 'components'), NATIVE, strict=True), bounds=native_bounds,
                       faceLimit=512, approvedSourcePath=NATIVE_PATH, approvedSourceSha256=SOURCE_SHA),
        'supportIndexing': {
            'detailedVertices': SUPPORT_VERTICES[0], 'nativeVertices': SUPPORT_VERTICES[1],
            'sourceFaceCornerOrderPreserved': True,
        },
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
            require(out.read_bytes() == data, 'changed SH032 encoding: ' + name)
        else:
            out.write_bytes(data)


if __name__ == '__main__':
    main()
