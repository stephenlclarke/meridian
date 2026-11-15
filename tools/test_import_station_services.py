"""Original service seed provenance and safe extraction; no Rust or actual World execution."""
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


class ServiceImportTests(unittest.TestCase):
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
