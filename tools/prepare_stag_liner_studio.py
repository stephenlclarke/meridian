"""Generate the Hull Studio schema 1 definition from the qualified SH035 source."""
import argparse
import json
import math
from pathlib import Path
import tempfile
import tomllib

from prepare_stag_liner_geometry import (
    ASSETS, DETAIL, PINS, authenticate, prepare, require,
)
from verify_stag_liner_geometry import INTEGRATION, ROOT, verify


OUTPUT = INTEGRATION / 'STAG-LINER-STUDIO.json'
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


def checked_output(selected):
    """Allow the canonical definition or a scratch JSON file beneath the system temp directory."""
    path = selected.absolute()
    if '..' in path.parts or path.suffix != '.json':
        raise ValueError('Invalid Stag Liner Hull Studio output filename')
    if path != OUTPUT:
        temporary = Path(tempfile.gettempdir()).absolute()
        scratch = temporary.resolve(strict=True)
        if path.is_relative_to(temporary):
            path = scratch / path.relative_to(temporary)
        if not path.is_relative_to(scratch) or path.is_relative_to(ROOT):
            raise ValueError('Stag Liner Hull Studio output must be the definition or a temporary JSON file')
    if not path.parent.is_dir() or path.parent.resolve(strict=True) != path.parent:
        raise ValueError('Stag Liner Hull Studio output directory is missing or redirects')
    if path.is_symlink() or path.resolve() != path or (path.exists() and not path.is_file()):
        raise ValueError('Stag Liner Hull Studio output must be a regular file without redirects')
    return path


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=OUTPUT,
                        help='canonical definition or a JSON file in the system temporary directory')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args(argv)
    output = checked_output(args.output)
    data = (json.dumps(definition(), allow_nan=False, separators=(',', ':')) + '\n').encode()
    if args.check:
        require(output.read_bytes() == data, 'changed Stag Liner Hull Studio definition')
    else:
        output.write_bytes(data)


if __name__ == '__main__':
    main()
