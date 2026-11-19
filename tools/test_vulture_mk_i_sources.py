"""Check immutable SH037 meshes, material inputs and independent controls."""
import contextlib
import hashlib
from io import BytesIO, TextIOWrapper
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

import numpy as np

import measure_vulture_mk_i_air as measure
from vulture_mk_i_exact_fit import TriangleIndex, bound_fan, full_upper, grid_index
import prepare_vulture_mk_i_geometry as vulture_mk_i
import verify_vulture_mk_i_geometry as proof
import prepare_vulture_mk_i_studio as studio


class TestVultureMkISources(unittest.TestCase):
    def test_packaging_preserves_originals_and_certifies_separate_proxy(self):
        for name, data in vulture_mk_i.prepare(vulture_mk_i.ASSETS).items():
            with self.subTest(name=name):
                self.assertEqual((vulture_mk_i.ASSETS / name).read_bytes(), data)
        manifest = json.loads((vulture_mk_i.ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())
        self.assertEqual(manifest['metresPerSourceUnitExact'], '6000/611')
        self.assertEqual(manifest['originalNative']['faces'], 3016)
        self.assertEqual(manifest['studioNativeProxy']['faces'], 512)
        self.assertEqual(manifest['studioNativeProxy']['vertices'], 272)
        vertices = (vulture_mk_i.ASSETS / 'source.vertices').read_bytes()
        self.assertEqual(struct.unpack_from('<8sIII', vertices), (b'MSH037V1', 5132, 1812, 272))
        self.assertAlmostEqual(manifest['detail']['bounds'][1][2] - manifest['detail']['bounds'][0][2], 30)

    def test_each_original_visual_preserves_material_winding_and_no_texture(self):
        source = tomllib.loads((vulture_mk_i.ASSETS / 'resource.toml').read_text())
        for model_name, file, face_count in (
            ('design', 'visual.bin', 8272),
            ('nativeDesign', 'original-native-visual.bin', 3016),
        ):
            with self.subTest(mesh=model_name):
                self.check_visual(source[model_name], file, face_count)

    def check_visual(self, mesh, file, face_count):
        visual = (vulture_mk_i.ASSETS / file).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'VUL1VIZ1', 3 * face_count, 56))
        corners = iter(struct.iter_unpack('<12fII', visual[16:]))
        scale = 6000 / 611
        for face, material_id in zip(mesh['faces'], mesh['faceMaterials'], strict=True):
            a, b, c = [[raw * scale for raw in mesh['vertices'][i]] for i in face]
            ab = [q - p for p, q in zip(a, b, strict=True)]
            ac = [q - p for p, q in zip(a, c, strict=True)]
            cross = [ab[j] * ac[k] - ab[k] * ac[j] for j, k in ((1, 2), (2, 0), (0, 1))]
            normal = [v / hypot(*cross) for v in cross]
            material = mesh['materials'][material_id]
            tint = [(material['colour'] >> shift & 255) / 255 for shift in (24, 16, 8, 0)]
            for coords in (a, b, c):
                raw = next(corners)
                expected = struct.unpack('<12f', struct.pack('<12f', *coords, *normal, *tint, 0.0, 0.0))
                self.assertEqual(raw[:12], expected)
                self.assertEqual(raw[12:], (0xffff_ffff, int(material['emissive'])))
        self.assertIsNone(next(corners, None))

    def test_cli_refuses_changed_streams_and_wrong_original_hull(self):
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp) / 'sh037'
            shutil.copytree(vulture_mk_i.ASSETS, folder)
            args = [vulture_mk_i.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(vulture_mk_i.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(vulture_mk_i.__file__, run_name='__main__')
                (folder / 'original-native-visual.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed encoded SH037 file'):
                    runpy.run_path(vulture_mk_i.__file__, run_name='__main__')
            shutil.copyfile(vulture_mk_i.ROOT / 'assets/fleet/sh010/resource.toml', folder / 'resource.toml')
            with self.assertRaisesRegex(ValueError, 'changed SH037 original'):
                vulture_mk_i.prepare(folder)

    def test_sh037_engineering_binds_scale_and_the_original_native_archive(self):
        resource, engineering, sources = vulture_mk_i.source_models(vulture_mk_i.ASSETS)
        self.assertEqual(resource['name'], 'Vulture Mk I')
        self.assertEqual(engineering['metresPerSourceUnitExact'], '6000/611')
        self.assertEqual(engineering['targetDetailedLengthMetres'], 30.0)
        self.assertEqual(engineering['dryMassKilograms'], 51330)
        self.assertEqual(sources[1][0]['faces'], resource['nativeDesign']['faces'])

    def test_proxy_refuses_shrunken_and_open_surfaces(self):
        _, _, source = vulture_mk_i.source_models(vulture_mk_i.ASSETS)
        raw = (vulture_mk_i.ASSETS / vulture_mk_i.PROXY).read_bytes()
        data = json.loads(raw)
        data['vertices'] = [[v * .9 for v in point] if group == 0 else point
                            for point, group in zip(data['vertices'], data['groups'], strict=True)]
        with self.assertRaisesRegex(ValueError, 'strict original vertex containment'):
            vulture_mk_i.certify_proxy(data, source)
        data = json.loads(raw)
        data['triangles'].pop()
        with self.assertRaisesRegex(ValueError, 'not closed and consistently oriented'):
            vulture_mk_i.certify_proxy(data, source)

    def test_opposed_channels_structural_joints_and_station_controls_replay(self):
        result = json.loads((proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(json.loads(json.dumps(proof.verify())), result)
        self.assertEqual(list(result['clearanceMillimetres'].values()), [243, 180, 953, 243, 180, 953])
        self.assertEqual(result['alignedMillimetres'], 44572)
        self.assertEqual(result['offsetStartMillimetres'], 978)
        self.assertAlmostEqual(result['sourceLipWitness']['fraction'], .5)
        self.assertEqual(len(result['originalJoints']), 5)
        for row in result['originalGlazingBacking']:
            self.assertTrue(all(.225 <= v <= .264 for v in row['recessMetres']))
        for row in result['originalJoints']:
            self.assertEqual(row['sourceFaceHitCountsAboveBelow'], [[1, 1]] * 4)
        with TextIOWrapper(BytesIO(), encoding='ascii', newline='\r\n') as out:
            with contextlib.redirect_stdout(out):
                runpy.run_path(proof.__file__, run_name='__main__')
            self.assertEqual(out.buffer.getvalue(), (proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())

    def test_fit_and_difference_emit_utf8_lf_with_crlf_text_stdout(self):
        for options, generator, file in (
            ([], 'audit', 'DETERMINISTIC-SURFACE-FIT.json'),
            (['--difference'], 'original_difference', 'DETERMINISTIC-ORIGINAL-DIFFERENCE.json'),
        ):
            with self.subTest(file=file):
                expected = (proof.INTEGRATION / file).read_bytes()
                with TextIOWrapper(BytesIO(), encoding='ascii', newline='\r\n') as out:
                    with (
                        contextlib.redirect_stdout(out),
                        mock.patch.object(sys, 'argv', [measure.__file__, *options]),
                        mock.patch.object(measure, generator, return_value=json.loads(expected)) as produce,
                    ):
                        measure.main()
                    produce.assert_called_once_with()
                    self.assertEqual(out.buffer.getvalue(), expected)

    def test_hull_studio_retains_source_ordered_meshes_and_witness_probes(self):
        source = tomllib.loads((vulture_mk_i.ASSETS / 'resource.toml').read_text())
        definition = json.loads(studio.OUTPUT.read_bytes())
        criteria = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        self.assertEqual(definition, studio.definition())
        self.assertEqual(hashlib.sha256(studio.OUTPUT.read_bytes()).hexdigest(),
                         criteria['hullStudio']['sha256'])
        original = source['design']
        detail = definition['detail']
        self.assertEqual(detail['vertices'], original['vertices'])
        self.assertEqual(detail['triangles'], original['faces'])
        groups = (vulture_mk_i.ASSETS / 'source-piece-membership.bin').read_bytes()
        self.assertEqual(detail['groups'], list(groups[:5132]))
        proxy = json.loads((vulture_mk_i.ASSETS / vulture_mk_i.PROXY).read_bytes())
        for key in ('vertices', 'triangles', 'groups'):
            self.assertEqual(definition['native'][key], proxy[key])
        self.assertEqual(definition['id'], 'SH037')
        physics = json.loads((vulture_mk_i.ASSETS / 'engineering.json').read_bytes())
        self.assertEqual(definition['engineering']['massKg'], physics['dryMassKilograms'])
        self.assertEqual(definition['engineering']['centerOfMass'], physics['centreOfMassMetres'])
        self.assertEqual(sum(definition['engineering']['inertia'], []), physics['inertiaAboutCentreOfMassKgM2'])
        self.assertEqual(definition['flight']['maxSpeed'], 58.3)
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
                with self.assertRaisesRegex(ValueError, 'changed Vulture Mk I Studio definition'):
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

    def test_exact_source_ties_and_independent_sensor_bound(self):
        vertices, names, groups, meshes, faces = measure.load()
        original, scale, triangles = measure.fit.integer_source(vertices, meshes)
        (num, den), ties = TriangleIndex(triangles['detailed']).nearest(original[6038])
        self.assertEqual(ties, [3654, 3655, 3658, 3659, 3661])
        shared = set.intersection(*(set(triangles['detailed'][i]) for i in ties))
        self.assertEqual(len(shared), 1)
        offset = measure.fit.sub(original[6038], shared.pop())
        self.assertEqual(measure.fit.dot(offset, offset) * den, num)
        slot = groups[names.index('positive-sensor')]
        fan = faces[np.isin(faces[:, 0], slot)]
        bound_fan(vertices[slot], np.searchsorted(slot, fan))
        index = grid_index(meshes['detailed'][1])
        self.assertLessEqual(full_upper(index, vertices[fan], 905), 905)

    def test_fit_contact_and_numerical_helper_pins(self):
        criteria = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        replay = criteria['independentReplay']
        for name, expected in (replay['tools'] | replay['fixtures']).items():
            self.assertEqual(hashlib.sha256((proof.ROOT / name).read_bytes()).hexdigest(), expected)
        for name, expected in criteria['air']['reports'].items():
            self.assertEqual(hashlib.sha256((proof.INTEGRATION / name).read_bytes()).hexdigest(), expected)
        summary = json.loads((proof.INTEGRATION / 'DETERMINISTIC-SURFACE-FIT.json').read_bytes())['summary']
        for kind, low, upper in (('detailed', 931, 986), ('originalNative', 941, 1030)):
            self.assertEqual(summary[kind]['largestSample']['certifiedLowerMillimetres'], low)
            self.assertEqual(summary[kind]['fullUpperMillimetres'], upper)

        delta = json.loads((proof.INTEGRATION / 'DETERMINISTIC-ORIGINAL-DIFFERENCE.json').read_bytes())
        self.assertEqual(delta['detailed']['strictLowerMillimetres'], 146)
        self.assertEqual(delta['originalNative']['strictLowerMillimetres'], 323)
