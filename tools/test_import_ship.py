"""Headless importer checks; these do not run geometry or GPU qualification."""

import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import unittest


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("import_ship", ROOT / "tools/import_ship.py")
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


class ShipImportTests(unittest.TestCase):
    def test_unselected_source_is_refused(self):
        with self.assertRaisesRegex(ValueError, "hash differs"):
            IMPORTER.convert(b"id = 'SH025'\n")

    def test_packaged_mesh_matches_provenance_and_vertex_contract(self):
        manifest = json.loads((ROOT / "assets/legacy/manifest.json").read_text())
        mesh = (ROOT / manifest["output"]).read_bytes()
        self.assertEqual(hashlib.sha256(mesh).hexdigest(), manifest["outputSHA256"])
        self.assertEqual(len(mesh), manifest["outputBytes"])
        self.assertEqual(len(mesh), 3484 * 3 * 24)
        # The first indexed triangle is 23,22,21 in the immutable source.
        first_position = struct.unpack_from("<3f", mesh)
        expected = [-20.458895841545136, -5.3505493693338595, -59.45309420575484]
        for actual, reference in zip(first_position, expected, strict=True):
            self.assertAlmostEqual(actual, reference, places=4)
        self.assertTrue((ROOT / manifest["notice"]).is_file())


if __name__ == "__main__":
    unittest.main()
