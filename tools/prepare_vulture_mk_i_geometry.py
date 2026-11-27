"""Encode the two original SH037 meshes and certify its separate conservative proxy."""

import argparse
from collections import Counter
from fractions import Fraction
import hashlib
from itertools import pairwise
import json
import math
from pathlib import Path
import struct
import tomllib

from import_goshawk_envelope_source import components, pin


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/fleet/sh037'
DETAIL = (5132, 8272, 501)
NATIVE = (1812, 3016, 155)
PIECES = (
    'pressure-hull', 'cockpit', 'positive-shoulder', 'positive-drive', 'positive-sensor',
    'negative-shoulder', 'negative-drive', 'negative-sensor',
)
LEGACY = 'a11fe16924d87d06021816f9037952afcbac08db'
SCALE = Fraction(6000, 611)
SOURCE_PATH = 'replacement-hulls/SH037-design.toml'
SOURCE_SHA = 'c00f8163a07a5c90c2867334d03e275311afb3060ea5434841b6ab6005553f23'
PINS = {
    'contract.toml': '1205c491e12819aaed94eff59ab4c8e22bc52cea61abbdbdb77834fa03301099',
    'engineering.json': '0043c48d88c2c86063ad8aaaf99870dba03fbef733d5ed553f0db3066db50345',
    'native-export.json': 'e56098421799168e67178fe5fee7c8b5d829142af921a24e425d7f508d556ca6',
    'resource.toml': '4719491723bbaffa5901691987a7b980053d5d0b63c8afc10f3882c31139de78',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
MAGIC = b'MSH037V1'
VISUAL_MAGIC = b'VUL1VIZ1'
PROXY = 'native-proxy.json'


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def document(data):
    return (json.dumps(data, indent=2, allow_nan=False) + '\n').encode()


def authenticate(folder):
    raw = {name: (folder / name).read_bytes() for name in PINS}
    for name, data in raw.items():
        require(hashlib.sha256(data).hexdigest() == PINS[name], 'changed SH037 original: ' + name)
    resource, contract = (tomllib.loads(raw[name].decode()) for name in ('resource.toml', 'contract.toml'))
    export, engineering = (json.loads(raw[name]) for name in ('native-export.json', 'engineering.json'))
    require(resource['id'] == contract['id'] == export['id'] == engineering['id'] == 'SH037'
            and resource['name'] == contract['name'] == 'Vulture Mk I', 'wrong SH037 variant')
    require(export['sourcePath'] == contract['source']['geometryFile'] == SOURCE_PATH
            and export['sourceSHA256'] == contract['source']['geometrySHA256']
            == engineering['detailedSourceSHA256'] == SOURCE_SHA
            and export.get('coordinateUnits') is None, 'wrong SH037 relative source')
    require(engineering['contractSHA256'] == PINS['contract.toml']
            and Fraction(engineering['metresPerSourceUnitExact']) == SCALE
            and engineering['metresPerSourceUnit'] == float(SCALE)
            and engineering['targetDetailedLengthMetres'] == 30.0
            and engineering['dryMassKilograms'] == 51330, 'wrong adopted SH037 engineering')
    require(resource['nativeDesign'] == export['nativeDesign']
            and contract['geometry']['nativeFaceLimit'] == 512, 'wrong SH037 native archive')
    return resource, engineering


def owners(mesh, counts, per_side, start_drive, start_body, sensor):
    vertices, faces = mesh['vertices'], mesh['faces']
    groups = components(mesh)
    require((len(vertices), len(faces), len(groups)) == counts, 'wrong SH037 topology')
    require(mesh['license'] == mesh['appearanceLicense'] == 'MIT'
            and mesh['paletteName'] == 'Russet / graphite / bronze'
            and 'faceTextures' not in mesh, 'wrong SH037 original appearance')
    slots = [None] * len(vertices)
    for index, group in enumerate(groups):
        if index < 5:
            slot = int(index != 0)
        else:
            side, offset = divmod(index - 5, per_side)
            ordinal = offset + 5
            if ordinal < 29:
                slot = 1
            elif ordinal < start_drive:
                slot = 2 + 3 * side
            elif ordinal < start_body:
                slot = 3 + 3 * side
            elif ordinal < sensor:
                slot = 0
            else:
                slot = 4 + 3 * side
        for i in group:
            slots[i] = slot
    require(all(len(face) == len(set(face)) == 3 and all(slots[i] == slots[face[0]] for i in face)
                for face in faces), 'SH037 face leaves its original local assembly')
    return bytes(slots)


def source_models(folder):
    resource, engineering = authenticate(folder)
    detail, native = resource['design'], resource['nativeDesign']
    return resource, engineering, (
        (detail, owners(detail, DETAIL, 248, 78, 188, 252)),
        (native, owners(native, NATIVE, 75, 34, 63, 79)),
    )


def metres(point):
    return [value * float(SCALE) for value in point]


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b, strict=True))


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def dot(a, b):
    return sum(x * y for x, y in zip(a, b, strict=True))


def dyadic(points):
    ratios = [[v.as_integer_ratio() for v in point] for point in points]
    scale = max(denominator for point in ratios for _, denominator in point)
    return [tuple(n * (scale // d) for n, d in point) for point in ratios]


def certify_proxy(proxy, source):
    require(proxy['id'] == 'SH037' and proxy['kind'] == 'authored-conservative-native-proxy'
            and proxy['sourcePins'] == PINS, 'wrong SH037 native proxy identity')
    vertices, faces, slots = proxy['vertices'], proxy['triangles'], proxy['groups']
    require(0 < len(faces) <= 512 and len(vertices) == len(slots)
            and set(slots) == set(range(len(PIECES)))
            and all(len(point) == 3 and all(math.isfinite(v) for v in point) for point in vertices),
            'invalid SH037 proxy cardinality')
    require(all(len(face) == len(set(face)) == 3
                and all(type(i) is int and 0 <= i < len(vertices) and slots[i] == slots[face[0]] for i in face)
                for face in faces), 'proxy face leaves its support')
    require(set(i for face in faces for i in face) == set(range(len(vertices))), 'unused proxy vertex')
    source_vertices = [metres(point) for model, _ in source for point in model['vertices']]
    source_groups = b''.join(groups for _, groups in source)
    cloud = dyadic(source_vertices + [metres(point) for point in vertices])
    packed = cloud[len(source_vertices):]
    edges = Counter()
    groups = [[p for p, group in zip(cloud, source_groups) if group == slot] for slot in range(len(PIECES))]
    for face in faces:
        a, b, c = map(packed.__getitem__, face)
        normal = cross(sub(b, a), sub(c, a))
        require(any(normal), 'degenerate SH037 proxy triangle')
        boundary = dot(normal, a)
        require(max(dot(normal, p) for p in groups[slots[face[0]]]) < boundary,
                'SH037 proxy fails strict original vertex containment')
        for edge in pairwise((*face, face[0])):
            edges[edge] += 1
    require(all(count == edges[(b, a)] == 1 for (a, b), count in edges.items()),
            'SH037 proxy triangle surface is not closed and consistently oriented')


def encode_visual(model):
    vertices = [metres(v) for v in model['vertices']]
    data = bytearray(struct.pack('<8sII', VISUAL_MAGIC, len(model['faces']) * 3, 56))
    for face, slot in zip(model['faces'], model['faceMaterials'], strict=True):
        points = [vertices[i] for i in face]
        raw_normal = cross(sub(points[1], points[0]), sub(points[2], points[0]))
        length = math.sqrt(dot(raw_normal, raw_normal))
        require(length > 0.0, 'degenerate original SH037 face')
        material = model['materials'][slot]
        tint = [(material['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
        normal = [v / length for v in raw_normal]
        for point in points:
            data.extend(struct.pack('<12fII', *point, *normal, *tint, 0.0, 0.0,
                                    0xffff_ffff, int(material['emissive'])))
    return bytes(data)


def prepare(folder):
    _, _, originals = source_models(folder)
    raw_proxy = (folder / PROXY).read_bytes()
    proxy = json.loads(raw_proxy)
    certify_proxy(proxy, originals)
    model_entries = [*originals, ({'vertices': proxy['vertices'], 'faces': proxy['triangles']}, bytes(proxy['groups']))]
    counts = [len(model['vertices']) for model, _ in model_entries]
    vertices = bytearray(struct.pack('<8sIII', MAGIC, *counts))
    faces = bytearray()
    membership = bytearray()
    offset = 0
    source_bounds = []
    for model, groups in model_entries:
        points = [metres(v) for v in model['vertices']]
        source_bounds.append([[f(axis) for axis in zip(*points)] for f in (min, max)])
        vertices.extend(b''.join(struct.pack('<3d', *point) for point in points))
        faces.extend(b''.join(struct.pack('<3I', *(i + offset for i in face)) for face in model['faces']))
        membership.extend(groups)
        offset += len(points)
    output = {
        'source.vertices': bytes(vertices),
        'source-triangles.bin': bytes(faces),
        'source-piece-membership.bin': bytes(membership),
        'PIECES.json': document({'pieces': PIECES, 'roundingMetres': .02}),
        'visual.bin': encode_visual(originals[0][0]),
        'original-native-visual.bin': encode_visual(originals[1][0]),
    }
    output['SOURCE-GEOMETRY.json'] = document({
        'id': 'SH037', 'name': 'Vulture Mk I', 'legacyRevision': LEGACY,
        'sourceUnits': 'relative', 'metresPerSourceUnit': float(SCALE),
        'metresPerSourceUnitExact': str(SCALE), 'scaleBasis': 'Adopted original engineering scale',
        'detail': dict(zip(('vertices', 'faces', 'components'), DETAIL), bounds=source_bounds[0]),
        'originalNative': dict(zip(('vertices', 'faces', 'components'), NATIVE), bounds=source_bounds[1]),
        'studioNativeProxy': {'vertices': counts[2], 'faces': len(proxy['triangles']),
                              'faceLimit': 512, 'bounds': source_bounds[2], 'input': pin(raw_proxy)},
        'shapePieces': len(PIECES), 'pins': PINS, 'outputs': {name: pin(data) for name, data in output.items()},
    })
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ASSETS)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, data in prepare(args.assets).items():
        target = args.assets / name
        if args.check:
            require(target.read_bytes() == data, 'changed encoded SH037 file: ' + name)
        else:
            target.write_bytes(data)


if __name__ == '__main__':
    main()
