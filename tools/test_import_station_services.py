"""Original service seed provenance and safe extraction; no Rust or actual World execution."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('import_station_services', ROOT / 'tools/import_station_services.py')
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


def schema_markets():
    # Synthetic parsed-schema inputs, not original provenance or playable content.
    states = []
    for system, ask, bid, stock in [(7, 226, 208, 128), (185, 216, 196, 112)]:
        food = {'index': 0, 'name': 'Food', 'unit': 't', 'epoch': 0,
                'system': system, 'worldVersion': 'l337-world-1',
                'askTenthsCR': ask, 'bidTenthsCR': bid, 'stock': stock}
        states.append({'galaxy': 0, 'system': system,
                       'quotes': [food] + [{'index': index} for index in range(1, 17)]})
    states.extend({'galaxy': 1, 'system': index, 'quotes': []} for index in range(2046))
    return {'version': 'l337-world-1', 'markets': states}


def convert_schema(markets, manifest_overrides=None):
    raw_markets = json.dumps(markets).encode()
    manifest = {'bytes': len(raw_markets),
                'sha256': hashlib.sha256(raw_markets).hexdigest(),
                'sourceDescriptiveStatuses': ['Synthetic schema fixture; not provenance']}
    manifest.update(manifest_overrides or {})
    raw_manifest = json.dumps(manifest).encode()
    pins = {key: (IMPORTER.SOURCES[key][0], hashlib.sha256(raw).hexdigest())
            for key, raw in [('markets', raw_markets), ('manifest', raw_manifest)]}
    with patch.dict(IMPORTER.SOURCES, pins):
        return IMPORTER.convert(raw_manifest, raw_markets)


class ServiceImportTests(unittest.TestCase):
    def test_schema_conversion_encodes_both_sites_without_changing_inputs(self):
        markets = schema_markets()
        original = copy.deepcopy(markets)
        raw, metadata = convert_schema(markets)
        self.assertEqual(raw, struct.pack('<8sHHIIQHIIQ', b'MMKT0001', 2,
                                         7, 226, 208, 128, 185, 216, 196, 112))
        decoded = json.loads(metadata)
        self.assertEqual(decoded['outputSHA256'], hashlib.sha256(raw).hexdigest())
        self.assertEqual([quote['system'] for quote in decoded['selectedQuotes']], [7, 185])
        self.assertEqual(markets, original)

    def test_schema_conversion_refuses_manifest_and_world_binding_changes(self):
        for override in [{'bytes': 0}, {'sha256': 'foreign'}]:
            with self.subTest(manifest=override):
                with self.assertRaisesRegex(ValueError, 'manifest binding'):
                    convert_schema(schema_markets(), override)
        for change in ['version', 'count']:
            markets = schema_markets()
            if change == 'version':
                markets['version'] = 'foreign-world'
            else:
                markets['markets'].pop()
            with self.subTest(world=change):
                with self.assertRaisesRegex(ValueError, 'manifest binding'):
                    convert_schema(markets)

    def test_schema_conversion_refuses_missing_duplicate_or_wrong_sized_markets(self):
        for change in ['missing', 'duplicate', 'short', 'long']:
            markets = schema_markets()
            if change == 'missing':
                markets['markets'][0]['galaxy'] = 1
            elif change == 'duplicate':
                markets['markets'][-1] = copy.deepcopy(markets['markets'][0])
            elif change == 'short':
                markets['markets'][0]['quotes'].pop()
            else:
                markets['markets'][0]['quotes'].append({'index': 17})
            original = copy.deepcopy(markets)
            with self.subTest(change=change):
                with self.assertRaisesRegex(ValueError, 'station market'):
                    convert_schema(markets)
                self.assertEqual(markets, original)

    def test_schema_conversion_refuses_missing_or_duplicate_food(self):
        for index, replacement in [(0, 17), (1, 0)]:
            markets = schema_markets()
            markets['markets'][0]['quotes'][index]['index'] = replacement
            with self.subTest(index=index):
                with self.assertRaisesRegex(ValueError, 'Food quote'):
                    convert_schema(markets)

    def test_schema_conversion_refuses_changed_food_identity(self):
        for field, value in [('name', 'foreign'), ('unit', 'kg'), ('epoch', 1),
                             ('system', 185), ('worldVersion', 'foreign-world')]:
            markets = schema_markets()
            markets['markets'][0]['quotes'][0][field] = value
            with self.subTest(field=field):
                with self.assertRaisesRegex(ValueError, 'Food identity'):
                    convert_schema(markets)

    def test_schema_conversion_checks_price_and_stock_boundaries(self):
        maximum_stock = (2 ** 63 - 1) // 17
        invalid = [('bidTenthsCR', -1, 'price'), ('bidTenthsCR', 227, 'price'),
                   ('askTenthsCR', 1000001, 'price'), ('stock', -1, 'stock'),
                   ('stock', maximum_stock + 1, 'stock')]
        for field, value, message in invalid:
            markets = schema_markets()
            markets['markets'][1]['quotes'][0][field] = value
            with self.subTest(field=field, value=value):
                with self.assertRaisesRegex(ValueError, message):
                    convert_schema(markets)
        markets = schema_markets()
        markets['markets'][0]['quotes'][0].update(
            bidTenthsCR=1000000, askTenthsCR=1000000, stock=maximum_stock)
        raw, _ = convert_schema(markets)
        self.assertEqual(struct.unpack_from('<HIIQ', raw, 10),
                         (7, 1000000, 1000000, maximum_stock))

    def test_unselected_bytes_and_blob_selectors_refuse(self):
        with self.assertRaisesRegex(ValueError, 'hash mismatch'):
            IMPORTER.convert(b'foreign', b'foreign')
        for identifier in ['--output=unexpected', '../markets', 'markets;echo injected']:
            with patch.object(IMPORTER.subprocess, 'check_output') as call:
                with self.assertRaisesRegex(ValueError, 'Unselected'):
                    IMPORTER.read_blob(ROOT, identifier)
                call.assert_not_called()

    def test_exact_git_source_command_uses_cwd_and_disables_external_helpers(self):
        raw = b'isolated test bytes'
        path, _ = IMPORTER.SOURCES['markets']
        with tempfile.TemporaryDirectory(prefix='--source-objects-') as directory:
            repository = Path(directory).resolve()
            with patch.dict(IMPORTER.SOURCES, {'markets': (path, hashlib.sha256(raw).hexdigest())}):
                with patch.object(IMPORTER.subprocess, 'check_output', return_value=raw) as call:
                    self.assertEqual(IMPORTER.read_blob(repository, 'markets'), raw)
                    call.assert_called_once_with(
                        ['git', '--no-pager', 'show', '--no-ext-diff', '--no-textconv',
                         IMPORTER.REVISION + ':' + path], cwd=repository,
                    )

    def test_invalid_directory_and_wrong_hash_refuse(self):
        with patch.object(IMPORTER.subprocess, 'check_output', return_value=b'foreign') as call:
            with self.assertRaisesRegex(ValueError, 'directory'):
                IMPORTER.read_blob(Path(__file__), 'markets')
            call.assert_not_called()
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                IMPORTER.read_blob(ROOT, 'markets')

    def test_packaged_food_quotes_retain_source_prices_initial_stock_and_notice(self):
        manifest = json.loads((ROOT / 'assets/legacy/station-services-manifest.json').read_text())
        raw = (ROOT / manifest['output']).read_bytes()
        self.assertEqual(manifest['sourceRevision'], IMPORTER.REVISION)
        self.assertEqual(manifest['outputSHA256'], hashlib.sha256(raw).hexdigest())
        self.assertEqual(manifest['outputBytes'], len(raw))
        self.assertEqual(len(raw), 46)
        self.assertEqual(struct.unpack_from('<8sH', raw), (b'MMKT0001', 2))
        self.assertEqual(struct.unpack_from('<HIIQ', raw, 10), (7, 226, 208, 128))
        self.assertEqual(struct.unpack_from('<HIIQ', raw, 28), (185, 216, 196, 112))
        self.assertEqual([q['index'] for q in manifest['selectedQuotes']], [0, 0])
        self.assertEqual([q['unit'] for q in manifest['selectedQuotes']], ['t', 't'])
        self.assertTrue((ROOT / manifest['notice']).is_file())
        self.assertTrue(all('not installed' in value.lower()
                            for value in manifest['sourceDescriptiveStatuses'].values()))


if __name__ == '__main__':
    unittest.main()
