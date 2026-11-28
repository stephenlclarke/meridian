"""Build the schema-one inspection definition for the original SH043 Eagle Cruiser."""
import argparse
import json
import math
import tomllib

from atomic_asset_output import checked_directory, checked_file, replace_output
from prepare_eagle_cruiser_geometry import ASSETS, DETAIL, PINS, ROOT, authenticate, prepare, require
from verify_eagle_cruiser_geometry import verify

INTEGRATION = ROOT / 'integration/sh043-eagle-cruiser'
OUTPUT = INTEGRATION / 'EAGLE-CRUISER-STUDIO.json'
PROBE_RADIUS_METRES = .1


def definition():
    detail, native = authenticate(ASSETS)
    derived = prepare(ASSETS)
    provenance = json.loads(derived['SOURCE-GEOMETRY.json'])
    groups = derived['source-piece-membership.bin']
    finite = json.loads((ASSETS / 'finite-dry-source.json').read_bytes())['emptyDryReference']
    routes = json.loads((INTEGRATION / 'CONTACT-CANDIDATE.json').read_bytes())['routes']
    clearance = verify()['clearanceMillimetres']
    probes = []
    for route in routes:
        name = route['name']
        require(clearance[name] > PROBE_RADIUS_METRES * 1000, 'not enough witnessed clearance')
        start = [math.fsum(axis) / 3 for axis in zip(*route['triangle'], strict=True)]
        end = [a - b for a, b in zip(start, route['travel'], strict=True)]
        probes.append({'name': name, 'start': start, 'end': end, 'radiusMetres': PROBE_RADIUS_METRES})

    def mesh(original, members):
        return {'vertices': original['vertices'], 'triangles': original['faces'], 'groups': list(members)}

    return {
        'schema': 1,
        'id': 'SH043',
        'name': 'Eagle Cruiser',
        'metresPerUnit': 1,
        'roundingMetres': .02,
        'detail': mesh(detail, groups[:DETAIL[0]]),
        'native': mesh(native, groups[DETAIL[0]:]),
        'materials': [
            {'name': item['name'], 'rgb': [item['colour'] >> shift & 255 for shift in (24, 16, 8)],
             'alpha': item['colour'] & 255, 'emissive': item['emissive']}
            for item in detail['materials']
        ],
        'faceMaterials': detail['faceMaterials'],
        'faceTextures': [
            {'uv': mapping['uv'], 'texture': detail['textures'][mapping['texture']]['asset']
             if mapping['texture'] >= 0 else None}
            for mapping in detail['faceTextures']
        ],
        'engineering': {
            'massKg': finite['massKg'],
            'centerOfMass': finite['centreMetres'],
            'inertia': [finite['inertiaAboutCentreOfMassKgM2'][i:i + 3] for i in range(0, 9, 3)],
        },
        'flight': {'maxSpeed': tomllib.loads((ASSETS / 'resource.toml').read_text())['game']['speed']},
        'clearances': probes,
        'provenance': {
            'sourceId': 'SH043',
            'revision': provenance['legacyRevision'],
            'sourceHashes': PINS,
            'derivedHashes': provenance['outputs'],
            'engineeringReference': finite['id'],
            'license': (ASSETS / 'LICENSE').read_text(),
            'originalMetresPerUnit': 1,
            'scope': 'catalogue inspection only; no operating authority',
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    checked_directory(OUTPUT.parent)
    checked_file(OUTPUT)
    data = (json.dumps(definition(), allow_nan=False, separators=(',', ':')) + '\n').encode()
    if args.check:
        require(OUTPUT.read_bytes() == data, 'changed Eagle Cruiser inspection definition')
    else:
        replace_output(OUTPUT, data)


if __name__ == '__main__':
    main()
