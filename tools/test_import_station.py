"""Pinned original station/hull import tests; no Rust, GPU or legacy runtime execution."""
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('import_station', ROOT/'tools/import_station.py')
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


class StationImportTests(unittest.TestCase):
    def test_source_mismatch_refuses_before_importing_or_writing(self):
        with self.assertRaisesRegex(ValueError, 'hash mismatch'):
            IMPORTER.outputs(b'unselected', b'unselected')

    def test_reads_the_explicit_git_revision_instead_of_working_files_or_head(self):
        raw = b'isolated source input'
        with patch.dict(IMPORTER.PINS, {'SH025':hashlib.sha256(raw).hexdigest()}):
            with patch.object(IMPORTER.subprocess, 'check_output', return_value=raw) as call:
                self.assertEqual(IMPORTER.read_blob(Path('/reference-objects'),'SH025'),raw)
                call.assert_called_once_with(['git','-C','/reference-objects','show',IMPORTER.REVISION+':Sources/L337Core/Resources/Ships/SH025.toml'])

    def test_packaged_outputs_hash_counts_format_and_notice_match(self):
        manifest=json.loads((ROOT/'assets/legacy/station-manifest.json').read_text())
        self.assertEqual(manifest['sourceRevision'], IMPORTER.REVISION)
        self.assertEqual({x['id']:x['sha256'] for x in manifest['sourceFiles']}, IMPORTER.PINS)
        for name, expected in manifest['outputs'].items():
            raw=(ROOT/'assets/legacy'/name).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), expected['sha256'])
            self.assertEqual(len(raw),expected['bytes'])
        self.assertEqual(len((ROOT/'assets/legacy/sh028.mesh').read_bytes()),10056*3*24)
        for name,counts in [('sh028.physics',(272,488)),('sh025.physics',(536,964))]:
            raw=(ROOT/'assets/legacy'/name).read_bytes()
            self.assertEqual(raw[:8],b'MPHYS001')
            vertices,triangles=struct.unpack_from('<II',raw,8)
            self.assertEqual((vertices,triangles),counts)
            self.assertEqual(len(raw),16+vertices*24+triangles*12)
            indices=struct.unpack_from('<'+'I'*(triangles*3),raw,16+vertices*24)
            self.assertTrue(all(i<vertices for i in indices))
        self.assertTrue((ROOT/manifest['notice']).is_file())

    def test_original_port_is_not_a_fallback_aperture(self):
        raw=(ROOT/'assets/legacy/sh028.port').read_bytes()
        self.assertEqual(raw[:8],b'MPORT001')
        self.assertEqual(struct.unpack_from('<10dI',raw,8),(0.,0.,136.32,0.,0.,1.,0.,1.,0.,53.52,4))
        self.assertEqual(struct.unpack_from('<12d',raw,92),(-62.32,-50.8,0.,62.32,-50.8,0.,62.32,50.8,0.,-62.32,50.8,0.))
        self.assertEqual(len(raw),188)


if __name__ == '__main__':
    unittest.main()
