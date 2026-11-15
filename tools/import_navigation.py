"""Extract the exact original G01 navigation data from immutable a11 Git blobs."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[1]
REVISION = 'a11fe16924d87d06021816f9037952afcbac08db'
SOURCE = 'Sources/L337Core/Resources/OriginalWorlds/catalogue.json'
SOURCE_SHA = '120543eedc02a705cce0e37d026125bf926ec22651cff9337ca5d943f2ffd99d'


def read_blob(repository):
    directory = Path(repository).resolve(strict=True)
    if not directory.is_dir():
        raise ValueError('Source repository must be a directory')
    raw = subprocess.check_output(
        ['git', '--no-pager', 'show', '--no-ext-diff', '--no-textconv', REVISION + ':' + SOURCE],
        cwd=directory,
    )
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA:
        raise ValueError('Original world catalogue hash mismatch')
    return raw


def convert(raw):
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA:
        raise ValueError('Original world catalogue hash mismatch')
    catalogue = json.loads(raw)
    systems = [s for s in catalogue['systems'] if s['galaxy'] == 0]
    if len(systems) != 256 or [s['system'] for s in systems] != list(range(256)):
        raise ValueError('Original G01 system IDs differ')
    output = bytearray(struct.pack('<8sH', b'MNAV0001', 256))
    for system in systems:
        name = system['name'].encode()
        if not name or len(name) > 255:
            raise ValueError('Invalid system name')
        if not all(math.isfinite(system[key]) for key in ('xLY', 'yLY')):
            raise ValueError('Invalid system coordinates')
        output.extend(struct.pack('<HddB', system['system'], system['xLY'], system['yLY'], len(name)))
        output.extend(name)
    data = bytes(output)
    manifest = {
        'sourceRevision': REVISION, 'sourcePath': SOURCE, 'sourceSHA256': SOURCE_SHA,
        'sourceBytes': len(raw), 'sourceVersion': catalogue['version'],
        'output': 'assets/legacy/g01.navigation',
        'outputSHA256': hashlib.sha256(data).hexdigest(), 'outputBytes': len(data),
        'systems': 256,
        'coordinateUnit': 'Cartesian LY; hypot(dx,dy), no anisotropic conversion/rounding',
        'format': 'MNAV0001 + LEu16 count; per row LEu16 system, LEf64 xLY/yLY, u8 UTF8-name length, name bytes',
        'notice': 'assets/legacy/Elite-L337-MIT.txt',
        'scope': 'Original G01 only; no galaxy fallback or gameplay authority from descriptive coordinates',
    }
    return data, (json.dumps(manifest, indent=2) + '\n').encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data, manifest = convert(read_blob(args.repository))
    for name, value in [('g01.navigation', data), ('navigation-manifest.json', manifest)]:
        path = ROOT / 'assets/legacy' / name
        if args.check:
            if path.read_bytes() != value:
                raise ValueError('Navigation import differs: ' + name)
        else:
            path.write_bytes(value)
    print('Original G01 navigation ' + ('matches' if args.check else 'imported'))


if __name__ == '__main__':
    main()
