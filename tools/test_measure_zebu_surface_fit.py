"""Verify the original SH036 fit adapter and bounded surface certificate."""
from contextlib import redirect_stdout
from io import StringIO
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import numpy as np

import measure_zebu_surface_fit as zebu

REPORT_FOLDER = zebu.ASSETS.parents[2] / 'integration/sh036-zebu'


class ZebuFitTests(unittest.TestCase):
    def test_loader_pins_and_distinguishes_both_unmodified_meshes(self):
        vertices, names, groups, meshes = zebu.load()
        self.assertEqual((len(vertices), len(names), len(groups)), (1904,24,24))
        self.assertEqual(len(meshes['detailed'][1].faces), 3468)
        self.assertEqual(len(meshes['native'][1].faces), 192)
        self.assertEqual(meshes['native'][0], 1790)
        self.assertEqual(sum(len(i) for i in groups), len(vertices))
        self.assertTrue(np.array_equal(np.sort(np.concatenate(groups)), np.arange(len(vertices))))

    def test_loader_refuses_byte_edits_to_manifest_and_source_geometry(self):
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary) / 'assets'
            shutil.copytree(zebu.ASSETS, folder)
            manifest = folder / 'SOURCE-GEOMETRY.json'
            original = manifest.read_bytes()
            manifest.write_bytes(original + b' ')
            with self.assertRaisesRegex(ValueError, 'Changed SH036 manifest'):
                zebu.load(folder)
            manifest.write_bytes(original)
            triangles = folder / 'source-triangles.bin'
            data = triangles.read_bytes()
            triangles.write_bytes(data[:-1] + bytes([data[-1]^1]))
            with self.assertRaisesRegex(ValueError, 'Changed SH036 fit input: source-triangles.bin'):
                zebu.load(folder)

    def test_bounded_original_surfaces_not_just_vertex_samples(self):
        committed = json.loads((REPORT_FOLDER / 'SURFACE-FIT.json').read_bytes())
        measured = zebu.audit()
        self.assertEqual(measured['refinementDepth'], 8)
        self.assertFalse(measured['samplesOnly'])
        self.assertEqual(len(measured['pieces']), 24)
        # Independently recompute full surface bounds. Platform-specific floating
        # nominations can choose different exact witnesses, but must satisfy these limits.
        for key, lower, upper in (('detailed',540,700), ('native',4970,5000)):
            summary = measured['summary'][key]
            sample = summary['largestSample']
            self.assertGreaterEqual(sample['certifiedLowerMillimetres'], lower)
            self.assertLessEqual(summary['fullUpperMillimetres'], upper)
            self.assertGreaterEqual(summary['fullUpperMillimetres'], sample['certifiedLowerMillimetres'])
            self.assertEqual(summary['fullUpperMillimetres'], max(row['sourceComparisons'][key]['fullPieceSurfaceUpperMillimetres'] for row in measured['pieces']))
            self.assertGreaterEqual(committed['summary'][key]['largestSample']['certifiedLowerMillimetres'], lower)
            self.assertLessEqual(committed['summary'][key]['fullUpperMillimetres'], upper)
        self.assertEqual(zebu.fit.DEPTH, 6)

    def test_direct_original_to_original_witnesses(self):
        report = zebu.original_difference()
        self.assertEqual(report['native']['mandatoryOppositeSourceVertex'], 1744)
        self.assertEqual(report['native']['strictLowerMillimetres'], 4967)
        self.assertEqual(report['detailed']['strictLowerMillimetres'], 88)
        committed = json.loads((REPORT_FOLDER / 'ORIGINAL-DIFFERENCE.json').read_bytes())
        self.assertEqual(report['native']['strictLowerMillimetres'], committed['native']['strictLowerMillimetres'])

    def test_certificate_failure_restores_shared_refinement_depth(self):
        previous = zebu.fit.DEPTH
        with patch.object(zebu.fit, 'certified_upper', side_effect=AssertionError('bound refused')), self.assertRaises(AssertionError):
            zebu.certified_zebu_upper(None, None, 0)
        self.assertEqual(zebu.fit.DEPTH, previous)
        with patch.object(zebu.fit, 'audit_fan', side_effect=AssertionError('invalid fan')), self.assertRaisesRegex(ValueError, 'Could not certify radial fan'):
            zebu.build_pieces(np.zeros((4,3)), ['invalid'], [np.arange(4)])

    def test_cli_selects_report_modes(self):
        for flags, operation, expected in (([], 'audit', [False]), (['--samples-only'], 'audit', [True]), (['--difference'], 'original_difference', [])):
            with self.subTest(flags=flags), patch('sys.argv', ['fit'] + flags), patch.object(zebu, operation, return_value={'passed': True}) as run, redirect_stdout(StringIO()) as stdout:
                zebu.main()
                run.assert_called_once_with(*expected)
                self.assertEqual(json.loads(stdout.getvalue()), {'passed': True})


if __name__ == '__main__':
    unittest.main()
