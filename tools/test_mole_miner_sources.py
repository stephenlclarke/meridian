"""Check immutable SH016 inputs, visuals and independent certificates."""
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

import prepare_mole_miner_geometry as mole
import verify_mole_miner_geometry as proof


class TestMoleMinerSources(unittest.TestCase):
    def test_packaging_reproduces_both_meshes(self):
        for name, data in mole.prepare(mole.FOLDER).items():
            with self.subTest(name=name):
                self.assertEqual((mole.FOLDER / name).read_bytes(), data)
        manifest = json.loads((mole.FOLDER / 'SOURCE-GEOMETRY.json').read_bytes())
        self.assertTrue(manifest['sourceOriginUnchanged'])
        self.assertEqual(manifest['native']['faceLimitStatus'], 'ORIGINAL_EXCEEDS_LIMIT')
        self.assertEqual((manifest['native']['faces'], manifest['native']['faceLimit']), (1100, 512))

    def test_each_original_normal_material_uv_and_shared_map(self):
        source = tomllib.loads((mole.FOLDER / 'resource.toml').read_text())
        shared = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())['originalTextures']
        for model_name, file, face_count, counts in (
            ('design', 'visual.bin', 12_836, [476, 0, 12_360]),
            ('nativeDesign', 'native-visual.bin', 1100, [212, 0, 888]),
        ):
            with self.subTest(mesh=model_name):
                self.check_visual(source[model_name], file, face_count, counts, shared)

    def check_visual(self, mesh, file, face_count, counts, shared):
        visual = (mole.FOLDER / file).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'MOLEVIS1', 3 * face_count, 56))
        corners = iter(struct.iter_unpack('<12fII', visual[16:]))
        scale = float(mole.SCALE)
        use = [0, 0, 0]
        for face, mat_idx, mapping in zip(mesh['faces'], mesh['faceMaterials'], mesh['faceTextures'], strict=True):
            a, b, c = [tuple(v * scale for v in mesh['vertices'][i]) for i in face]
            ab = [q - p for p, q in zip(a, b, strict=True)]
            ac = [q - p for p, q in zip(a, c, strict=True)]
            cross = [ab[j] * ac[k] - ab[k] * ac[j] for j, k in ((1, 2), (2, 0), (0, 1))]
            normal = [v / hypot(*cross) for v in cross]
            mat = mesh['materials'][mat_idx]
            tint = [(mat['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
            for point, uv in zip((a, b, c), mapping['uv'], strict=True):
                row = next(corners)
                expected = struct.unpack('<12f', struct.pack('<12f', *point, *normal, *tint, *uv))
                self.assertEqual(row[:12], expected)
                self.assertEqual(row[12:], (mapping['texture'], int(mat['emissive'])))
            use[mapping['texture']] += 1
        self.assertEqual(use, counts)
        self.assertIsNone(next(corners, None))
        self.assertEqual({Path(path).stem for path in shared},
                         {asset['asset'] for asset in mesh['textures']})
        for name, expected in shared.items():
            raw = (proof.ROOT / name).read_bytes()
            self.assertEqual(len(raw), expected['bytes'])
            self.assertEqual(hashlib.sha256(raw).hexdigest(), expected['sha256'])

    def test_cli_refuses_changed_streams_and_wrong_original_hull(self):
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp) / 'sh016'
            shutil.copytree(mole.FOLDER, folder)
            args = [mole.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(mole.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(mole.__file__, run_name='__main__')
                (folder / 'source-piece-membership.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed SH016 encoding'):
                    runpy.run_path(mole.__file__, run_name='__main__')
            shutil.copyfile(mole.ROOT / 'assets/fleet/sh010/resource.toml', folder / 'resource.toml')
            with self.assertRaisesRegex(ValueError, 'changed SH016 original'):
                mole.prepare(folder)

    def test_source_partition_rejects_a_face_crossing_a_seam(self):
        with self.assertRaisesRegex(ValueError, 'triangle crosses a support seam'):
            mole.verify_face_coverage([(0, 1, 4)], [[0, 1, 2, 3], [2, 3, 4, 5]], 6)
        with self.assertRaisesRegex(ValueError, 'source vertex missing'):
            mole.verify_face_coverage([(0, 1, 2)], [[0, 1, 2, 3]], 5)

    def test_clearance_contact_and_opaque_joint_replay(self):
        result = json.loads((proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(json.loads(json.dumps(proof.verify())), result)
        self.assertEqual(list(result['clearanceMillimetres'].values()), [1909, 2, 2, 173, 173])
        self.assertEqual(result['alignedMillimetres'], 39731)
        self.assertEqual(result['offsetStartMillimetres'], 979)
        self.assertAlmostEqual(result['sourceLipWitness']['fraction'], .5, places=13)
        self.assertEqual([item['sourceFaceHitCountsAboveBelow'] for item in result['glazingJoints']],
                         [[[1, 1]] * 4] * 2)
        with contextlib.redirect_stdout(StringIO()) as captured:
            runpy.run_path(proof.__file__, run_name='__main__')
        self.assertEqual(json.loads(captured.getvalue()), result)

    def test_contact_certificates_refuse_near_misses(self):
        tri = [(0, 0, 0), (4, 0, 0), (0, 4, 0)]
        with self.assertRaisesRegex(ValueError, 'no strict source lip crossing'):
            proof.lip_intersection(tri, (0, 1, 0), 3, (1, -1))
        tri_xz = [[(0, 1, 0), (4, 1, 0), (0, 1, 4)]]
        with self.assertRaisesRegex(ValueError, 'parity ray hit a source edge'):
            proof.inside_y((0, 0, 0), tri_xz)
        with self.assertRaisesRegex(ValueError, 'left the source volume'):
            proof.inside_y((1, 0, 1), tri_xz)
        with self.assertRaisesRegex(ValueError, 'misses a separating plane'):
            proof.separating_planes([[(0, 0, 0), (2, 0, 0)]], [(1, 0, 0)], [(1, 0, 0)])

    def test_air_fit_contact_and_numerical_helper_pins(self):
        criteria = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        replay = criteria['independentReplay']
        for name, expected in (replay['tools'] | replay['fixtures']).items():
            self.assertEqual(hashlib.sha256((proof.ROOT / name).read_bytes()).hexdigest(), expected)
        for name, expected in criteria['air']['reports'].items():
            self.assertEqual(hashlib.sha256((proof.INTEGRATION / name).read_bytes()).hexdigest(), expected)
        summary = json.loads((proof.INTEGRATION / 'SURFACE-FIT.json').read_bytes())['summary']
        for kind, low, upper in (('detailed', 1102, 1460), ('native', 2102, 2175)):
            self.assertEqual(summary[kind]['largestSample']['certifiedLowerMillimetres'], low)
            self.assertEqual(summary[kind]['fullUpperMillimetres'], upper)
