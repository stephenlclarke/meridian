"""Import exact a11 Git blobs: SH028 display plus SH028/SH025 physical station-loop data."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
REVISION = 'a11fe16924d87d06021816f9037952afcbac08db'
PINS = {
    'SH025': 'aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883',
    'SH028': '65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9',
}


def read_blob(repository, identifier):
    sources = {
        'SH025': REVISION + ':Sources/L337Core/Resources/Ships/SH025.toml',
        'SH028': REVISION + ':Sources/L337Core/Resources/Ships/SH028.toml',
    }
    if identifier not in sources:
        raise ValueError('Unselected source identifier')
    directory = Path(repository).resolve(strict=True)
    if not directory.is_dir():
        raise ValueError('Source repository must be a directory')
    # Only fixed blob specifications reach argv; the caller's directory is a cwd.
    raw = subprocess.check_output(
        ['git', '--no-pager', 'show', '--no-ext-diff', '--no-textconv', sources[identifier]],
        cwd=directory,
    )
    if hashlib.sha256(raw).hexdigest() != PINS[identifier]:
        raise ValueError('Immutable source hash mismatch: ' + identifier)
    return raw


def design(source, key):
    result = source[key]
    if result['license'] != 'MIT' or result['appearanceLicense'] != 'MIT':
        raise ValueError('Unexpected source notice')
    vertices = result['vertices']
    faces = result['faces']
    if not vertices or any(len(v) != 3 or any(not math.isfinite(x) or abs(x) > 1e12 for x in v) for v in vertices):
        raise ValueError('Invalid vertex data')
    if any(len(f) != 3 or any(not isinstance(i, int) or not 0 <= i < len(vertices) for i in f) for f in faces):
        raise ValueError('Invalid triangle indices')
    return result


def physical_bytes(source):
    mesh = design(source, 'nativeDesign')
    output = bytearray(struct.pack('<8sII', b'MPHYS001', len(mesh['vertices']), len(mesh['faces'])))
    for vertex in mesh['vertices']:
        output.extend(struct.pack('<3d', *vertex))
    for face in mesh['faces']:
        output.extend(struct.pack('<3I', *face))
    return bytes(output)


def display_bytes(source):
    mesh = design(source, 'design')
    if len(mesh['faces']) != len(mesh['faceMaterials']):
        raise ValueError('Missing face material')
    output = bytearray()
    for face, index in zip(mesh['faces'], mesh['faceMaterials'], strict=True):
        if not 0 <= index < len(mesh['materials']):
            raise ValueError('Invalid material')
        color = mesh['materials'][index]['colour']
        rgb = [((color >> shift) & 255) / 255 for shift in (24, 16, 8)]
        linear = [c / 12.92 if c <= .04045 else ((c + .055) / 1.055) ** 2.4 for c in rgb]
        for vertex in face:
            output.extend(struct.pack('<6f', *mesh['vertices'][vertex], *linear))
    return bytes(output)


def port_bytes(source):
    port = source['nativeDocking']
    values = [port[k][axis] for k in ('center', 'outward', 'up') for axis in ('x', 'y', 'z')]
    output = bytearray(struct.pack('<8s10dI', b'MPORT001', *values, port['tunnelDepth'], len(port['aperture'])))
    for point in port['aperture']:
        output.extend(struct.pack('<3d', *(point[a] for a in ('x', 'y', 'z'))))
    return bytes(output)


def outputs(raw_ship, raw_station):
    for identifier, raw in [('SH025', raw_ship), ('SH028', raw_station)]:
        if hashlib.sha256(raw).hexdigest() != PINS[identifier]:
            raise ValueError('Immutable source hash mismatch: ' + identifier)
    ship, station = tomllib.loads(raw_ship.decode()), tomllib.loads(raw_station.decode())
    if (len(station['design']['vertices']), len(station['design']['faces'])) != (5888, 10056):
        raise ValueError('Selected display geometry differs')
    if (len(station['nativeDesign']['vertices']), len(station['nativeDesign']['faces'])) != (272, 488):
        raise ValueError('Selected collision LOD differs')
    result = {
        'sh028.mesh': display_bytes(station),
        'sh028.physics': physical_bytes(station),
        'sh025.physics': physical_bytes(ship),
        'sh028.port': port_bytes(station),
    }
    manifest = {
        'sourceRevision': REVISION,
        'sourceFiles': [{'id': key, 'path': f'Sources/L337Core/Resources/Ships/{key}.toml', 'sha256': PINS[key], 'bytes': len(raw)} for key, raw in [('SH025', raw_ship), ('SH028', raw_station)]],
        'outputs': {name: {'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)} for name, data in result.items()},
        'station': {'id': 'SH028', 'name': station['name'], 'displayVertices': 5888, 'displayTriangles': 10056, 'nativeVertices': 272, 'nativeTriangles': 488, 'port': station['nativeDocking'], 'geometryContext': station['geometryContext']},
        'player': {'id': 'SH025', 'nativeVertices': len(ship['nativeDesign']['vertices']), 'nativeTriangles': len(ship['nativeDesign']['faces']), 'geometryContext': ship['geometryContext'], 'game': ship['game']},
        'displayFormat': 'Original triangle order LEf32 xyz + linearRGB,24B/vertex; no UV/textures',
        'physicalFormat': 'MPHYS001 + LEu32 vertex/triangle counts + LEf64 xyz + LEu32 triangle indices; source frame/values unchanged',
        'portFormat': 'MPORT001 + LEf64 center/outward/up/depth + LEu32 aperture count + LEf64 xyz',
        'notice': 'assets/legacy/Elite-L337-MIT.txt',
        'scope': 'SH028 bounded native collision LOD, not full display surface equivalence or legacy171 fleet acceptance. No original save/profile or mission authority imported.',
        'importer': 'tools/import_station.py',
    }
    result['station-manifest.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository', type=Path, help='Object database containing the exact a11 revision; working tree is never read')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = outputs(read_blob(args.repository, 'SH025'), read_blob(args.repository, 'SH028'))
    for name, data in result.items():
        path = ROOT / 'assets/legacy' / name
        if args.check:
            if path.read_bytes() != data:
                raise ValueError('Committed import mismatch: ' + name)
        else:
            path.write_bytes(data)
    print('Selected original station/hull display/physical/port data ' + ('match' if args.check else 'imported'))


if __name__ == '__main__':
    main()
