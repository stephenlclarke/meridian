"""Check immutable SH023 meshes, material inputs and independent controls."""
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

import prepare_peregrine_mk3_geometry as peregrine_mk3
import verify_peregrine_mk3_geometry as proof


class TestPeregrineMk3Sources(unittest.TestCase):
    def test_packaging_reproduces_both_metre_meshes(self):
        for name, data in peregrine_mk3.prepare(peregrine_mk3.FOLDER).items():
            with self.subTest(name=name):
                self.assertEqual((peregrine_mk3.FOLDER / name).read_bytes(), data)
        manifest = json.loads((peregrine_mk3.FOLDER / 'SOURCE-GEOMETRY.json').read_bytes())
        self.assertEqual(manifest['metresPerSourceUnit'], '250000/19057')
        self.assertEqual((manifest['native']['faces'], manifest['native']['faceLimit']), (512, 512))

    def test_each_original_normal_material_uv_and_shared_map(self):
        source = tomllib.loads((peregrine_mk3.FOLDER / 'resource.toml').read_text())
        shared = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())['originalTextures']
        for model_name, file, face_count, counts in (
            ('design', 'visual.bin', 3484, [1356, 0, 2128]),
            ('nativeDesign', 'native-visual.bin', 512, [322, 0, 190]),
        ):
            with self.subTest(mesh=model_name):
                self.check_visual(source[model_name], file, face_count, counts, shared)

    def check_visual(self, mesh, file, face_count, counts, shared):
        visual = (peregrine_mk3.FOLDER / file).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'PRG3VIZ1', 3 * face_count, 56))
        corners = iter(struct.iter_unpack('<12fII', visual[16:]))
        uses = [0, 0, 0]
        for face, material_id, mapping in zip(mesh['faces'], mesh['faceMaterials'], mesh['faceTextures'], strict=True):
            a, b, c = [tuple(q * float(peregrine_mk3.SCALE) for q in mesh['vertices'][i]) for i in face]
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
                self.assertEqual(raw[12:], (slot, int(material['emissive'])))
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
            folder = Path(temp) / 'sh023'
            shutil.copytree(peregrine_mk3.FOLDER, folder)
            args = [peregrine_mk3.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(peregrine_mk3.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(peregrine_mk3.__file__, run_name='__main__')
                (folder / 'native-visual.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed SH023 encoding'):
                    runpy.run_path(peregrine_mk3.__file__, run_name='__main__')
            shutil.copyfile(peregrine_mk3.ROOT / 'assets/fleet/sh010/resource.toml', folder / 'resource.toml')
            with self.assertRaisesRegex(ValueError, 'changed SH023 original'):
                peregrine_mk3.prepare(folder)

    def test_source_partition_rejects_malformed_membership(self):
        with self.assertRaisesRegex(ValueError, 'duplicate vertex assignment'):
            peregrine_mk3.membership([(0, 1, 2)], [[0, 1, 2], [1]], 3)
        with self.assertRaisesRegex(ValueError, 'crosses a support'):
            peregrine_mk3.membership([(0, 1, 2)], [[0, 1], [2]], 3)
        with self.assertRaisesRegex(ValueError, 'source vertex missing'):
            peregrine_mk3.membership([(0, 1, 2)], [[0, 1]], 3)

    def test_bow_cockpit_and_station_controls_replay(self):
        result = json.loads((proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(json.loads(json.dumps(proof.verify())), result)
        self.assertEqual(list(result['clearanceMillimetres'].values()), [1961, 2746, 2090, 2090])
        self.assertEqual(result['alignedMillimetres'], 41904)
        self.assertEqual(result['offsetStartMillimetres'], 978)
        self.assertAlmostEqual(result['sourceLipWitness']['fraction'], .5, places=14)
        cockpit = result['cockpitCrossmemberJoint']
        self.assertEqual(cockpit['sourceFaceHitCountsAboveBelow'], [[1, 1]] * 4)
        self.assertEqual(cockpit['primaryBodyToCrossmemberVerticalGapMillimetres'], [360, 360])
        with contextlib.redirect_stdout(StringIO()) as out:
            runpy.run_path(proof.__file__, run_name='__main__')
        self.assertEqual(json.loads(out.getvalue()), result)

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
        for kind, low, upper in (('detailed', 1129, 1133), ('native', 2405, 2407)):
            self.assertEqual(summary[kind]['largestSample']['certifiedLowerMillimetres'], low)
            self.assertEqual(summary[kind]['fullUpperMillimetres'], upper)
