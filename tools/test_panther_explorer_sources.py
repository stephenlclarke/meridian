"""Check immutable SH032 meshes, material inputs and independent controls."""
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

import prepare_panther_explorer_geometry as panther_explorer
import verify_panther_explorer_geometry as proof
import prepare_panther_explorer_studio as studio


class TestPantherExplorerSources(unittest.TestCase):
    def test_packaging_reproduces_both_metre_meshes(self):
        for name, data in panther_explorer.prepare(panther_explorer.ASSETS).items():
            with self.subTest(name=name):
                self.assertEqual((panther_explorer.ASSETS / name).read_bytes(), data)
        manifest = json.loads((panther_explorer.ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())
        self.assertEqual(manifest['metresPerSourceUnit'], 1)
        self.assertEqual((manifest['native']['faces'], manifest['native']['faceLimit']), (352, 512))

    def test_each_original_normal_material_uv_and_shared_map(self):
        source = tomllib.loads((panther_explorer.ASSETS / 'resource.toml').read_text())
        shared = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())['originalTextures']
        for model_name, file, face_count, counts in (
            ('design', 'visual.bin', 5596, [1520, 1896, 940, 1240]),
            ('nativeDesign', 'native-visual.bin', 352, [160, 128, 24, 40]),
        ):
            with self.subTest(mesh=model_name):
                self.check_visual(source[model_name], file, face_count, counts, shared)

    def check_visual(self, mesh, file, face_count, counts, shared):
        visual = (panther_explorer.ASSETS / file).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'PNTXVIZ1', 3 * face_count, 56))
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
            folder = Path(temp) / 'sh032'
            shutil.copytree(panther_explorer.ASSETS, folder)
            args = [panther_explorer.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(panther_explorer.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(panther_explorer.__file__, run_name='__main__')
                (folder / 'native-visual.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed SH032 encoding'):
                    runpy.run_path(panther_explorer.__file__, run_name='__main__')
            shutil.copyfile(panther_explorer.ROOT / 'assets/fleet/sh010/resource.toml', folder / 'resource.toml')
            with self.assertRaisesRegex(ValueError, 'changed SH032 original'):
                panther_explorer.prepare(folder)

    def test_legacy_activation_and_physical_contract_pin_the_metre_native_source(self):
        evidence = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        activation, physical = evidence['legacyMetreAuthority']['files']
        a, p = [(proof.ROOT / row['artifact']).read_bytes() for row in (activation, physical)]
        for item, raw in zip((activation, physical), (a, p), strict=True):
            self.assertEqual(hashlib.sha256(raw).hexdigest(), item['sha256'])
        explorer = a.split(b'case "SH032":', 1)[1].split(b'case "SH033":', 1)[0]
        self.assertIn(b'name: "Panther Explorer", role: .leisure', explorer)
        self.assertIn(panther_explorer.PINS['contract.toml'].encode(), explorer)
        self.assertIn(panther_explorer.SOURCE_SHA.encode(), explorer)
        self.assertIn(b'batch-01-native-lod/', a)
        section = p.split(b'private static func calibrationCase10(', 1)[1].split(b'@inline(never)', 1)[0]
        self.assertIn(b'OriginalPantherFamilyActivation.registration(for: definition.id)', section)
        self.assertIn(b'model.sourcePath == approved.modelPath', section)
        self.assertIn(b'model.sourceSHA256 == approved.modelSHA256', section)
        self.assertIn(b'metresPerSourceUnit = 1', section)

    def test_opposed_channels_structural_joints_and_station_controls_replay(self):
        result = json.loads((proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(json.loads(json.dumps(proof.verify())), result)
        self.assertEqual(list(result['clearanceMillimetres'].values()), [328, 448, 558, 930, 304, 252, 213] * 2)
        self.assertEqual(result['alignedMillimetres'], 43218)
        self.assertEqual(result['offsetStartMillimetres'], 979)
        self.assertEqual(result['sourceLipWitness']['fraction'], .5)
        self.assertEqual(len(result['mergedArchBlockedPointsMetres']), 8)
        for left, right in zip(list(result['mergedArchBlockedPointsMetres'].values())[:4],
                               list(result['mergedArchBlockedPointsMetres'].values())[4:], strict=True):
            self.assertEqual(left, [-right[0], right[1], right[2]])
        self.assertEqual(len(result['originalJoints']), 10)
        for row in result['originalJoints']:
            self.assertEqual(row['sourceFaceHitCountsAboveBelow'], [[1, 1]] * 4)
        with contextlib.redirect_stdout(StringIO()) as out:
            runpy.run_path(proof.__file__, run_name='__main__')
        self.assertEqual(json.loads(out.getvalue()), result)

    def test_hull_studio_retains_source_ordered_meshes_and_witness_probes(self):
        source = tomllib.loads((panther_explorer.ASSETS / 'resource.toml').read_text())
        definition = json.loads(studio.OUTPUT.read_bytes())
        criteria = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        self.assertEqual(definition, json.loads(json.dumps(studio.definition())))
        self.assertEqual(hashlib.sha256(studio.OUTPUT.read_bytes()).hexdigest(),
                         criteria['hullStudio']['sha256'])
        split = panther_explorer.SUPPORT_VERTICES[0]
        packed_groups = (panther_explorer.ASSETS / 'source-piece-membership.bin').read_bytes()
        for field, original, membership, arch_starts, stride, cuts in (
            ('detail', source['design'], packed_groups[:split], (388, 1124), 12, (5, 14, 22)),
            ('native', source['nativeDesign'], packed_groups[split:], (36, 92), 4, (1, 2, 3)),
        ):
            with self.subTest(mesh=field):
                mesh = definition[field]
                original_count = len(original['vertices'])
                self.assertEqual(mesh['vertices'][:original_count], original['vertices'])
                self.assertEqual(len(mesh['triangles']), len(original['faces']))
                expected_duplicates = [
                    tuple(original['vertices'][base + cut * stride + i])
                    for base in arch_starts for cut in cuts for i in range(stride)
                ]
                self.assertCountEqual(map(tuple, mesh['vertices'][original_count:]), expected_duplicates)
                self.assertEqual(mesh['groups'], list(membership))
                for packed_face, original_face in zip(mesh['triangles'], original['faces'], strict=True):
                    self.assertEqual([mesh['vertices'][i] for i in packed_face],
                                     [original['vertices'][i] for i in original_face])
                    self.assertEqual(len({mesh['groups'][i] for i in packed_face}), 1)
                self.assertEqual({i for face in mesh['triangles'] for i in face},
                                 set(range(len(mesh['vertices']))))
        self.assertEqual(definition['id'], 'SH032')
        self.assertIsNone(definition['engineering'])
        self.assertEqual(definition['flight']['maxSpeed'], 47.0)
        self.assertEqual(definition['faceMaterials'], source['design']['faceMaterials'])
        clearance = proof.verify()['clearanceMillimetres']
        fixtures = json.loads((proof.INTEGRATION / 'FIXTURES.json').read_bytes())
        self.assertEqual(len(definition['clearances']), 14)
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
                with self.assertRaisesRegex(ValueError, 'changed Panther Explorer Hull Studio definition'):
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
        section = [(0, 0, 1), (4, 0, 1), (0, 4, 1)]
        with self.assertRaisesRegex(ValueError, 'expected distinct original arch vertices'):
            proof.blocked_merged_arch([(0, 0, 0)] * 2, section, (0, 0, 2))
        with self.assertRaisesRegex(ValueError, 'merged-arch witness lies outside'):
            proof.blocked_merged_arch([(0, 0, 0), (0, 1, 0)], section, (0, 0, 2))
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
        for kind, low, upper in (('detailed', 437, 1001), ('native', 2127, 2170)):
            self.assertEqual(summary[kind]['largestSample']['certifiedLowerMillimetres'], low)
            self.assertEqual(summary[kind]['fullUpperMillimetres'], upper)

        delta = json.loads((proof.INTEGRATION / 'ORIGINAL-DIFFERENCE.json').read_bytes())
        self.assertEqual(delta['detailed']['strictLowerMillimetres'], 88)
        self.assertEqual(delta['native']['strictLowerMillimetres'], 2109)

        comparison = criteria['archSupportSelection']
        path = proof.INTEGRATION / comparison['unsplitFit']
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), comparison['unsplitFitSha256'])
        unsplit = json.loads(path.read_bytes())['summary']
        self.assertEqual(unsplit['detailed']['fullUpperMillimetres'], 2428)
        self.assertEqual(unsplit['native']['fullUpperMillimetres'], 2567)
