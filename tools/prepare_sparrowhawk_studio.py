"""Generate the Hull Studio schema 1 definition from the qualified SH030 source."""
import argparse
import json
import math
from pathlib import Path
import tomllib

from prepare_sparrowhawk_geometry import (
    ASSETS, DETAIL, PINS, authenticate, prepare, require,
)
from verify_sparrowhawk_geometry import INTEGRATION, verify


OUTPUT = INTEGRATION / 'SPARROWHAWK-STUDIO.json'
PROBE_RADIUS_METRES = .10


def definition():
    detail, native = authenticate(ASSETS)
    derived = prepare(ASSETS)
    geometry = json.loads(derived['SOURCE-GEOMETRY.json'])
    rounding = json.loads(derived['PIECES.json'])['roundingMetres']
    groups = derived['source-piece-membership.bin']
    fit = verify()['clearanceMillimetres']
    routes = json.loads((INTEGRATION / 'FIXTURES.json').read_bytes())['routes']
    probes = []
    for route in routes:
        name = route['name']
        require(fit[name] > PROBE_RADIUS_METRES * 1000, 'no source clearance for ' + name)
        start = [math.fsum(axis) / 3 for axis in zip(*route['triangle'], strict=True)]
        end = [a - b for a, b in zip(start, route['travel'], strict=True)]
        probes.append({'name': name, 'start': start, 'end': end, 'radiusMetres': PROBE_RADIUS_METRES})

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
        require(args.output.read_bytes() == data, 'changed Sparrowhawk Hull Studio definition')
    else:
        args.output.write_bytes(data)


if __name__ == '__main__':
    main()
