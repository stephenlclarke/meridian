"""Prepare the original metre-authored SH026 collision and visual streams."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import tomllib

from collections import Counter
from itertools import pairwise

from encode_goshawk_visual import TEXTURES
from import_goshawk_envelope_source import components, pin


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/fleet/sh026'
DETAIL = (5764, 8884, 661)
NATIVE = (306, 512, 25)
PIECE_COUNT = 180
MAGIC = b'MSH026V1'
VISUAL_MAGIC = b'HARRVIZ1'
UNMAPPED = 0xffff_ffff
LEGACY = 'a11fe16924d87d06021816f9037952afcbac08db'
LEGACY_PATH = 'replacement-hulls/SH026-design.toml'
SOURCE_SHA = 'd0b3155b579a53d027741e93cae48295749c315447e0891319946f569421dd58'
PINS = {
    'resource.toml': 'c76bbee83e1c58ec423a11988b9681794ef2e899c897a09ca1bbe66bb48aefc3',
    'pre-bare-native-export.json': 'f74febcad4a0a8003acc9e0b44a78b8dfbb9ebd2928bb1618e187cfbe87b2a6d',
    'contract.toml': 'f23d79dc0c1a0caf93d36a118fc33db7bc7289eac393f49d0c64795d6aa0972e',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
NATIVE_DETAIL = (0, 1, 2, 3, 4, 5, 6, 163, 164, 165, 166, 327, 328, 329,
                 330, 331, 492, 637, 638, 639, 640, 641, 642, 643, 644)
STRUCTURE = (
    'central-body', 'hammerhead', 'opaque-canopy-housing', 'right-outer-glass', 'right-inner-glass',
    'left-inner-glass', 'left-outer-glass', 'left-engine-pod', 'left-aft-pod-bridge',
    'left-forward-pod-bridge', 'left-engine-recess', 'left-front-pod-detail', 'right-engine-pod',
    'right-aft-pod-bridge', 'right-forward-pod-bridge', 'right-engine-recess', 'right-front-pod-detail',
    'left-pylon', 'left-rail-bed', 'left-outer-rail', 'left-inner-rail',
    'right-pylon', 'right-rail-bed', 'right-inner-rail', 'right-outer-rail',
)
PANEL_REGIONS = (
    ('hammerhead-panel', 7, 163, 4, {2, 3}),
    ('left-pod-panel', 167, 327, 5, {0, 3}),
    ('right-pod-panel', 332, 492, 5, {0, 3}),
    ('central-body-panel', 493, 637, 4, {2, 3}),
)


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def document(data):
    return (json.dumps(data, indent=2, allow_nan=False) + '\n').encode()


def authenticate(folder):
    raw = {name: (folder / name).read_bytes() for name in PINS}
    for name, data in raw.items():
        require(hashlib.sha256(data).hexdigest() == PINS[name], 'changed SH026 original: ' + name)
    resource = tomllib.loads(raw['resource.toml'].decode())
    contract = tomllib.loads(raw['contract.toml'].decode())
    export = json.loads(raw['pre-bare-native-export.json'])
    require(resource['id'] == contract['id'] == export['id'] == 'SH026'
            and resource['name'] == contract['name'] == 'Harrier', 'wrong original identity')
    require(export['sourcePath'] == contract['source']['geometryFile'] == LEGACY_PATH
            and export['sourceSHA256'] == contract['source']['geometrySHA256'] == SOURCE_SHA
            and len(export['nativeDesign']['faces']) == 2852, 'wrong original pre-bare export')
    detail, native = resource['design'], resource['nativeDesign']
    require(all('uniform30/4.192 authoringmetres/sourceunit' in mesh['source']
                for mesh in (detail, native))
            and 'fourpreviewmissilesremoved' in native['source'], 'wrong active bare metre source')
    require(NATIVE[1] == contract['geometry']['nativeFaceLimit'] == 512, 'wrong native face limit')
    return detail, native


def source_bounds(mesh, counts):
    coords, faces = mesh['vertices'], mesh['faces']
    require((len(coords), len(faces)) == counts[:2], 'wrong original source counts')
    require(mesh['license'] == mesh['appearanceLicense'] == 'MIT'
            and mesh['paletteName'] == 'Orange / black accents', 'wrong appearance provenance')
    require([item['asset'] for item in mesh['textures']] == TEXTURES
            and all(t['repeat'] and t['license'] == 'MIT' for t in mesh['textures']),
            'wrong original shared maps')
    require(all(len(p) == 3 and all(math.isfinite(v) for v in p) for p in coords),
            'invalid original coordinates')
    require(all(len(f) == len(set(f)) == 3 and all(type(i) is int and 0 <= i < len(coords) for i in f)
                for f in faces), 'invalid original triangle')
    return [[func(axis) for axis in zip(*coords, strict=True)] for func in (min, max)]


def verified_components(mesh, counts):
    groups = components(mesh)
    require(len(groups) == counts[2], 'wrong original component count')
    owners = {v: i for i, indices in enumerate(groups) for v in indices}
    materials = [set() for _ in groups]
    directed = Counter()
    for face, material in zip(mesh['faces'], mesh['faceMaterials'], strict=True):
        materials[owners[face[0]]].add(material)
        directed.update(pairwise(face + face[:1]))
    require(all(n == directed[b, a] == 1 for (a, b), n in directed.items()),
            'original mesh is missing a paired outward edge')
    return groups, materials


def partition(detail, native):
    detailed, mats = verified_components(detail, DETAIL)
    native_groups, _ = verified_components(native, NATIVE)
    named = dict(zip(NATIVE_DETAIL, STRUCTURE, strict=True))
    labels = {i: name for i, name in named.items()}
    for title, start, stop, stride, mounts in PANEL_REGIONS:
        for i in range(start, stop, stride):
            require(mats[i] <= mounts and all(x == {4} for x in mats[i + 1:i + stride]),
                    'changed local vent-panel structure')
            labels.update((part, f'{title}-{(i - start) // stride}') for part in range(i, i + stride))
    for i in range(645, DETAIL[2]):
        require(mats[i] == {7}, 'changed small source service panel')
        labels[i] = f'centre-service-panel-{i - 645}'
    require(set(labels) == set(range(DETAIL[2])), 'missing local support group')
    names = list(dict.fromkeys(labels[i] for i in range(DETAIL[2])))
    require(len(names) == PIECE_COUNT, 'wrong local support count')
    slots = {name: i for i, name in enumerate(names)}

    def assign(mesh, groups, part_names):
        assigned = [None] * len(mesh['vertices'])
        for group, label in zip(groups, part_names, strict=True):
            for i in group:
                assigned[i] = slots[label]
        require(all(assigned[a] == assigned[b] == assigned[c] for a, b, c in mesh['faces']),
                'source triangle crosses local supports')
        return bytes(assigned)

    for old, new in zip(NATIVE_DETAIL, native_groups, strict=True):
        original = {tuple(detail['vertices'][i]) for i in detailed[old]}
        converted = {tuple(native['vertices'][i]) for i in new}
        require(len(original & converted) >= 6, 'active native part pairing changed')
    return (names, assign(detail, detailed, [labels[i] for i in range(DETAIL[2])])
            + assign(native, native_groups, STRUCTURE))


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
                'invalid Harrier visual face')
        material = mesh['materials'][material_number]
        rgba = ((material['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0))
        color = list(rgba)
        n = [v / length for v in normal]
        coordinates = mapping['uv']
        if slot == -1:
            require(not coordinates, 'unmapped Harrier face has unexpected UVs')
            coordinates = ((0.0, 0.0),) * 3
        for p, uv in zip(points, coordinates, strict=True):
            data.extend(struct.pack('<12fII', *p, *n, *color, *uv, slot & UNMAPPED, int(material['emissive'])))
    return bytes(data)


def prepare(folder):
    detail, native = authenticate(folder)
    detail_bounds, native_bounds = source_bounds(detail, DETAIL), source_bounds(native, NATIVE)
    require(all(math.isclose(hi[2] - lo[2], 30.0, rel_tol=0, abs_tol=1e-12)
                for lo, hi in (detail_bounds, native_bounds)), 'changed authored metre length')
    names, memberships = partition(detail, native)
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
        'id': 'SH026', 'name': 'Harrier', 'legacyRevision': LEGACY,
        'sourceUnits': 'metres', 'metresPerSourceUnit': 1, 'authoringMetresPerOriginalUnit': '1875/262',
        'detail': dict(zip(('vertices', 'faces', 'components'), DETAIL, strict=True), bounds=detail_bounds),
        'native': dict(zip(('vertices', 'faces', 'components'), NATIVE, strict=True), bounds=native_bounds,
                       faceLimit=512, activeInput='resource.toml'),
        'historicalNative': {'faces': 2852, 'sourcePath': LEGACY_PATH, 'sourceSha256': SOURCE_SHA},
        'localVentPanelSupports': 139,
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
            require(out.read_bytes() == data, 'changed SH026 encoding: ' + name)
        else:
            out.write_bytes(data)


if __name__ == '__main__':
    main()
