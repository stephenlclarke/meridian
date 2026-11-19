"""Verify SH036 fixed-orientation physical crossing and source-pinning controls."""
from contextlib import redirect_stdout
from io import StringIO
import json
from pathlib import Path
import runpy
import tempfile
import unittest

from verify_zebu_station import FIXTURE, ROOT, lip_intersection, verify


class ZebuStationTests(unittest.TestCase):
    def test_aligned_whole_hull_clears_and_offset_native_vertex_crosses_lip(self):
        report = verify()
        self.assertEqual(report['aligned']['netMillimetres'], 39038)
        self.assertGreater(report['aligned']['guardedFirstLeadingEdgeZMetres'], report['aligned']['stationLipZMetres'][1])
        self.assertLess(report['aligned']['guardedLastTrailingEdgeZMetres'], report['aligned']['stationLipZMetres'][0])
        self.assertEqual(report['offsetStart']['netMillimetres'], 1159)
        crossing = report['nativeLipCrossing']
        self.assertEqual((crossing['nativeLocalVertex'], crossing['nativeGlobalVertex'], crossing['stationFace']), (31,1821,482))
        self.assertEqual(crossing['pointMetres'], [69.9,5.03,136.32])
        self.assertEqual(crossing['actorCentreZMetres'], 161.82)
        self.assertTrue(0 < crossing['fraction'] < 1 and min(crossing['barycentrics']) > 0)
        self.assertAlmostEqual(sum(crossing['barycentrics']), 1)
        self.assertEqual(report, json.loads(FIXTURE.with_name('STATION-RESULT.json').read_bytes()))

    def test_cli_prints_identical_exact_result(self):
        with redirect_stdout(StringIO()) as stdout:
            runpy.run_module('verify_zebu_station', run_name='__main__')
        self.assertEqual(stdout.getvalue().encode(), FIXTURE.with_name('STATION-RESULT.json').read_bytes())

    def test_changed_original_station_bytes_are_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in json.loads(FIXTURE.read_bytes())['sourcePins']:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                data = (ROOT / name).read_bytes()
                path.write_bytes(data if not name.endswith('sh028.physics') else data[:-1]+bytes([data[-1]^1]))
            with self.assertRaisesRegex(ValueError, 'Changed station sweep input assets/legacy/sh028.physics'):
                verify(root, FIXTURE)

    def test_native_contact_refuses_wrong_staged_claims(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'fixture.json'
            value = json.loads(FIXTURE.read_bytes())
            value['offset']['actorXMetres'] = 0
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, 'No strict original source lip crossing'):
                verify(ROOT, path)
            value['offset']['nativeLocalVertex'] = -1
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, 'not an original native vertex'):
                verify(ROOT, path)
            value['aligned']['actorCentreZMetres'] = [163, 140]
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, 'does not fully traverse'):
                verify(ROOT, path)
        with self.assertRaisesRegex(ValueError, 'Wrong station lip plane'):
            lip_intersection([(0,0,0),(1,0,1),(0,1,0)], (0,0,1), 0, [2,0])


if __name__ == '__main__':
    unittest.main()
