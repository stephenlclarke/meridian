"""Extract exact original definitions and Avenor/Renas initial quotes from pinned Git blobs."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import struct
import subprocess
import tomllib
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REVISION = 'a11fe16924d87d06021816f9037952afcbac08db'
TREE = '68fec574df12ddf74acb14747e0dd6839f478642'
SOURCES = {
    'authoring': ('Sources/L337Core/Resources/OriginalWorlds/authoring.toml',
                  '47d83caae5a9b22a4a932f19c55f79c5cb5435b962826a0969ece096b11f80eb'),
    'manifest': ('Sources/L337Core/Resources/OriginalMarkets/manifest.json',
                 'fcc9180da8b415bb2ac0666acae197dc30f5862dac79d613fb6c6e6dfdb8e5c3'),
    'markets': ('Sources/L337Core/Resources/OriginalMarkets/markets.json',
                '4f7e7c9235f13f8151f2d53051828078f6c7d701ffd119c15031dd9137176c5d'),
    'identities': ('Sources/L337Core/GameplayContracts.swift',
                   '365ba38c3ed2dc3312eae0dfee677a3c8dec676fa0b9f925a53f1764a3e5496a'),
    'cargo': ('Sources/L337Core/CommanderCargoValidation.swift',
              '05c5c1a872560370159c4db0f1701996c65f7a1142a9f8dcfd53e731b2397f32'),
    'sellable': ('Sources/L337Core/BlackjackMission.swift',
                 'b741355ac86300a8e791dde90ca6a318ae749009d76b0042cc0e768951be88d1'),
    'display': ('Sources/L337Core/MarketDisplayOrder.swift',
                '3133b011340a50faca60df11869e9e0fb46d6157b6ffea23e5294e26503a9f54'),
    'game': ('Sources/L337Core/Game.swift',
             'e5b4e30c835d460aa7ca6f77364db0aaeaf877bb9d73bacc162e9111b26e2e17'),
    'universe': ('Sources/L337Core/Universe.swift',
                 'bc3cdb9d0c15d2f809c61de4bb3337735ebf7f087cc08c2eb079595e9cd6b849'),
    'reservations': ('Sources/L337Core/ScriptNavigationTradeState.swift',
                     '52043ac3d4208fa2b5cf8b914cd0beddc9a43bdbd0387b9a46fa4f536070f5f2'),
}
UNITS = {'t': 0, 'kg': 1, 'g': 2}
SYSTEMS = [7, 185]
COUNT = 17
MAX_QUANTITY = (2 ** 63 - 1) // COUNT
DEFINITION_FIELDS = {'index', 'name', 'unit', 'sector', 'basePriceCR', 'targetStock', 'technology', 'volatility'}
QUOTE_FIELDS = {'system', 'index', 'name', 'unit', 'askTenthsCR', 'bidTenthsCR', 'stock', 'targetStock', 'epoch', 'worldVersion'}


def authenticate(identifier: str, raw: bytes) -> None:
    if identifier not in SOURCES or hashlib.sha256(raw).hexdigest() != SOURCES[identifier][1]:
        raise ValueError('Immutable commodity source hash mismatch: ' + identifier)


def read_blob(repository: Path, identifier: str) -> bytes:
    if identifier not in SOURCES:
        raise ValueError('Unselected commodity source')
    directory = Path(repository).resolve(strict=True)
    if not directory.is_dir():
        raise ValueError('Source repository must be a directory')
    raw = subprocess.check_output(
        ['git', '--no-pager', 'show', '--no-ext-diff', '--no-textconv', REVISION + ':' + SOURCES[identifier][0]],
        cwd=directory,
    )
    authenticate(identifier, raw)
    return raw


def bounded_integer(value: Any, maximum: int, label: str) -> int:
    if type(value) is not int or not 0 <= value <= maximum:
        raise ValueError('Invalid ' + label)
    return value


def text_bytes(value: Any, label: str) -> bytes:
    if not isinstance(value, str) or not value or len(value.encode()) > 255:
        raise ValueError('Invalid ' + label)
    return value.encode()


def definitions(authoring: dict[str, Any]) -> tuple[list[dict[str, Any]], list[int]]:
    rows = authoring['commodities']
    if authoring['version'] != 'l337-world-1' or len(rows) != COUNT:
        raise ValueError('Original commodity definition count/version differs')
    order, by_id = [], {}
    for row in rows:
        if set(row) != DEFINITION_FIELDS:
            raise ValueError('Original commodity definition fields differ')
        identifier = bounded_integer(row['index'], COUNT - 1, 'commodity ID')
        if identifier in by_id:
            raise ValueError('Duplicate commodity ID')
        text_bytes(row['name'], 'commodity name')
        text_bytes(row['sector'], 'commodity sector')
        if row['unit'] not in UNITS:
            raise ValueError('Invalid native commodity unit')
        for field in ['basePriceCR', 'volatility']:
            if type(row[field]) not in (int, float) or not math.isfinite(row[field]) or row[field] < 0:
                raise ValueError('Invalid definition ' + field)
        bounded_integer(row['targetStock'], MAX_QUANTITY, 'definition stock')
        bounded_integer(row['technology'], 65535, 'commodity technology')
        by_id[identifier] = row
        order.append(identifier)
    return [by_id[index] for index in range(COUNT)], order


def source_semantics(raw: dict[str, bytes]) -> tuple[list[str], list[int]]:
    identities = raw['identities'].decode()
    enum = re.search(r'public enum CommodityIdentity:[^{]+\{\s*case (.*?)\n  public var unit:', identities, re.S)
    if enum is None:
        raise ValueError('Missing native commodity identity source')
    names = [name.strip() for name in enum.group(1).replace('\n', '').split(',')]
    if len(names) != COUNT or len(set(names)) != COUNT or not all(re.fullmatch('[A-Za-z]+', name) for name in names):
        raise ValueError('Native commodity identity source differs')
    expression = re.search(
        r'commander\.legal \|= commander\.cargo\[(\d+)\] \+ (\d+) \* commander\.cargo\[(\d+)\] \+ commander\.cargo\[(\d+)\]',
        raw['game'].decode(),
    )
    if expression is None:
        raise ValueError('Missing native departure legality source')
    first, multiplier, second, third = map(int, expression.groups())
    weights = [0] * COUNT
    for identifier, weight in [(first, 1), (second, multiplier), (third, 1)]:
        bounded_integer(identifier, COUNT - 1, 'departure commodity ID')
        bounded_integer(weight, 255, 'departure legality weight')
        if weights[identifier] or weight == 0:
            raise ValueError('Duplicate/empty departure legality term')
        weights[identifier] = weight
    # These are explicit source witnesses, not configurable runtime permission switches.
    witnesses = {
        'identities': 'public var tonnes: Int { commodity.unit == "t" ? quantity : 0 }',
        'cargo': 'for (amount, commodity) in zip(cargo, Commodity.all) where commodity.unit == "t"',
        'sellable': 'cargo[index] - (index == 10 ? markedFirearms : 0) - (index == 3 ? nativeCaptiveCount : 0)',
        'display': 'Universe.originalWorlds.authoring.commodities.map(\\.index)',
        'reservations': 'func reservedNavigationTradeCargo(_ commodity: Int) -> Int',
    }
    for key, text in witnesses.items():
        if text not in raw[key].decode():
            raise ValueError('Missing original semantics witness: ' + key)
    if '- reservedNavigationTradeCargo(index)' not in raw['sellable'].decode():
        raise ValueError('Missing native reserved cargo source')
    return names, weights


def catalogue_bytes(rows: list[dict[str, Any]], order: list[int], weights: list[int]) -> bytes:
    output = bytearray(struct.pack('<8sH', b'MCOM0001', COUNT))
    for row, weight in zip(rows, weights, strict=True):
        name, sector = text_bytes(row['name'], 'commodity name'), text_bytes(row['sector'], 'commodity sector')
        output.extend(struct.pack('<4B', row['index'], UNITS[row['unit']], weight, len(name)))
        output.extend(name)
        output.extend(struct.pack('<B', len(sector)))
        output.extend(sector)
        output.extend(struct.pack('<dQHd', row['basePriceCR'], row['targetStock'], row['technology'], row['volatility']))
    output.extend(bytes(order))
    return bytes(output)


def validated_quote(quote: dict[str, Any], definition: dict[str, Any], system: int) -> dict[str, Any]:
    if set(quote) != QUOTE_FIELDS:
        raise ValueError('Original quote fields differ')
    if (quote['name'], quote['unit'], quote['system'], quote['worldVersion']) != (
            definition['name'], definition['unit'], system, 'l337-world-1'):
        raise ValueError('Quote/commodity identity or native unit differs')
    for field in ['askTenthsCR', 'bidTenthsCR']:
        bounded_integer(quote[field], 2 ** 32 - 1, 'quote money')
    for field in ['stock', 'targetStock']:
        bounded_integer(quote[field], MAX_QUANTITY, 'quote quantity')
    bounded_integer(quote['epoch'], 2 ** 64 - 1, 'quote epoch')
    if quote['bidTenthsCR'] > quote['askTenthsCR']:
        raise ValueError('Crossed original quote')
    return quote


def selected_quotes(document: dict[str, Any], rows: list[dict[str, Any]], system: int) -> list[dict[str, Any]]:
    states = [state for state in document['markets'] if state['galaxy'] == 0 and state['system'] == system]
    if len(states) != 1 or states[0]['key'] != f'G01-S{system:03}' or len(states[0]['quotes']) != COUNT:
        raise ValueError('Selected original market identity/count differs')
    if type(states[0]['galaxy']) is not int or type(states[0]['system']) is not int:
        raise ValueError('Invalid original market ID type')
    by_id = {}
    for quote in states[0]['quotes']:
        identifier = bounded_integer(quote['index'], COUNT - 1, 'quote ID')
        if identifier in by_id:
            raise ValueError('Duplicate quote ID')
        by_id[identifier] = validated_quote(quote, rows[identifier], system)
    return [by_id[index] for index in range(COUNT)]


def market_bytes(document: dict[str, Any], rows: list[dict[str, Any]]) -> tuple[bytes, list[dict[str, Any]]]:
    if document['version'] != 'l337-world-1' or len(document['markets']) != 2048:
        raise ValueError('Original market count/version differs')
    output = bytearray(struct.pack('<8sHH', b'MMKT0002', len(SYSTEMS), COUNT))
    selected = []
    for system in SYSTEMS:
        quotes = selected_quotes(document, rows, system)
        output.extend(struct.pack('<H', system))
        for quote in quotes:
            output.extend(struct.pack('<BIIQQQ', quote['index'], quote['askTenthsCR'], quote['bidTenthsCR'],
                                      quote['stock'], quote['targetStock'], quote['epoch']))
        selected.append({'galaxy': 0, 'system': system, 'quotes': quotes})
    return bytes(output), selected


def convert(raw: dict[str, bytes]) -> dict[str, bytes]:
    if set(raw) != set(SOURCES):
        raise ValueError('Incomplete commodity source input set')
    for key, value in raw.items():
        authenticate(key, value)
    original_manifest = json.loads(raw['manifest'])
    if (original_manifest['bytes'] != len(raw['markets'])
            or original_manifest['sha256'] != SOURCES['markets'][1]):
        raise ValueError('Original market manifest binding differs')
    authoring = tomllib.loads(raw['authoring'].decode())
    rows, order = definitions(authoring)
    native_names, weights = source_semantics(raw)
    catalogue = catalogue_bytes(rows, order, weights)
    markets, selected = market_bytes(json.loads(raw['markets']), rows)
    outputs = {'commodities.catalogue': catalogue, 'g01-commodities.markets': markets}
    manifest = {
        'sourceRevision': REVISION, 'sourceTree': TREE,
        'sources': {key: {'path': pin[0], 'sha256': pin[1], 'bytes': len(raw[key])} for key, pin in SOURCES.items()},
        'definitions': rows, 'nativeIdentityCases': native_names, 'displayOrder': order,
        'nativeUnits': UNITS, 'departureLegalWeights': weights, 'markets': selected,
        'outputs': {name: {'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)} for name, data in outputs.items()},
        'catalogueFormat': 'MCOM0001 + LEu16 count17; rows u8 ID/unitTag/departureLegalWeight/nameBytes, UTF8name, u8 sectorBytes, UTF8sector, LEf64 basePriceCR, LEu64 targetStock, LEu16 technology, LEf64 volatility; final17u8 display-order IDs',
        'marketFormat': 'MMKT0002 + LEu16 marketCount2/commodityCount17; each LEu16 system then17rows u8 ID, LEu32 ask/bid credit tenths, LEu64 stock/targetStock/epoch',
        'semantics': {
            'ordinaryCommodityTonnes': 'quantity for unit t; zero for kg/g, not fractional SI hold conversion',
            'quantityBound': MAX_QUANTITY,
            'sellableCargo': 'max(0,cargo[index]-markedFirearms(if10)-nativeCaptiveCount(if3)-reservedNavigationTradeCargo(index))',
            'departureLegality': 'legal |= cargo[3] + 2*cargo[6] + cargo[10]; no automatic purchase/sale fine',
            'transaction': 'one native unit per action; actual ask/bid, stock, funds, hold and sellable custody; sale caps credits at4294967295',
            'authority': 'definitions/quotes do not grant stock refill, cargo, custody, people consent, private approval, paid fitting or payment',
        },
        'originalAuthoring': {
            'auditReceipt': 'assets/legacy/original-economy-origin.json',
            'auditReceiptSHA256': 'a3a829192e7147ce5840924b1d0d8dbee419d8411f41057937ac26bdd1e557a8',
            'worldEconomyGenerator': 'original-world-economy/generate.py',
            'generatorSHA256': '05f54ff56d624f3090252a70306f145f092d22c881cd4d6faea227cb945250ed',
            'runtimeMarketCompiler': 'Tools/prepare-original-markets.py',
            'compilerSHA256': '2e4e450ec53cc85c6a082dd2bcd63149597d3c347d1f72b0cc0486dac3ea23a3',
            'adoptedOriginalCommit': '8a3021b75fabf01b4743538aba0ffe7ca0bfb161',
            'scope': 'Original L337-authored budgets/supply-demand; independent original-output reproduction recorded by MC1; no Elite/Oolite coordinate/price tables as generator inputs',
        },
        'sourceDescriptiveStatuses': original_manifest['sourceDescriptiveStatuses'],
        'notice': 'assets/legacy/Elite-L337-MIT.txt',
        'scope': 'Exact17definitions and34initialquotes only; live stock/transactions/save/legality application remain World-owned; not playable or commercial clearance',
    }
    outputs['commodity-content-manifest.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    return outputs


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    outputs = convert({key: read_blob(args.repository, key) for key in SOURCES})
    for name, value in outputs.items():
        path = ROOT / 'assets/legacy' / name
        if args.check:
            if path.read_bytes() != value:
                raise ValueError('Committed commodity import mismatch: ' + name)
        else:
            path.write_bytes(value)
    print('Original17commodity definitions and34quotes ' + ('match' if args.check else 'imported'))


if __name__ == '__main__':
    main()
