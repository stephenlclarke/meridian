"""Check immutable SH029 meshes, material inputs and independent controls."""
import contextlib
import hashlib
from io import StringIO
import json
from math import hypot
from pathlib import Path
import runpy
import shutil
import struct
import sys
import tempfile
import tomllib
import unittest
from unittest import mock

import prepare_okapi_geometry as okapi
import verify_okapi_geometry as proof


class TestOkapiSources(unittest.TestCase):
    def test_packaging_reproduces_both_metre_meshes(self):
        for name, data in okapi.prepare(okapi.ASSETS).items():
            with self.subTest(name=name):
                self.assertEqual((okapi.ASSETS / name).read_bytes(), data)
        manifest = json.loads((okapi.ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())
        self.assertEqual(manifest['metresPerSourceUnit'], 1)
        self.assertEqual((manifest['native']['faces'], manifest['native']['faceLimit']), (120, 512))

    def test_each_original_normal_material_uv_and_shared_map(self):
        source = tomllib.loads((okapi.ASSETS / 'resource.toml').read_text())
        shared = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())['originalTextures']
        for model_name, file, face_count, counts in (
            ('design', 'visual.bin', 2376, [340, 268, 552, 1216]),
            ('nativeDesign', 'native-visual.bin', 120, [68, 36, 0, 16]),
        ):
            with self.subTest(mesh=model_name):
                self.check_visual(source[model_name], file, face_count, counts, shared)

    def check_visual(self, mesh, file, face_count, counts, shared):
        visual = (okapi.ASSETS / file).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'OKAPVIZ1', 3 * face_count, 56))
        corners = iter(struct.iter_unpack('<12fII', visual[16:]))
        uses = [0, 0, 0, 0]
        for face, material_id, mapping in zip(mesh['faces'], mesh['faceMaterials'], mesh['faceTextures'], strict=True):
            a, b, c = [mesh['vertices'][i] for i in face]
            ab = [q - p for p, q in zip(a, b, strict=True)]
            ac = [q - p for p, q in zip(a, c, strict=True)]
            cross = [ab[j] * ac[k] - ab[k] * ac[j] for j, k in ((1, 2), (2, 0), (0, 1))]
            normal = [v / hypot(*cross) for v in cross]
            material = mesh['materials'][material_id]
            tint = [(material['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
            slot = mapping['texture']
            for coords, uv in zip((a, b, c), mapping['uv'], strict=True):
                raw = next(corners)
                expected = struct.unpack('<12f', struct.pack('<12f', *coords, *normal, *tint, *uv))
                self.assertEqual(raw[:12], expected)
                self.assertEqual(raw[12:], (slot if slot >= 0 else 0xffff_ffff, int(material['emissive'])))
            uses[slot] += 1
        self.assertEqual(uses, counts)
        self.assertIsNone(next(corners, None))
        self.assertEqual({Path(path).stem for path in shared},
                         {asset['asset'] for asset in mesh['textures']})
        for name, expected in shared.items():
            raw = (proof.ROOT / name).read_bytes()
            self.assertEqual(len(raw), expected['bytes'])
            self.assertEqual(hashlib.sha256(raw).hexdigest(), expected['sha256'])

    def test_cli_refuses_changed_streams_and_wrong_original_hull(self):
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp) / 'sh029'
            shutil.copytree(okapi.ASSETS, folder)
            args = [okapi.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(okapi.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(okapi.__file__, run_name='__main__')
                (folder / 'native-visual.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed SH029 encoding'):
                    runpy.run_path(okapi.__file__, run_name='__main__')
            shutil.copyfile(okapi.ROOT / 'assets/fleet/sh010/resource.toml', folder / 'resource.toml')
            with self.assertRaisesRegex(ValueError, 'changed SH029 original'):
                okapi.prepare(folder)

    def test_legacy_activation_and_physical_contract_pin_the_metre_native_source(self):
        evidence = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        activation, physical = evidence['legacyMetreAuthority']['files']
        a, p = [(proof.ROOT / row['artifact']).read_bytes() for row in (activation, physical)]
        for item, raw in zip((activation, physical), (a, p), strict=True):
            self.assertEqual(hashlib.sha256(raw).hexdigest(), item['sha256'])
        self.assertIn(b'static let id = "SH029"', a)
        self.assertIn(b'physical.metresPerSourceUnit == 1', a)
        self.assertIn(okapi.PINS['contract.toml'].encode(), a)
        section = p.split(b'private static func calibrationCase14(', 1)[1].split(b'@inline(never)', 1)[0]
        self.assertIn(okapi.NATIVE_PATH.encode(), section)
        self.assertIn(okapi.SOURCE_SHA.encode(), section)
        self.assertIn(b'metresPerSourceUnit = 1', section)

    def test_pallet_rail_cockpit_and_station_controls_replay(self):
        result = json.loads((proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(json.loads(json.dumps(proof.verify())), result)
        self.assertEqual(list(result['clearanceMillimetres'].values()), [628, 208, 628, 208, 29, 29, 29])
        self.assertEqual(result['alignedMillimetres'], 45018)
        self.assertEqual(result['offsetStartMillimetres'], 979)
        self.assertEqual(result['sourceLipWitness']['fraction'], .5)
        self.assertEqual(len(result['originalJoints']), 5)
        for row in result['originalJoints']:
            self.assertEqual(row['sourceFaceHitCountsAboveBelow'], [[1, 1]] * 4)
        for gap in result['originalTopRailGapsMetres']:
            self.assertAlmostEqual(gap, .1)
        with contextlib.redirect_stdout(StringIO()) as out:
            runpy.run_path(proof.__file__, run_name='__main__')
        self.assertEqual(json.loads(out.getvalue()), result)

    def test_top_rail_control_refuses_contact_or_wrong_material(self):
        bottom = [(0, 0, 0), (1, 0, 0), (0, 0, 1)]
        with self.assertRaisesRegex(ValueError, 'changed source top-rail spacing'):
            proof.pallet_rail_gap([bottom, bottom], [0, 2], [(0, 1), (1, 2)])
        with self.assertRaisesRegex(ValueError, 'changed original pallet or rail material'):
            proof.pallet_rail_gap([bottom, bottom], [0, 0], [(0, 1), (1, 2)])

    def test_controls_refuse_wrong_edges_and_missing_separation(self):
        lip = [(0, 0, 0), (4, 0, 0), (0, 4, 0)]
        with self.assertRaisesRegex(ValueError, 'no strict source lip crossing'):
            proof.lip_intersection(lip, (0, 1, 0), 3, (1, -1))
        top = [[(0, 1, 0), (4, 1, 0), (0, 1, 4)]]
        with self.assertRaisesRegex(ValueError, 'parity ray hit a source edge'):
            proof.inside_y((0, 0, 0), top)
        with self.assertRaisesRegex(ValueError, 'left the source volume'):
            proof.inside_y((1, 0, 1), top)
        with self.assertRaisesRegex(ValueError, 'misses a separating plane'):
            proof.separating_planes([[(0, 0, 0), (2, 0, 0)]], [(1, 0, 0)], [(1, 0, 0)])

    def test_fit_contact_and_numerical_helper_pins(self):
        criteria = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        replay = criteria['independentReplay']
        for name, expected in (replay['tools'] | replay['fixtures']).items():
            self.assertEqual(hashlib.sha256((proof.ROOT / name).read_bytes()).hexdigest(), expected)
        for name, expected in criteria['air']['reports'].items():
            self.assertEqual(hashlib.sha256((proof.INTEGRATION / name).read_bytes()).hexdigest(), expected)
        summary = json.loads((proof.INTEGRATION / 'SURFACE-FIT.json').read_bytes())['summary']
        for kind, low, upper in (('detailed', 373, 386), ('native', 1600, 1611)):
            self.assertEqual(summary[kind]['largestSample']['certifiedLowerMillimetres'], low)
            self.assertEqual(summary[kind]['fullUpperMillimetres'], upper)
