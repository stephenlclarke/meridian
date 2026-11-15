"""Navigation import identity and safe immutable Git extraction; no Rust execution."""
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('import_navigation', ROOT / 'tools/import_navigation.py')
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


class NavigationImportTests(unittest.TestCase):
    def test_unselected_bytes_refuse_before_conversion(self):
        with self.assertRaisesRegex(ValueError, 'hash mismatch'):
            IMPORTER.convert(b'foreign catalogue')

    def test_fixed_blob_extraction_uses_directory_as_cwd_without_diff_helpers(self):
        raw = b'isolated test input'
        with tempfile.TemporaryDirectory(prefix='--reference-objects-') as directory:
            repository = Path(directory).resolve()
            with patch.object(IMPORTER, 'SOURCE_SHA', hashlib.sha256(raw).hexdigest()):
                with patch.object(IMPORTER.subprocess, 'check_output', return_value=raw) as call:
                    self.assertEqual(IMPORTER.read_blob(repository), raw)
                    call.assert_called_once_with(
                        ['git', '--no-pager', 'show', '--no-ext-diff', '--no-textconv',
                         IMPORTER.REVISION + ':' + IMPORTER.SOURCE], cwd=repository,
                    )

    def test_invalid_directory_and_wrong_source_hash_refuse(self):
        with patch.object(IMPORTER.subprocess, 'check_output', return_value=b'foreign') as call:
            with self.assertRaisesRegex(ValueError, 'directory'):
                IMPORTER.read_blob(Path(__file__))
            call.assert_not_called()
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                IMPORTER.read_blob(ROOT)

    def test_packaged_g01_bytes_hash_names_coordinates_and_notice_match(self):
        manifest = json.loads((ROOT / 'assets/legacy/navigation-manifest.json').read_text())
        raw = (ROOT / manifest['output']).read_bytes()
        self.assertEqual(manifest['sourceRevision'], IMPORTER.REVISION)
        self.assertEqual(manifest['sourceSHA256'], IMPORTER.SOURCE_SHA)
        self.assertEqual(manifest['outputSHA256'], hashlib.sha256(raw).hexdigest())
        self.assertEqual(manifest['outputBytes'], len(raw))
        self.assertTrue((ROOT / manifest['notice']).is_file())
        self.assertEqual(struct.unpack_from('<8sH', raw), (b'MNAV0001', 256))
        offset = 10
        rows = []
        for expected in range(256):
            identifier, x, y, length = struct.unpack_from('<HddB', raw, offset)
            offset += 19
            name = raw[offset:offset + length].decode()
            offset += length
            self.assertEqual(identifier, expected)
            rows.append((name, x, y))
        self.assertEqual(offset, len(raw))
        self.assertEqual(rows[7], ('Avenor', 52.4955, 51.0662))
        self.assertEqual(rows[185], ('Renas', 54.3368, 48.2497))


if __name__ == '__main__':
    unittest.main()
