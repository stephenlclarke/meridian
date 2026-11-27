"""Generate the Hull Studio schema 1 definition from the qualified SH029 source."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import tomllib

from prepare_okapi_geometry import (
    ASSETS, DETAIL, PINS, authenticate, prepare, require,
)
from verify_okapi_geometry import INTEGRATION, verify


OUTPUT = INTEGRATION / 'OKAPI-STUDIO.json'
REFERENCE_REVISION = 'd03f1b4ecfd7588bb6b27763e31c230ebe1fbf5a'
REFERENCE_HASHES = {
    'engineering-budget.json': 'f64a24c2863cf047f852d99313ca7ee131ebcda40c3e140f43572d6fbf3aa827',
    'finite-empty-reference.json': 'ad07ebd5e5ed80be9f5bc62b2da23c6d46892112bb74f0a7ec407016b93b405f',
}


def definition():
    detail, native = authenticate(ASSETS)
    derived = prepare(ASSETS)
    geometry = json.loads(derived['SOURCE-GEOMETRY.json'])
    rounding = json.loads(derived['PIECES.json'])['roundingMetres']
    groups = derived['source-piece-membership.bin']
    for name, expected in REFERENCE_HASHES.items():
        require(hashlib.sha256((ASSETS / name).read_bytes()).hexdigest() == expected,
                'Changed original Okapi physical reference ' + name)
    require(detail['materials'] == native['materials'], 'Original palettes differ')
    fit = verify()['clearanceMillimetres']
    routes = json.loads((INTEGRATION / 'FIXTURES.json').read_bytes())['routes']
    probes = []
    for route in routes:
        name = route['name']
        radius = .020 if name.endswith('-top-rail-gap') else .100
        require(fit[name] > radius * 1000, 'no source clearance for ' + name)
        start = [math.fsum(axis) / 3 for axis in zip(*route['triangle'], strict=True)]
        end = [a - b for a, b in zip(start, route['travel'], strict=True)]
        probes.append({'name': name, 'start': start, 'end': end, 'radiusMetres': radius})

    def mesh(original, membership):
        return {'vertices': original['vertices'], 'triangles': original['faces'],
                'groups': list(membership)}

    return {
        'schema': 1,
        'id': geometry['id'],
        'name': geometry['name'],
        'metresPerUnit': geometry['metresPerSourceUnit'],
        'roundingMetres': rounding,
        'detail': mesh(detail, groups[:DETAIL[0]]),
        'native': mesh(native, groups[DETAIL[0]:]),
        'materials': [
            {'name': row['name'], 'rgb': [row['colour'] >> shift & 255 for shift in (24, 16, 8)],
             'alpha': row['colour'] & 255, 'emissive': row['emissive']}
            for row in detail['materials']
        ],
        'faceMaterials': detail['faceMaterials'],
        'nativeFaceMaterials': native['faceMaterials'],
        'faceTextures': [
            {'uv': row['uv'], 'texture': detail['textures'][row['texture']]['asset']
             if row['texture'] >= 0 else None}
            for row in detail['faceTextures']
        ],
        'engineering': None,
        'flight': {'maxSpeed': tomllib.loads((ASSETS / 'resource.toml').read_text())['game']['speed']},
        'clearances': probes,
        'provenance': {
            'sourceId': geometry['id'],
            'revision': geometry['legacyRevision'],
            'sourceHashes': PINS,
            'derivedHashes': geometry['outputs'],
            'originalPhysicalReferences': {'revision': REFERENCE_REVISION, 'sha256': REFERENCE_HASHES},
            'license': (ASSETS / 'LICENSE').read_text(),
            'originalMetresPerUnit': geometry['metresPerSourceUnit'],
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=OUTPUT)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = (json.dumps(definition(), allow_nan=False, separators=(',', ':')) + '\n').encode()
    if args.check:
        require(args.output.read_bytes() == data, 'changed Okapi Hull Studio definition')
    else:
        args.output.write_bytes(data)


if __name__ == '__main__':
    main()
