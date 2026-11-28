"""Check immutable SH027 meshes, material inputs and independent controls."""
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

import prepare_ibex_geometry as ibex
import verify_ibex_geometry as proof


class TestIbexSources(unittest.TestCase):
    def test_studio_binds_complete_original_meshes_materials_and_trench_routes(self):
        import prepare_ibex_studio as studio
        package = studio.definition()
        self.assertEqual(package, json.loads(studio.OUTPUT.read_bytes()))
        detail, native = ibex.authenticate(ibex.ASSETS)
        for key, mesh in (('detail', detail), ('native', native)):
            self.assertEqual(package[key]['vertices'], mesh['vertices'])
            self.assertEqual(package[key]['triangles'], mesh['faces'])
        self.assertEqual(package['faceMaterials'], detail['faceMaterials'])
        self.assertEqual(package['nativeFaceMaterials'], native['faceMaterials'])
        textures = [t['asset'] for t in detail['textures']]
        self.assertEqual(package['faceTextures'],
                         [{'uv': row['uv'], 'texture': textures[row['texture']] if row['texture'] >= 0 else None}
                          for row in detail['faceTextures']])
        self.assertIsNone(package['engineering'])
        fixtures = json.loads((proof.INTEGRATION / 'FIXTURES.json').read_bytes())['routes']
        for route, fixture in zip(package['clearances'], fixtures, strict=True):
            self.assertEqual(route['name'], fixture['name'])
            self.assertEqual(route['radiusMetres'], .1)
            for start, end, travel in zip(route['start'], route['end'], fixture['travel'], strict=True):
                self.assertAlmostEqual(start - end, travel)

    def test_packaging_reproduces_both_metre_meshes(self):
        for name, data in ibex.prepare(ibex.ASSETS).items():
            with self.subTest(name=name):
                self.assertEqual((ibex.ASSETS / name).read_bytes(), data)
        manifest = json.loads((ibex.ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())
        self.assertEqual(manifest['metresPerSourceUnit'], 1)
        self.assertEqual((manifest['native']['faces'], manifest['native']['faceLimit']), (184, 512))

    def test_each_original_normal_material_uv_and_shared_map(self):
        source = tomllib.loads((ibex.ASSETS / 'resource.toml').read_text())
        shared = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())['originalTextures']
        for model_name, file, face_count, counts in (
            ('design', 'visual.bin', 3108, [912, 280, 700, 1216]),
            ('nativeDesign', 'native-visual.bin', 184, [96, 48, 24, 16]),
        ):
            with self.subTest(mesh=model_name):
                self.check_visual(source[model_name], file, face_count, counts, shared)

    def check_visual(self, mesh, file, face_count, counts, shared):
        visual = (ibex.ASSETS / file).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'IBEXVIZ1', 3 * face_count, 56))
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
            folder = Path(temp) / 'sh027'
            shutil.copytree(ibex.ASSETS, folder)
            args = [ibex.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(ibex.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(ibex.__file__, run_name='__main__')
                (folder / 'native-visual.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed SH027 encoding'):
                    runpy.run_path(ibex.__file__, run_name='__main__')
            shutil.copyfile(ibex.ROOT / 'assets/fleet/sh010/resource.toml', folder / 'resource.toml')
            with self.assertRaisesRegex(ValueError, 'changed SH027 original'):
                ibex.prepare(folder)

    def test_legacy_activation_and_physical_contract_pin_the_metre_native_source(self):
        evidence = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        activation, physical = evidence['legacyMetreAuthority']['files']
        a, p = [(proof.ROOT / row['artifact']).read_bytes() for row in (activation, physical)]
        for item, raw in zip((activation, physical), (a, p), strict=True):
            self.assertEqual(hashlib.sha256(raw).hexdigest(), item['sha256'])
        self.assertIn(b'static let id = "SH027"', a)
        self.assertIn(b'physical.metresPerSourceUnit == 1', a)
        self.assertIn(ibex.PINS['contract.toml'].encode(), a)
        section = p.split(b'private static func calibrationCase7(', 1)[1].split(b'@inline(never)', 1)[0]
        self.assertIn(ibex.NATIVE_PATH.encode(), section)
        self.assertIn(ibex.SOURCE_SHA.encode(), section)
        self.assertIn(b'metresPerSourceUnit = 1', section)

    def test_trench_bridge_cockpit_and_station_controls_replay(self):
        result = json.loads((proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(json.loads(json.dumps(proof.verify())), result)
        self.assertEqual(list(result['clearanceMillimetres'].values()), [1997, 1010, 225, 225])
        self.assertEqual(result['alignedMillimetres'], 42658)
        self.assertEqual(result['offsetStartMillimetres'], 979)
        self.assertEqual(result['sourceLipWitness']['fraction'], .5)
        self.assertEqual(len(result['originalJoints']), 3)
        for row in result['originalJoints']:
            self.assertEqual(row['sourceFaceHitCountsAboveBelow'], [[1, 1]] * 4)
        for area in result['bridgeMatingAreaSquareMetres']:
            self.assertAlmostEqual(area, 3.24)
        with contextlib.redirect_stdout(StringIO()) as out:
            runpy.run_path(proof.__file__, run_name='__main__')
        self.assertEqual(json.loads(out.getvalue()), result)

    def test_mating_rectangles_refuse_triangles_that_overlap_along_a_boundary_edge(self):
        a, b, c, d = (0, 0, 0), (0, 2, 0), (0, 0, 2), (0, 2, 2)
        with self.assertRaisesRegex(ValueError, 'rectangle diagonal'):
            proof.bridge_seam([(a, b, c), (a, b, d)], [(0, 2), (0, 2)])

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
        for kind, low, upper in (('detailed', 287, 300), ('native', 3201, 3206)):
            self.assertEqual(summary[kind]['largestSample']['certifiedLowerMillimetres'], low)
            self.assertEqual(summary[kind]['fullUpperMillimetres'], upper)
