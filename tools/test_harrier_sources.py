"""Check immutable SH026 meshes, material inputs and independent controls."""
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

import prepare_harrier_geometry as harrier
import verify_harrier_geometry as proof
import prepare_harrier_studio as studio


class TestHarrierSources(unittest.TestCase):
    def test_packaging_reproduces_both_metre_meshes(self):
        for name, data in harrier.prepare(harrier.ASSETS).items():
            with self.subTest(name=name):
                self.assertEqual((harrier.ASSETS / name).read_bytes(), data)
        manifest = json.loads((harrier.ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())
        self.assertEqual(manifest['metresPerSourceUnit'], 1)
        self.assertEqual((manifest['native']['faces'], manifest['native']['faceLimit']), (512, 512))

    def test_studio_reproduces_bare_meshes_and_per_face_appearance(self):
        package = studio.definition()
        self.assertEqual(package, json.loads(studio.OUTPUT.read_bytes()))
        detail, native = harrier.authenticate(harrier.ASSETS)
        for key, original, materials in (('detail', detail, 'faceMaterials'),
                                         ('native', native, 'nativeFaceMaterials')):
            self.assertEqual(package[key]['vertices'], original['vertices'])
            self.assertEqual(package[key]['triangles'], original['faces'])
            self.assertEqual(package[materials], original['faceMaterials'])
        for mapped, original in zip(package['faceTextures'], detail['faceTextures'], strict=True):
            slot = original['texture']
            self.assertEqual(mapped['texture'], detail['textures'][slot]['asset'] if slot >= 0 else None)
            self.assertEqual(mapped['uv'], original['uv'] if slot >= 0 else [[0., 0.]] * 3)
        probes = package['clearances']
        self.assertEqual(len(probes), 4)
        for probe in probes:
            self.assertEqual(probe['radiusMetres'], .05 if probe['name'].endswith('-empty-rail') else .10)
        self.assertIsNone(package['engineering'])

    def test_each_original_normal_material_uv_and_shared_map(self):
        source = tomllib.loads((harrier.ASSETS / 'resource.toml').read_text())
        shared = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())['originalTextures']
        for model_name, file, face_count, counts in (
            ('design', 'visual.bin', 8884, [1828, 404, 6452, 200]),
            ('nativeDesign', 'native-visual.bin', 512, [168, 144, 112, 88]),
        ):
            with self.subTest(mesh=model_name):
                self.check_visual(source[model_name], file, face_count, counts, shared)

    def check_visual(self, mesh, file, face_count, counts, shared):
        visual = (harrier.ASSETS / file).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'HARRVIZ1', 3 * face_count, 56))
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
            uv_input = mapping['uv']
            if slot == -1:
                self.assertEqual(uv_input, [])
                uv_input = [[0.0, 0.0]] * 3
            for coords, uv in zip((a, b, c), uv_input, strict=True):
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
            folder = Path(temp) / 'sh026'
            shutil.copytree(harrier.ASSETS, folder)
            args = [harrier.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(harrier.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(harrier.__file__, run_name='__main__')
                (folder / 'native-visual.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed SH026 encoding'):
                    runpy.run_path(harrier.__file__, run_name='__main__')
            shutil.copyfile(harrier.ROOT / 'assets/fleet/sh010/resource.toml', folder / 'resource.toml')
            with self.assertRaisesRegex(ValueError, 'changed SH026 original'):
                harrier.prepare(folder)

    def test_active_original_uses_the_bare_mesh_and_older_archive_is_retained(self):
        evidence = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        raw_cache, c01 = evidence['legacyRuntimeAuthority']['files']
        raw, native = [(proof.ROOT / row['artifact']).read_bytes() for row in (raw_cache, c01)]
        for row, data in zip((raw_cache, c01), (raw, native), strict=True):
            self.assertEqual(hashlib.sha256(data).hexdigest(), row['sha256'])
        self.assertIn(b'ship.id == "SH026"', raw)
        self.assertIn(b'return (ship.id, native.rawMesh)', raw)
        self.assertIn(b'ShipCatalogue.ship("SH026")', native)
        self.assertIn(b'let collision = native.rawMesh', native)
        archive = json.loads((harrier.ASSETS / 'pre-bare-native-export.json').read_bytes())
        self.assertEqual(len(archive['nativeDesign']['faces']), 2852)

    def test_rail_pod_cockpit_and_station_controls_replay(self):
        result = json.loads((proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(json.loads(json.dumps(proof.verify())), result)
        self.assertEqual(list(result['clearanceMillimetres'].values()), [1036, 99, 851, 99])
        self.assertEqual(result['alignedMillimetres'], 46493)
        self.assertEqual(result['offsetStartMillimetres'], 979)
        self.assertAlmostEqual(result['sourceLipWitness']['fraction'], .5, places=14)
        self.assertEqual(len(result['originalJoints']), 14)
        for joint in result['originalJoints']:
            self.assertEqual(joint['sourceFaceHitCountsAboveBelow'], [[1, 1]] * 4)
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
        for kind, low, upper in (('detailed', 154, 195), ('native', 1168, 1170)):
            self.assertEqual(summary[kind]['largestSample']['certifiedLowerMillimetres'], low)
            self.assertEqual(summary[kind]['fullUpperMillimetres'], upper)
