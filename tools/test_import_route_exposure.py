"""Pinned original route-exposure import checks; no game/compiler execution."""
import hashlib
import json
from pathlib import Path
import struct
import unittest
from unittest.mock import patch

import import_route_exposure as importer

ROOT = Path(__file__).resolve().parents[1]


class RouteExposureImportTests(unittest.TestCase):
    def test_unpinned_source_refuses_before_conversion(self):
        with self.assertRaisesRegex(ValueError, 'hash mismatch'):
            importer.convert(b'foreign catalogue')

    def test_committed_bytes_match_source_manifest_and_all_original_ids(self):
        manifest = json.loads((ROOT / 'assets/legacy/route-exposure-manifest.json').read_text())
        raw = (ROOT / manifest['output']).read_bytes()
        self.assertEqual(manifest['sourceRevision'], importer.REVISION)
        self.assertEqual(manifest['sourceSHA256'], importer.SOURCE_SHA)
        self.assertEqual(manifest['outputSHA256'], hashlib.sha256(raw).hexdigest())
        self.assertEqual(manifest['outputBytes'], len(raw))
        self.assertTrue((ROOT / manifest['notice']).is_file())
        self.assertEqual(struct.unpack_from('<8sH', raw), (b'MRISK001', 256))
        self.assertEqual(len(raw), 2570)
        for index, (identifier, risk) in enumerate(struct.iter_unpack('<Hd', raw[10:])):
            self.assertEqual(identifier, index)
            self.assertTrue(0 <= risk <= 1)
        self.assertEqual(struct.unpack_from('<Hd', raw, 10 + 185 * 10), (185, 0.2))

    def test_structural_converter_rejects_bad_ids_and_invalid_exposure_after_identity_gate(self):
        # Deliberately synthetic parser inputs: the pinned production hash is patched
        # only inside this unit case; no fixture grants original/runtime authority.
        rows = [{'galaxy': 0, 'system': n, 'hostileFraction': 0.2} for n in range(256)]
        for change in ['duplicate', 'nan', 'negative', 'too_high', 'bool']:
            data = json.loads(json.dumps(rows))
            if change == 'duplicate':
                data[1]['system'] = 0
            else:
                data[1]['hostileFraction'] = {'nan': float('nan'), 'negative': -0.1, 'too_high': 1.1, 'bool': True}[change]
            raw = json.dumps({'version': 'unit-only', 'systems': data}).encode()
            with patch.object(importer, 'SOURCE_SHA', hashlib.sha256(raw).hexdigest()):
                with self.assertRaises(ValueError):
                    importer.convert(raw)


if __name__ == '__main__':
    unittest.main()
