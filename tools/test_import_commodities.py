"""Original commodity extraction/schema tests; no core trading or native runtime execution."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('import_commodities', ROOT / 'tools/import_commodities.py')
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)
EXPECTED_NAMES = ['Food', 'Textiles', 'Radioactives', 'Slaves', 'Liquor', 'Luxuries',
                  'Narcotics', 'Computers', 'Machinery', 'Alloys', 'Weapons', 'Furs',
                  'Minerals', 'Gold', 'Platinum', 'Gem-Stones', 'Alien Items']
EXPECTED_ORDER = [9, 8, 10, 2, 7, 12, 13, 15, 14, 1, 4, 5, 0, 11, 6, 3, 16]
# Independently read from the pinned market JSON, not calculated from base prices.
EXPECTED_QUOTES = {
    7: [(226, 208, 128, 126), (347, 319, 128, 157), (5142, 4746, 12, 14),
        (3811, 3517, 4, 4), (708, 652, 72, 88), (9145, 8441, 14, 16),
        (5661, 5225, 9, 10), (3613, 3335, 23, 23), (1315, 1213, 54, 56),
        (747, 689, 92, 114), (4414, 4074, 18, 20), (1193, 1101, 35, 37),
        (241, 221, 217, 218), (4345, 4009, 47, 58), (7605, 7019, 21, 28),
        (894, 824, 106, 140), (41338, 38158, 0, 0)],
    185: [(216, 196, 112, 143), (346, 316, 163, 150), (5418, 4950, 15, 16),
          (3866, 3532, 5, 6), (787, 719, 64, 85), (7879, 7199, 12, 16),
          (5072, 4634, 12, 11), (3438, 3140, 26, 27), (1375, 1255, 58, 65),
          (593, 541, 135, 130), (4269, 3901, 19, 23), (1102, 1006, 38, 41),
          (425, 387, 47, 60), (6192, 5658, 14, 16), (9032, 8254, 8, 9),
          (1455, 1329, 32, 40), (37041, 33849, 0, 0)],
    91: [(202, 184, 138, 148), (356, 326, 120, 124), (4582, 4208, 13, 13), (3047, 2797, 5, 5), (858, 788, 51, 70), (7008, 6436, 13, 14), (4493, 4125, 10, 12), (3288, 3018, 24, 23), (1294, 1188, 41, 51), (662, 608, 79, 103), (3094, 2840, 20, 18), (1024, 940, 42, 43), (359, 329, 99, 99), (4972, 4566, 22, 27), (8568, 7868, 13, 14), (1062, 974, 72, 65), (40226, 36946, 0, 0)],
}


def metadata():
    return json.loads((ROOT / 'assets/legacy/commodity-content-manifest.json').read_text())


def schema_inputs():
    # Compact parsed-data fixture only. Exact immutable source authentication is tested
    # separately and the owner runs --check against actual a11 Git blobs.
    manifest = metadata()
    ordered = [manifest['definitions'][identifier] for identifier in manifest['displayOrder']]
    authoring = {'version': 'l337-world-1', 'commodities': copy.deepcopy(ordered)}
    states = [{'key': f"G01-S{state['system']:03}", **copy.deepcopy(state)} for state in manifest['markets']]
    states.extend({'galaxy': 7, 'system': 255} for _ in range(2045))
    return authoring, {'version': 'l337-world-1', 'markets': states}


def schema_blobs():
    authoring, document = schema_inputs()
    lines = ['version = "l337-world-1"']
    for row in authoring['commodities']:
        lines.append('[[commodities]]')
        lines.extend(key + ' = ' + json.dumps(value) for key, value in row.items())
    markets = json.dumps(document).encode()
    cases = ', '.join(metadata()['nativeIdentityCases'])
    raw = {
        'authoring': ('\n'.join(lines) + '\n').encode(),
        'markets': markets,
        'manifest': json.dumps({'bytes': len(markets), 'sha256': hashlib.sha256(markets).hexdigest(),
                                'sourceDescriptiveStatuses': metadata()['sourceDescriptiveStatuses']}).encode(),
        'identities': ('public enum CommodityIdentity: Int {\n  case ' + cases
                       + '\n  public var unit: String\n'
                       + 'public var tonnes: Int { commodity.unit == "t" ? quantity : 0 }').encode(),
        'cargo': b'for (amount, commodity) in zip(cargo, Commodity.all) where commodity.unit == "t"',
        'sellable': b'cargo[index] - (index == 10 ? markedFirearms : 0) - (index == 3 ? nativeCaptiveCount : 0) - reservedNavigationTradeCargo(index)',
        'display': b'Universe.originalWorlds.authoring.commodities.map(\\.index)',
        'game': b'commander.legal |= commander.cargo[3] + 2 * commander.cargo[6] + commander.cargo[10]',
        'universe': b'isolated parsed schema fixture, not source provenance',
        'reservations': b'func reservedNavigationTradeCargo(_ commodity: Int) -> Int',
    }
    pins = {key: (IMPORTER.SOURCES[key][0], hashlib.sha256(value).hexdigest()) for key, value in raw.items()}
    return raw, pins


class CommodityImportTests(unittest.TestCase):
    def test_catalogue_independent_decoder_preserves_all_definition_fields_units_ids_and_order(self):
        manifest = metadata()
        raw = (ROOT / 'assets/legacy/commodities.catalogue').read_bytes()
        self.assertEqual(struct.unpack_from('<8sH', raw), (b'MCOM0001', 17))
        offset = 10
        for identifier in range(17):
            actual_id, unit, legal, length = struct.unpack_from('<4B', raw, offset)
            offset += 4
            name = raw[offset:offset + length].decode()
            offset += length
            length = raw[offset]
            offset += 1
            sector = raw[offset:offset + length].decode()
            offset += length
            base, target, technology, volatility = struct.unpack_from('<dQHd', raw, offset)
            offset += 26
            source = manifest['definitions'][identifier]
            expected_unit = 1 if identifier in (13, 14) else 2 if identifier == 15 else 0
            self.assertEqual((actual_id, name, unit), (identifier, EXPECTED_NAMES[identifier], expected_unit))
            self.assertEqual(legal, {3: 1, 6: 2, 10: 1}.get(identifier, 0))
            self.assertEqual((sector, base, target, technology, volatility),
                             tuple(source[key] for key in ['sector', 'basePriceCR', 'targetStock', 'technology', 'volatility']))
        self.assertEqual(list(raw[offset:]), EXPECTED_ORDER)
        self.assertEqual(offset + 17, len(raw))
        self.assertEqual(manifest['nativeIdentityCases'][10], 'firearms')
        self.assertEqual(manifest['nativeIdentityCases'][15], 'gemstones')

    def test_all51_actual_quotes_stock_target_and_epoch_are_exact_by_native_id(self):
        raw = (ROOT / 'assets/legacy/g01-commodities.markets').read_bytes()
        self.assertEqual(len(raw), 1701)
        self.assertEqual(struct.unpack_from('<8sHH', raw), (b'MMKT0002', 3, 17))
        offset = 12
        for system in [7, 185, 91]:
            self.assertEqual(struct.unpack_from('<H', raw, offset)[0], system)
            offset += 2
            for identifier, expected in enumerate(EXPECTED_QUOTES[system]):
                row = struct.unpack_from('<BIIQQQ', raw, offset)
                offset += 33
                self.assertEqual(row, (identifier, *expected, 0))
        self.assertEqual(offset, len(raw))
        self.assertEqual(metadata()['definitions'][16]['targetStock'], 4)
        self.assertEqual(EXPECTED_QUOTES[7][16][2:], (0, 0))

    def test_existing_food_bytes_notices_and_source_output_hashes_are_preserved(self):
        manifest = metadata()
        self.assertEqual(manifest['sourceRevision'], IMPORTER.REVISION)
        self.assertEqual(manifest['sourceTree'], IMPORTER.TREE)
        for key, pin in IMPORTER.SOURCES.items():
            self.assertEqual((manifest['sources'][key]['path'], manifest['sources'][key]['sha256']), pin)
        for name, expected in manifest['outputs'].items():
            raw = (ROOT / 'assets/legacy' / name).read_bytes()
            self.assertEqual((len(raw), hashlib.sha256(raw).hexdigest()), (expected['bytes'], expected['sha256']))
        food = (ROOT / 'assets/legacy/g01-food.markets').read_bytes()
        self.assertEqual(struct.unpack_from('<8sH', food), (b'MMKT0001', 3))
        self.assertEqual(struct.unpack_from('<HIIQ', food, 46), (91, 202, 184, 138))
        self.assertEqual(struct.unpack_from('<HIIQ', food, 10), (7, 226, 208, 128))
        self.assertEqual(struct.unpack_from('<HIIQ', food, 28), (185, 216, 196, 112))
        self.assertTrue((ROOT / manifest['notice']).is_file())
        origin = manifest['originalAuthoring']
        audit = (ROOT / origin['auditReceipt']).read_bytes()
        self.assertEqual(hashlib.sha256(audit).hexdigest(), origin['auditReceiptSHA256'])
        self.assertEqual(json.loads(audit)['sourceRevision'], IMPORTER.REVISION)

    def test_hash_input_set_and_unselected_blob_fail_before_conversion_or_git(self):
        with self.assertRaisesRegex(ValueError, 'Incomplete'):
            IMPORTER.convert({})
        with self.assertRaisesRegex(ValueError, 'hash mismatch'):
            IMPORTER.convert({key: b'foreign' for key in IMPORTER.SOURCES})
        for identifier in ['--output=unexpected', '../authoring', 'markets;echo injected']:
            with patch.object(IMPORTER.subprocess, 'check_output') as call:
                with self.assertRaisesRegex(ValueError, 'Unselected'):
                    IMPORTER.read_blob(ROOT, identifier)
                call.assert_not_called()

    def test_git_reads_fixed_pinned_blob_in_validated_cwd_without_external_helpers(self):
        raw = b'isolated source test'
        with tempfile.TemporaryDirectory(prefix='--source-objects-') as directory:
            repository = Path(directory).resolve()
            for identifier, (path, _) in IMPORTER.SOURCES.items():
                with patch.dict(IMPORTER.SOURCES, {identifier: (path, hashlib.sha256(raw).hexdigest())}):
                    with patch.object(IMPORTER.subprocess, 'check_output', return_value=raw) as call:
                        self.assertEqual(IMPORTER.read_blob(repository, identifier), raw)
                        call.assert_called_once_with(
                            ['git', '--no-pager', 'show', '--no-ext-diff', '--no-textconv', IMPORTER.REVISION + ':' + path],
                            cwd=repository,
                        )
        with patch.object(IMPORTER.subprocess, 'check_output', return_value=b'foreign') as call:
            with self.assertRaisesRegex(ValueError, 'directory'):
                IMPORTER.read_blob(Path(__file__), 'markets')
            call.assert_not_called()
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                IMPORTER.read_blob(ROOT, 'markets')

    def test_parsed_extraction_uses_ids_for_quotes_and_retains_authoring_display_order(self):
        authoring, document = schema_inputs()
        rows, order = IMPORTER.definitions(authoring)
        self.assertEqual(order, EXPECTED_ORDER)
        self.assertEqual([r['index'] for r in rows], list(range(17)))
        for state in document['markets'][:2]:
            state['quotes'].reverse()
        actual, _ = IMPORTER.market_bytes(document, rows)
        self.assertEqual(actual, (ROOT / 'assets/legacy/g01-commodities.markets').read_bytes())
        catalogue = IMPORTER.catalogue_bytes(rows, order, metadata()['departureLegalWeights'])
        self.assertEqual(catalogue, (ROOT / 'assets/legacy/commodities.catalogue').read_bytes())

    def test_definition_schema_rejects_duplicate_ids_units_nonfinite_values_and_extra_fields(self):
        for field, value in [('index', True), ('index', 99), ('name', ''), ('unit', 'tonne'),
                             ('basePriceCR', float('nan')), ('volatility', -1), ('targetStock', True)]:
            authoring, _ = schema_inputs()
            authoring['commodities'][0][field] = value
            with self.subTest(field=field, value=value):
                with self.assertRaises(ValueError):
                    IMPORTER.definitions(authoring)
        authoring, _ = schema_inputs()
        authoring['commodities'][1]['index'] = authoring['commodities'][0]['index']
        with self.assertRaisesRegex(ValueError, 'Duplicate'):
            IMPORTER.definitions(authoring)
        authoring, _ = schema_inputs()
        authoring['commodities'][0]['permission'] = True
        with self.assertRaisesRegex(ValueError, 'fields'):
            IMPORTER.definitions(authoring)

    def test_quotes_reject_duplicate_identity_wrong_units_crossed_price_and_quantity_overflow(self):
        for field, value in [('index', True), ('unit', 'kg'), ('name', 'invented'),
                             ('askTenthsCR', 0), ('stock', -1), ('stock', True),
                             ('stock', IMPORTER.MAX_QUANTITY + 1), ('epoch', 2 ** 64)]:
            authoring, document = schema_inputs()
            rows, _ = IMPORTER.definitions(authoring)
            document['markets'][0]['quotes'][0][field] = value
            with self.subTest(field=field, value=value):
                with self.assertRaises(ValueError):
                    IMPORTER.market_bytes(document, rows)
        authoring, document = schema_inputs()
        rows, _ = IMPORTER.definitions(authoring)
        document['markets'][0]['quotes'][1] = copy.deepcopy(document['markets'][0]['quotes'][0])
        with self.assertRaisesRegex(ValueError, 'Duplicate'):
            IMPORTER.market_bytes(document, rows)

    def test_missing_or_foreign_selected_market_never_substitutes_a_default(self):
        for key, value in [('key', 'G02-S007'), ('system', 8), ('galaxy', False)]:
            authoring, document = schema_inputs()
            rows, _ = IMPORTER.definitions(authoring)
            document['markets'][0][key] = value
            with self.subTest(key=key):
                with self.assertRaises(ValueError):
                    IMPORTER.market_bytes(document, rows)

    def test_complete_schema_conversion_keeps_binary_contract_and_refuses_broken_manifest_binding(self):
        raw, pins = schema_blobs()
        with patch.dict(IMPORTER.SOURCES, pins):
            outputs = IMPORTER.convert(raw)
        for name in ['commodities.catalogue', 'g01-commodities.markets']:
            self.assertEqual(outputs[name], (ROOT / 'assets/legacy' / name).read_bytes())
        converted = json.loads(outputs['commodity-content-manifest.json'])
        self.assertEqual(converted['departureLegalWeights'], [0, 0, 0, 1, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0])
        self.assertEqual(converted['displayOrder'], EXPECTED_ORDER)
        broken = copy.deepcopy(raw)
        manifest = json.loads(broken['manifest'])
        manifest['bytes'] += 1
        broken['manifest'] = json.dumps(manifest).encode()
        pins['manifest'] = (pins['manifest'][0], hashlib.sha256(broken['manifest']).hexdigest())
        with patch.dict(IMPORTER.SOURCES, pins):
            with self.assertRaisesRegex(ValueError, 'manifest binding'):
                IMPORTER.convert(broken)

    def test_semantic_source_witnesses_never_fall_back_to_assumed_units_or_legality(self):
        raw, _ = schema_blobs()
        for key in ['identities', 'game', 'cargo', 'sellable', 'display', 'reservations']:
            broken = dict(raw)
            broken[key] = b'unsupported semantics'
            with self.subTest(key=key):
                with self.assertRaises(ValueError):
                    IMPORTER.source_semantics(broken)

    def test_cli_writes_all_outputs_and_check_refuses_tampering_without_overwrite(self):
        raw, pins = schema_blobs()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / 'assets/legacy'
            output.mkdir(parents=True)
            with patch.object(IMPORTER, 'ROOT', root), patch.dict(IMPORTER.SOURCES, pins):
                with patch.object(IMPORTER, 'read_blob', side_effect=lambda _repository, identifier: raw[identifier]):
                    with patch.object(sys, 'argv', ['import_commodities', '.']):
                        IMPORTER.main()
                    with patch.object(sys, 'argv', ['import_commodities', '.', '--check']):
                        IMPORTER.main()
                    target = output / 'commodities.catalogue'
                    target.write_bytes(b'tampered')
                    before = {path.name: path.read_bytes() for path in output.iterdir()}
                    with patch.object(sys, 'argv', ['import_commodities', '.', '--check']):
                        with self.assertRaisesRegex(ValueError, 'mismatch'):
                            IMPORTER.main()
                    self.assertEqual({path.name: path.read_bytes() for path in output.iterdir()}, before)


if __name__ == '__main__':
    unittest.main()
