"""Extract original G01 route exposure; descriptive planning data grants no travel authority."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

from import_navigation import REVISION, SOURCE, SOURCE_SHA, read_blob

ROOT = Path(__file__).resolve().parents[1]


def convert(raw):
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA:
        raise ValueError('Original world catalogue hash mismatch')
    catalogue = json.loads(raw)
    systems = [s for s in catalogue['systems'] if s['galaxy'] == 0]
    if len(systems) != 256 or [s['system'] for s in systems] != list(range(256)):
        raise ValueError('Original G01 system IDs differ')
    output = bytearray(struct.pack('<8sH', b'MRISK001', 256))
    for system in systems:
        exposure = system['hostileFraction']
        if isinstance(exposure, bool) or not isinstance(exposure, (int, float)) or not 0 <= exposure <= 1:
            raise ValueError('Invalid original hostile fraction')
        output.extend(struct.pack('<Hd', system['system'], exposure))
    data = bytes(output)
    manifest = {
        'sourceRevision': REVISION, 'sourcePath': SOURCE, 'sourceSHA256': SOURCE_SHA,
        'sourceBytes': len(raw), 'sourceVersion': catalogue['version'],
        'output': 'assets/legacy/g01.route-exposure',
        'outputSHA256': hashlib.sha256(data).hexdigest(), 'outputBytes': len(data),
        'systems': 256, 'format': 'MRISK001 + LEu16 count; per row LEu16 system and LEf64 hostileFraction',
        'notice': 'assets/legacy/Elite-L337-MIT.txt',
        'scope': 'Immutable original G01 arrival exposure for read-only route scoring; no actor, encounter, market, refuel or travel authority',
    }
    return data, (json.dumps(manifest, indent=2) + '\n').encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data, manifest = convert(read_blob(args.repository))
    for name, value in [('g01.route-exposure', data), ('route-exposure-manifest.json', manifest)]:
        path = ROOT / 'assets/legacy' / name
        if args.check:
            if path.read_bytes() != value:
                raise ValueError('Route exposure import differs: ' + name)
        else:
            path.write_bytes(value)
    print('Original G01 route exposure ' + ('matches' if args.check else 'imported'))


if __name__ == '__main__':
    main()
