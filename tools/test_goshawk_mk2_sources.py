"""Check the immutable SH009 preparation and independent collision certificates."""
import contextlib
from io import StringIO
import json
from pathlib import Path
import runpy
import shutil
import sys
import tempfile
import unittest
from unittest import mock

import prepare_goshawk_mk2_geometry as geometry
import verify_goshawk_mk2_fixture_source as fixture


class GoshawkMk2SourceTests(unittest.TestCase):
    def test_current_originals_regenerate_all_sh009_outputs(self):
        generated = geometry.prepare(geometry.ASSETS)
        self.assertTrue(generated)
        for name, expected in generated.items():
            with self.subTest(output=name):
                self.assertEqual((geometry.ASSETS / name).read_bytes(), expected)
        membership = generated['source-piece-membership.bin']
        self.assertEqual((len(membership), set(membership)), (2672, set(range(73))))

    def test_cli_generates_and_checks_but_rejects_edited_outputs_and_source(self):
        with tempfile.TemporaryDirectory() as temporary, contextlib.redirect_stdout(StringIO()):
            folder = Path(temporary) / 'sh009'
            shutil.copytree(geometry.ASSETS, folder)
            output = folder / 'source-piece-membership.bin'
            output.unlink()
            cmd = [geometry.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', cmd):
                runpy.run_path(geometry.__file__, run_name='__main__')
            edited = cmd + ['--check']
            with mock.patch.object(sys, 'argv', edited):
                runpy.run_path(geometry.__file__, run_name='__main__')
                output.write_bytes(b'mutated')
                with self.assertRaisesRegex(ValueError, 'changed SH009 geometry encoding'):
                    runpy.run_path(geometry.__file__, run_name='__main__')
            original = folder / 'native-source.json'
            original.write_bytes(original.read_bytes() + b' ')
            with self.assertRaisesRegex(ValueError, 'changed SH009 source'):
                geometry.prepare(folder)

    def test_analytic_separation_uses_the_full_sweep(self):
        piece = [[(0, 0, 0), (1, 1, 1)]]
        axis = [(1, 0, 0)]
        self.assertEqual(fixture.swept_planes_mm(piece, [(2, 0, 0), (3, 0, 0)], axis), 979)
        with self.assertRaisesRegex(ValueError, 'no separating plane'):
            fixture.swept_planes_mm(piece, [(2, 0, 0), (1, 0, 0)], axis)

    def test_analytic_lip_witness_requires_strict_face_interior(self):
        triangle = [(0, 0, 0), (0, 4, 0), (4, 0, 0)]
        inside = fixture.lip_contact(triangle, (0, 1, 2), (0, 1, 0), 1, [1, -1])
        self.assertEqual(inside['barycentric'], [0.5, 0.25, 0.25])
        self.assertEqual(inside['sweepFraction'], 0.5)
        with self.assertRaisesRegex(ValueError, 'strict lip interior'):
            fixture.lip_contact(triangle, (0, 1, 2), (0, 1, 0), 3, [1, -1])

    def test_actual_fixtures_match_the_independent_air_results(self):
        recorded = fixture.ROOT / 'integration/sh009-goshawk-mk2/SH009-FIXTURE-RESULT.json'
        expected = json.loads(recorded.read_bytes())
        self.assertEqual(fixture.verify(), expected)
        self.assertEqual(expected['channelClearanceMillimetres'], 1462)
        self.assertEqual(expected['ventralClearanceMillimetres'], [614, 614])
        self.assertEqual(expected['lipIntersection']['triangleVertices'], [265, 266, 270])
        with contextlib.redirect_stdout(StringIO()) as out:
            runpy.run_path(fixture.__file__, run_name='__main__')
        self.assertEqual(json.loads(out.getvalue()), expected)
