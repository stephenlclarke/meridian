"""Prepare the immutable metre-authored SH012 geometry and detailed visual."""
import argparse
import hashlib
import json
import math
from collections import defaultdict
from pathlib import Path
import struct
import tomllib

from import_goshawk_envelope_source import components


ASSETS = Path(__file__).resolve().parents[1] / 'assets/fleet/sh012'
PINS = {
    'resource.toml': '21bc51b0e9526cf85c0b3d825e24e3cb899dbf15ac6dc7ffdd5885f38380b47c',
    'native-export.json': '91eaa94d9288a1f34bc0b1cd8ffa184035532a355f4214e08109a2e728ea1c38',
    'native-export-manifest.json': '118b05757dc54096faa15dd79801f38999b87fe9424f1b3513133ead94f71971',
    'contract.toml': 'a59ed199acd7c681bc4e590076b0853d19db9724c777c0aee483878d30fcb0df',
    'LICENSE': '033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766',
}
SOURCE_GEOMETRY = 'fleet-completion-20261019/batch-02-native-lod/SH012-design.toml'
SOURCE_DIGEST = '980854a3c77366b6c0e1d8f559ffcbdad7bbf2fe510988fdd86a380477f6e297'
TEXTURES = ['l337-patrol-diffuse', 'l337-strike-diffuse', 'l337-service-details']
DETAIL = (2308, 4496, 30)
NATIVE = (132, 228, 9)


def require(test, message):
    if not test:
        raise ValueError(message)


def pin(raw):
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def document(data):
    return (json.dumps(data, indent=2, allow_nan=False) + '\n').encode()


def authenticate(folder):
    raw = {name: (folder / name).read_bytes() for name in PINS}
    for name, data in raw.items():
        require(pin(data)['sha256'] == PINS[name], 'changed SH012 origin: ' + name)
    ship = tomllib.loads(raw['resource.toml'].decode())
    plan = tomllib.loads(raw['contract.toml'].decode())
    exported = json.loads(raw['native-export.json'])
    manifest = json.loads(raw['native-export-manifest.json'])
    require(ship['id'] == plan['id'] == exported['id'] == 'SH012', 'wrong Sable Liner identity')
    require(ship['name'] == plan['name'] == 'Sable Liner', 'wrong Sable Liner label')
    require(exported['coordinateUnits'] == 'metres', 'SH012 original metre authority missing')
    require(exported['sourcePath'] == plan['source']['geometryFile'] == SOURCE_GEOMETRY,
            'wrong original design path')
    require(exported['sourceSHA256'] == plan['source']['geometrySHA256'] == SOURCE_DIGEST,
            'wrong original design digest')
    require(exported['nativeDesign'] == ship['nativeDesign'], 'native export differs from staged source')
    expected = dict(pin(raw['native-export.json']), id='SH012', file='SH012.json',
                    sourcePath=SOURCE_GEOMETRY, sourceSHA256=SOURCE_DIGEST)
    require([r for r in manifest['models'] if r['id'] == 'SH012'] == [expected],
            'native export manifest mismatch')
    return ship['design'], ship['nativeDesign'], plan['geometry']['nativeFaceLimit']


def source_groups(model, counts):
    vertices, faces = model['vertices'], model['faces']
    require((len(vertices), len(faces)) == counts[:2], 'changed SH012 topology')
    require(model['paletteName'] == 'plum-gold' and model['license'] == 'MIT'
            and model['appearanceLicense'] == 'MIT', 'changed original presentation authority')
    require([item['asset'] for item in model['textures']] == TEXTURES, 'changed textures')
    require(all(len(f) == 3 and len(set(f)) == 3 and all(
        type(i) is int and 0 <= i < len(vertices) for i in f) for f in faces), 'invalid triangle')
    groups = components(model)
    require(len(groups) == counts[2], 'changed original index connectivity')
    by_vertex = {i: group[0] for group in groups for i in group}
    grouped = defaultdict(list)
    for i, face in enumerate(faces):
        owners = {by_vertex[v] for v in face}
        require(len(owners) == 1, 'triangle crossed an original component')
        grouped[owners.pop()].append(i)
    return grouped


def crescent_segments(faces, indices, start, stride, rings_per_piece, ring_count):
    pieces = defaultdict(list)
    for index in indices:
        rings = [(vertex - start) // stride for vertex in faces[index]]
        first, last = min(rings), max(rings)
        require(last < ring_count and last - first <= 1, 'triangle crossed a nonadjacent crescent ring')
        # Put the last cap with the previous span. A seam ring can appear on both pieces.
        pieces[min(first, ring_count - 2) // rings_per_piece].append(index)
    return list(pieces.values())


def partition(detailed, native, d_groups, n_groups):
    pairs = {0: 0, 890: 54, 1092: 72, 1100: 80, 1298: 90, 1500: 108, 1508: 116, 1706: 126}
    shared = [(f'assembly-{d}', ids, n_groups.get(pairs.get(d), []))
              for d, ids in d_groups.items() if d != 194]
    detail_ring = crescent_segments(detailed, d_groups[194], 194, 12, 6, 58)
    native_ring = crescent_segments(native, n_groups[18], 18, 4, 1, 9)
    return shared + [
        (f'detailed-crescent-{i}', ids, []) for i, ids in enumerate(detail_ring)
    ] + [
        (f'native-crescent-{i}', [], ids) for i, ids in enumerate(native_ring)
    ]


def encode_pieces(detailed, native, rows):
    require(len(rows) == 47, 'wrong conservative support count')
    payload = bytearray(struct.pack('<8sI', b'MSH12PC1', len(rows)))
    records = []
    for name, d, n in rows:
        ids = {v for i in d for v in detailed[i]} | {v + DETAIL[0] for i in n for v in native[i]}
        require(len(ids) >= 4, 'rounded support lacks sufficient source points')
        payload.extend(struct.pack('<I', len(ids)))
        payload.extend(b''.join(struct.pack('<I', i) for i in sorted(ids)))
        records.append({'piece': name, 'points': len(ids), 'detailedFaces': d, 'nativeFaces': n})
    require(sorted(i for _, d, _ in rows for i in d) == list(range(len(detailed))),
            'detailed face assigned incorrectly')
    require(sorted(i for _, _, n in rows for i in n) == list(range(len(native))),
            'native face assigned incorrectly')
    return bytes(payload), records


def encode_visual(model):
    require(len(model['faceMaterials']) == len(model['faceTextures']) == DETAIL[1],
            'missing original detail appearance')
    out = bytearray(struct.pack('<8sII', b'SBLRVIZ1', DETAIL[1] * 3, 56))
    for face, material_index, mapping in zip(
            model['faces'], model['faceMaterials'], model['faceTextures'], strict=True):
        positions = [model['vertices'][i] for i in face]
        x, y, z = (b - a for a, b in zip(positions[0], positions[1], strict=True))
        u, v, w = (b - a for a, b in zip(positions[0], positions[2], strict=True))
        normal = (y * w - z * v, z * u - x * w, x * v - y * u)
        length = math.sqrt(sum(n * n for n in normal))
        require(length > 0 and math.isfinite(length) and -1 <= mapping['texture'] < len(TEXTURES),
                'invalid original visual face')
        material = model['materials'][material_index]
        color = [(material['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
        for position, uv in zip(positions, mapping['uv'], strict=True):
            out.extend(struct.pack('<12fII', *position, *(n / length for n in normal), *color,
                                   *uv, mapping['texture'] & 0xffff_ffff, int(material['emissive'])))
    return bytes(out)


def prepare(folder):
    detailed, native, limit = authenticate(folder)
    d_groups, n_groups = source_groups(detailed, DETAIL), source_groups(native, NATIVE)
    require(NATIVE[1] <= limit == 512, 'wrong legacy exact-native face limit')
    d_faces, n_faces = detailed['faces'], native['faces']
    pieces = partition(d_faces, n_faces, d_groups, n_groups)
    piece_bytes, records = encode_pieces(d_faces, n_faces, pieces)
    coordinates = detailed['vertices'] + native['vertices']
    vertices = struct.pack('<8sII', b'MSH012V1', DETAIL[0], NATIVE[0])
    vertices += b''.join(struct.pack('<3d', *p) for p in coordinates)
    faces = b''.join(struct.pack('<3I', *face) for face in d_faces)
    faces += b''.join(struct.pack('<3I', *(v + DETAIL[0] for v in face)) for face in n_faces)
    outputs = {'source.vertices': vertices, 'source-triangles.bin': faces,
               'piece-indices.bin': piece_bytes, 'PIECES.json': document(records),
               'visual.bin': encode_visual(detailed)}
    outputs['SOURCE-GEOMETRY.json'] = document({
        'id': 'SH012', 'name': 'Sable Liner', 'legacyRevision': 'a11fe16924d87d06021816f9037952afcbac08db',
        'scaleMetresPerSourceUnit': 1, 'originalNativeFaceLimit': limit,
        'detailed': dict(zip(('vertices', 'faces', 'components'), DETAIL, strict=True)),
        'native': dict(zip(('vertices', 'faces', 'components'), NATIVE, strict=True)),
        'pieces': len(pieces), 'roundingMetres': 0.02, 'palette': 'plum-gold', 'textures': TEXTURES,
        'visualUntexturedSlot': 4294967295,
        'inputs': PINS, 'outputs': {name: pin(raw) for name, raw in outputs.items()},
    })
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ASSETS)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, raw in prepare(args.assets).items():
        path = args.assets / name
        if args.check:
            require(path.read_bytes() == raw, 'changed SH012 encoding: ' + name)
        else:
            path.write_bytes(raw)
    print('SH012 source: 4,724 original triangles, 47 local rounded supports, plum-gold visual.')


if __name__ == '__main__':
    main()
