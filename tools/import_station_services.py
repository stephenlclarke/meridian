"""Import original Avenor/Renas Food seeds; quantities are initial, never current stock."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[1]
REVISION = 'a11fe16924d87d06021816f9037952afcbac08db'
SOURCES = {
    'manifest': ('Sources/L337Core/Resources/OriginalMarkets/manifest.json',
                 'fcc9180da8b415bb2ac0666acae197dc30f5862dac79d613fb6c6e6dfdb8e5c3'),
    'markets': ('Sources/L337Core/Resources/OriginalMarkets/markets.json',
                '4f7e7c9235f13f8151f2d53051828078f6c7d701ffd119c15031dd9137176c5d'),
}


def read_blob(repository, identifier):
    if identifier not in SOURCES:
        raise ValueError('Unselected service source')
    directory = Path(repository).resolve(strict=True)
    if not directory.is_dir():
        raise ValueError('Source repository must be a directory')
    path, expected = SOURCES[identifier]
    raw = subprocess.check_output(
        ['git', '--no-pager', 'show', '--no-ext-diff', '--no-textconv', REVISION + ':' + path],
        cwd=directory,
    )
    if hashlib.sha256(raw).hexdigest() != expected:
        raise ValueError('Immutable market source hash mismatch')
    return raw


def food_quote(markets, system):
    states = [s for s in markets['markets'] if s['galaxy'] == 0 and s['system'] == system]
    if len(states) != 1 or len(states[0]['quotes']) != 17:
        raise ValueError('Original station market differs')
    quotes = [q for q in states[0]['quotes'] if q['index'] == 0]
    if len(quotes) != 1:
        raise ValueError('Original Food quote differs')
    quote = quotes[0]
    if (quote['name'], quote['unit'], quote['epoch'], quote['system'], quote['worldVersion']) != (
            'Food', 't', 0, system, 'l337-world-1'):
        raise ValueError('Original Food identity differs')
    if not 0 <= quote['bidTenthsCR'] <= quote['askTenthsCR'] <= 1000000:
        raise ValueError('Invalid original price')
    if not 0 <= quote['stock'] <= (2 ** 63 - 1) // 17:
        raise ValueError('Invalid original stock')
    return quote


def convert(raw_manifest, raw_markets):
    for identifier, raw in [('manifest', raw_manifest), ('markets', raw_markets)]:
        if hashlib.sha256(raw).hexdigest() != SOURCES[identifier][1]:
            raise ValueError('Immutable market source hash mismatch')
    source_manifest = json.loads(raw_manifest)
    markets = json.loads(raw_markets)
    if (source_manifest['bytes'] != len(raw_markets)
            or source_manifest['sha256'] != SOURCES['markets'][1]
            or markets['version'] != 'l337-world-1'
            or len(markets['markets']) != 2048):
        raise ValueError('Original market manifest binding differs')
    output = bytearray(struct.pack('<8sH', b'MMKT0001', 2))
    selected = []
    for system in [7, 185]:
        quote = food_quote(markets, system)
        output.extend(struct.pack('<HIIQ', system, quote['askTenthsCR'], quote['bidTenthsCR'], quote['stock']))
        selected.append(quote)
    data = bytes(output)
    manifest = {
        'sourceRevision': REVISION,
        'sources': {key: {'path': value[0], 'sha256': value[1]} for key, value in SOURCES.items()},
        'selectedQuotes': selected,
        'output': 'assets/legacy/g01-food.markets',
        'outputSHA256': hashlib.sha256(data).hexdigest(), 'outputBytes': len(data),
        'format': 'MMKT0001 + LEu16 count; rows LEu16 system, LEu32 ask/bid credit tenths, LEu64 initial stock',
        'sourceDescriptiveStatuses': source_manifest['sourceDescriptiveStatuses'],
        'scope': 'Original initial seeds only; not live/current stock, market activation, physical admission or playable acceptance',
        'notice': 'assets/legacy/Elite-L337-MIT.txt',
    }
    return data, (json.dumps(manifest, indent=2) + '\n').encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data, manifest = convert(read_blob(args.repository, 'manifest'), read_blob(args.repository, 'markets'))
    for name, value in [('g01-food.markets', data), ('station-services-manifest.json', manifest)]:
        path = ROOT / 'assets/legacy' / name
        if args.check:
            if path.read_bytes() != value:
                raise ValueError('Committed service import mismatch: ' + name)
        else:
            path.write_bytes(value)
    print('Original station Food seeds ' + ('match' if args.check else 'imported'))


if __name__ == '__main__':
    main()
