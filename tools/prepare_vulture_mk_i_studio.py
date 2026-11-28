"""Produce the SH037 Studio definition with adopted scale and certified native proxy."""

import argparse
import json
import math
from pathlib import Path

from prepare_vulture_mk_i_geometry import ASSETS, LEGACY, PINS, PROXY, SCALE, prepare, require, source_models
from verify_vulture_mk_i_geometry import INTEGRATION, verify


OUTPUT = INTEGRATION / 'VULTURE-MK-I-STUDIO.json'
PROBE_RADIUS_METRES = .10


def definition():
    resource, engineering, sources = source_models(ASSETS)
    detailed, membership = sources[0]
    proxy = json.loads((ASSETS / PROXY).read_bytes())
    geometry = json.loads(prepare(ASSETS)['SOURCE-GEOMETRY.json'])
    clearance_bounds = verify()['clearanceMillimetres']
    routes = json.loads((INTEGRATION / 'FIXTURES.json').read_bytes())['routes']
    probes = []
    for route in routes:
        name = route['name']
        require(clearance_bounds[name] > PROBE_RADIUS_METRES * 1000, 'no source clearance for ' + name)
        start = [math.fsum(axis) / 3 for axis in zip(*route['triangle'], strict=True)]
        end = [a - b for a, b in zip(start, route['travel'], strict=True)]
        probes.append({'name': name, 'start': start, 'end': end, 'radiusMetres': PROBE_RADIUS_METRES})
    tensor = engineering['inertiaAboutCentreOfMassKgM2']
    return {
        'schema': 1,
        'id': 'SH037',
        'name': resource['name'],
        'metresPerUnit': float(SCALE),
        'roundingMetres': .02,
        'detail': {'vertices': detailed['vertices'], 'triangles': detailed['faces'], 'groups': list(membership)},
        'native': {key: proxy[key] for key in ('vertices', 'triangles', 'groups')},
        'materials': [
            {'name': row['name'], 'rgb': [row['colour'] >> shift & 255 for shift in (24, 16, 8)],
             'alpha': row['colour'] & 255, 'emissive': row['emissive']}
            for row in detailed['materials']
        ],
        'faceMaterials': detailed['faceMaterials'],
        'engineering': {
            'massKg': engineering['dryMassKilograms'],
            'centerOfMass': engineering['centreOfMassMetres'],
            'inertia': [tensor[i:i+3] for i in (0, 3, 6)],
        },
        'flight': {'maxSpeed': resource['game']['speed']},
        'clearances': probes,
        'provenance': {
            'sourceId': 'SH037',
            'revision': LEGACY,
            'sourceHashes': PINS,
            'derivedHashes': geometry['outputs'],
            'nativeProxy': geometry['studioNativeProxy']['input'],
            'license': (ASSETS / 'LICENSE').read_text(),
            'originalMetresPerUnit': float(SCALE),
            'adoptedMetresPerSourceUnitExact': str(SCALE),
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=OUTPUT)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = (json.dumps(definition(), allow_nan=False, separators=(',', ':')) + '\n').encode()
    if args.check:
        require(args.output.read_bytes() == data, 'changed Vulture Mk I Studio definition')
    else:
        args.output.write_bytes(data)


if __name__ == '__main__':
    main()
