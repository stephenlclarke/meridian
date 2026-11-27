"""Verify the original SH041 bounded exposed and full patch certificates."""
from contextlib import redirect_stdout
from io import BytesIO, TextIOWrapper
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import numpy as np
import measure_serval_surface_fit as measure


class ServalFitTests(unittest.TestCase):
    def test_original_source_faces_and_groups(self):
        points,groups,meshes=measure.load();self.assertEqual((len(points),len(groups)),(2300,23));self.assertEqual([len(mesh.faces) for _,mesh in meshes.values()],[4236,224]);self.assertTrue(np.array_equal(np.sort(np.concatenate(groups)),np.arange(len(points))))

    def test_source_pin_mutation_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            folder=Path(temp)/'assets';shutil.copytree(measure.ASSETS,folder);manifest=folder/'SOURCE-GEOMETRY.json';raw=manifest.read_bytes();manifest.write_bytes(raw+b' ')
            with self.assertRaisesRegex(ValueError,'Changed original SH041 manifest'):measure.load(folder)
            manifest.write_bytes(raw);file=folder/'source-triangles.bin';raw=file.read_bytes();file.write_bytes(raw[:-1]+bytes([raw[-1]^1]))
            with self.assertRaisesRegex(ValueError,'Changed SH041 fit input source-triangles.bin'):measure.load(folder)

    def test_full_surface_bounds_against_both_original_meshes(self):
        result=measure.audit();self.assertEqual(result['refinementDepth'],8);self.assertEqual(len(result['pieces']),23)
        for name,lower,upper in (('detailed',270,360),('native',1310,1340)):
            row=result['summary'][name];self.assertGreaterEqual(row['largestCertifiedExposedSample']['certifiedLowerMillimetres'],lower);self.assertLessEqual(row['fullUpperMillimetres'],upper);self.assertEqual(row['fullUpperMillimetres'],max(p['comparisons'][name]['fullUpperMillimetres'] for p in result['pieces']))
        self.assertEqual(measure.fit.DEPTH,6)

    def test_original_difference_and_failure_controls(self):
        result=measure.original_difference();self.assertEqual(result['detailed']['strictLowerMillimetres'],88);self.assertEqual(result['native']['strictLowerMillimetres'],1301);self.assertEqual(result['native']['oppositeSourceVertex'],2054)
        with patch.object(measure.fit,'certified_upper',side_effect=AssertionError('invalid')),self.assertRaises(AssertionError):measure.bounded_upper(None,None,0)
        self.assertEqual(measure.fit.DEPTH,6)
        with patch.object(measure,'NAMES',('bad',)),patch.object(measure.fit,'audit_fan',side_effect=AssertionError('bad')),self.assertRaisesRegex(ValueError,'Cannot certify SH041 radial fan'):measure.pieces_for(np.zeros((4,3)),[np.arange(4)])

    def test_both_cli_modes_canonical_lf(self):
        for flags,name in (([],'audit'),(['--difference'],'original_difference')):
            with self.subTest(flags=flags),patch('sys.argv',['measure']+flags),patch.object(measure,name,return_value={'passed':True}) as run,BytesIO() as raw,TextIOWrapper(raw,encoding='utf-8',newline='\r\n') as out,redirect_stdout(out):
                measure.main();run.assert_called_once_with();out.flush();self.assertEqual(raw.getvalue(),b'{\n  "passed": true\n}\n')


if __name__=='__main__':unittest.main()
