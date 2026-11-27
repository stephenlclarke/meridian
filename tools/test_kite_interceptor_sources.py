"""Check immutable SH031 meshes, material inputs and independent controls."""
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

import prepare_kite_interceptor_geometry as kite_interceptor
import verify_kite_interceptor_geometry as proof
import prepare_kite_interceptor_studio as studio


class TestKiteInterceptorSources(unittest.TestCase):
    def test_packaging_reproduces_both_metre_meshes(self):
        for name, data in kite_interceptor.prepare(kite_interceptor.ASSETS).items():
            with self.subTest(name=name):
                self.assertEqual((kite_interceptor.ASSETS / name).read_bytes(), data)
        manifest = json.loads((kite_interceptor.ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())
        self.assertEqual(manifest['metresPerSourceUnit'], 1)
        self.assertEqual((manifest['native']['faces'], manifest['native']['faceLimit']), (160, 512))

    def test_each_original_normal_material_uv_and_shared_map(self):
        source = tomllib.loads((kite_interceptor.ASSETS / 'resource.toml').read_text())
        shared = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())['originalTextures']
        for model_name, file, face_count, counts in (
            ('design', 'visual.bin', 3108, [912, 280, 700, 1216]),
            ('nativeDesign', 'native-visual.bin', 160, [96, 48, 0, 16]),
        ):
            with self.subTest(mesh=model_name):
                self.check_visual(source[model_name], file, face_count, counts, shared)

    def check_visual(self, mesh, file, face_count, counts, shared):
        visual = (kite_interceptor.ASSETS / file).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'KITEVIZ1', 3 * face_count, 56))
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
            folder = Path(temp) / 'sh031'
            shutil.copytree(kite_interceptor.ASSETS, folder)
            args = [kite_interceptor.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(kite_interceptor.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(kite_interceptor.__file__, run_name='__main__')
                (folder / 'native-visual.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed SH031 encoding'):
                    runpy.run_path(kite_interceptor.__file__, run_name='__main__')
            shutil.copyfile(kite_interceptor.ROOT / 'assets/fleet/sh010/resource.toml', folder / 'resource.toml')
            with self.assertRaisesRegex(ValueError, 'changed SH031 original'):
                kite_interceptor.prepare(folder)

    def test_legacy_activation_and_physical_contract_pin_the_metre_native_source(self):
        evidence = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        activation, physical = evidence['legacyMetreAuthority']['files']
        a, p = [(proof.ROOT / row['artifact']).read_bytes() for row in (activation, physical)]
        for item, raw in zip((activation, physical), (a, p), strict=True):
            self.assertEqual(hashlib.sha256(raw).hexdigest(), item['sha256'])
        self.assertIn(b'static let id = "SH031"', a)
        self.assertIn(b'physical.metresPerSourceUnit == 1', a)
        self.assertIn(kite_interceptor.PINS['contract.toml'].encode(), a)
        section = p.split(b'private static func calibrationCase9(', 1)[1].split(b'@inline(never)', 1)[0]
        self.assertIn(kite_interceptor.NATIVE_PATH.encode(), section)
        self.assertIn(kite_interceptor.SOURCE_SHA.encode(), section)
        self.assertIn(b'metresPerSourceUnit = 1', section)

    def test_opposed_channels_structural_joints_and_station_controls_replay(self):
        result = json.loads((proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(json.loads(json.dumps(proof.verify())), result)
        self.assertEqual(list(result['clearanceMillimetres'].values()), [979, 448, 548, 979, 448, 548])
        self.assertEqual(result['alignedMillimetres'], 44278)
        self.assertEqual(result['offsetStartMillimetres'], 979)
        self.assertEqual(result['sourceLipWitness']['fraction'], .5)
        self.assertEqual(len(result['originalJoints']), 5)
        for row in result['originalJoints']:
            self.assertEqual(row['sourceFaceHitCountsAboveBelow'], [[1, 1]] * 4)
        with contextlib.redirect_stdout(StringIO()) as out:
            runpy.run_path(proof.__file__, run_name='__main__')
        self.assertEqual(json.loads(out.getvalue()), result)

    def test_hull_studio_retains_source_ordered_meshes_and_witness_probes(self):
        source = tomllib.loads((kite_interceptor.ASSETS / 'resource.toml').read_text())
        definition = json.loads(studio.OUTPUT.read_bytes())
        criteria = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        self.assertEqual(definition, studio.definition())
        self.assertEqual(hashlib.sha256(studio.OUTPUT.read_bytes()).hexdigest(),
                         criteria['hullStudio']['sha256'])
        for field, original, membership in (
            ('detail', source['design'], (0, kite_interceptor.DETAIL[0])),
            ('native', source['nativeDesign'], (kite_interceptor.DETAIL[0], None)),
        ):
            with self.subTest(mesh=field):
                mesh = definition[field]
                self.assertEqual(mesh['vertices'], original['vertices'])
                self.assertEqual(mesh['triangles'], original['faces'])
                groups = (kite_interceptor.ASSETS / 'source-piece-membership.bin').read_bytes()
                self.assertEqual(mesh['groups'], list(groups[slice(*membership)]))
                self.assertEqual(set(i for face in mesh['triangles'] for i in face),
                                 set(range(len(mesh['vertices']))))
        self.assertEqual(definition['id'], 'SH031')
        self.assertIsNone(definition['engineering'])
        self.assertEqual(definition['flight']['maxSpeed'], 55.0)
        self.assertEqual(definition['faceMaterials'], source['design']['faceMaterials'])
        clearance = proof.verify()['clearanceMillimetres']
        fixtures = json.loads((proof.INTEGRATION / 'FIXTURES.json').read_bytes())
        self.assertEqual(len(definition['clearances']), 6)
        for route, fixture in zip(definition['clearances'], fixtures['routes'], strict=True):
            self.assertEqual(route['name'], fixture['name'])
            self.assertEqual(route['radiusMetres'], .10)
            self.assertGreater(clearance[route['name']], 100)
            for start, end, travel in zip(route['start'], route['end'], fixture['travel'], strict=True):
                self.assertAlmostEqual(start - end, travel)
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'result.json'
            args = [studio.__file__, '--output', str(output)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(studio.__file__, run_name='__main__')
            self.assertEqual(output.read_bytes(), studio.OUTPUT.read_bytes())
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(studio.__file__, run_name='__main__')
                output.write_bytes(b'{}')
                with self.assertRaisesRegex(ValueError, 'changed Kite Interceptor Hull Studio definition'):
                    runpy.run_path(studio.__file__, run_name='__main__')

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
        for kind, low, upper in (('detailed', 210, 321), ('native', 1229, 1438)):
            self.assertEqual(summary[kind]['largestSample']['certifiedLowerMillimetres'], low)
            self.assertEqual(summary[kind]['fullUpperMillimetres'], upper)

        delta = json.loads((proof.INTEGRATION / 'ORIGINAL-DIFFERENCE.json').read_bytes())
        self.assertEqual(delta['detailed']['strictLowerMillimetres'], 88)
        self.assertEqual(delta['native']['strictLowerMillimetres'], 1405)
