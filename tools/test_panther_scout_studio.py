"""Independent source topology, variant identity and clearance witness checks."""

import json
import hashlib
import shutil
import tempfile
import unittest
from pathlib import Path

from prepare_panther_scout_studio import ASSETS, FIXTURES, OUTPUT, authenticated, clearance_fixtures, definition
from verify_panther_scout_station import OUTPUT_REPORT, verify


class PantherScoutStudioTests(unittest.TestCase):
    def test_original_geometry_appearance_and_variant(self):
        source, _ = authenticated(ASSETS)
        package = definition()
        self.assertEqual(package, json.loads(OUTPUT.read_bytes()))
        self.assertEqual((package['id'], package['name']), ('SH033', 'Panther Scout'))
        self.assertIsNone(package['engineering'])
        self.assertEqual(package['metresPerUnit'], 1)
        self.assertEqual(package['roundingMetres'], .02)
        self.assertEqual(len(package['provenance']['groupNames']), 35)
        self.assertIn('scout-survey-spine', package['provenance']['groupNames'])
        self.assertNotIn('left-side-window', package['provenance']['groupNames'])
        for source_key, package_key, counts in [('design','detail',(2744,5212)), ('nativeDesign','native',(212,320))]:
            original, derived = source[source_key], package[package_key]
            self.assertEqual((len(derived['vertices']), len(derived['triangles'])), counts)
            for old_face, new_face in zip(original['faces'], derived['triangles'], strict=True):
                self.assertEqual([original['vertices'][i] for i in old_face], [derived['vertices'][i] for i in new_face])
                self.assertEqual(len({derived['groups'][i] for i in new_face}), 1)
        self.assertEqual(package['faceMaterials'], source['design']['faceMaterials'])
        self.assertEqual([row['uv'] for row in package['faceTextures']], [row['uv'] for row in source['design']['faceTextures']])
        self.assertEqual(len(package['faceMaterials']), 5212)

    def test_open_and_solid_witnesses_are_bound_to_source(self):
        result = clearance_fixtures(definition())
        self.assertEqual(result, json.loads(FIXTURES.read_bytes()))
        self.assertEqual(len(result['openRoutes']), 4)
        self.assertEqual(len(result['solidRoutes']), 4)
        self.assertGreater(min(row['provedLowerGapMetres'] for row in result['openRoutes']), .57)
        self.assertTrue(all(row['expectedPassed'] for row in result['openRoutes']))
        self.assertTrue(all(not row['expectedPassed'] for row in result['solidRoutes']))
        for row in result['solidRoutes']:
            for a, b, witness in zip(row['start'], row['end'], row['supportInteriorWitness'], strict=True):
                self.assertAlmostEqual((a+b)/2, witness)
        self.assertGreater(min(result['stationAperture']['alignedProjectionMarginMetres']), 43)
        self.assertGreater(result['stationAperture']['offsetProjectionExcessMetres'], 7)

    def test_original_mutation_is_refused_before_geometry(self):
        with tempfile.TemporaryDirectory() as tmp:
            copied = Path(tmp)/'sh033'
            shutil.copytree(ASSETS, copied)
            path = copied/'native-export.json'
            original = path.read_bytes()
            path.write_bytes(original.replace(b'SH033', b'SH032', 1))
            with self.assertRaisesRegex(ValueError, 'Changed Scout source native-export.json'):
                definition(copied)

    def test_actual_station_triangles_and_native_lip_crossing(self):
        report = verify()
        self.assertEqual(report, json.loads(OUTPUT_REPORT.read_bytes()))
        self.assertEqual(report['aligned']['minimumSeparationMillimetres'], 43218)
        self.assertEqual(report['offsetOpening']['minimumSeparationMillimetres'], 979)
        crossing = report['offsetLipContact']
        self.assertEqual(crossing['stationFace'], 482)
        self.assertEqual(crossing['nativePointMetres'], [0.0,-.524,17.0])
        self.assertEqual(crossing['crossingFraction'], .5)
        self.assertGreater(min(crossing['barycentrics']), 0)
        self.assertFalse(crossing['expectedClear'])

    def test_retained_fit_bounds_and_independent_mesh_witnesses(self):
        report = json.loads(OUTPUT.with_name('SURFACE-FIT.json').read_bytes())
        self.assertEqual(report['packageSHA256'], hashlib.sha256(OUTPUT.read_bytes()).hexdigest())
        self.assertEqual(report['groups'], 35)
        self.assertEqual(len(report['perGroup']), 35)
        for domain, minimum, maximum in [('detailed',437,1001), ('native',2164,2292)]:
            summary = report['summary'][domain]
            self.assertEqual(summary['largestSample']['certifiedLowerMillimetres'], minimum)
            self.assertEqual(summary['fullUpperMillimetres'], maximum)
            self.assertEqual(max(row['comparisons'][domain]['fullRoundedPieceUpperMillimetres']
                                 for row in report['perGroup']), maximum)
            self.assertLess(minimum, maximum)
        self.assertEqual(report['originalCrossSourceDiscrepancy']['detailed']['strictLowerMillimetres'], 88)
        self.assertEqual(report['originalCrossSourceDiscrepancy']['native']['strictLowerMillimetres'], 1848)


if __name__ == '__main__':
    unittest.main()
