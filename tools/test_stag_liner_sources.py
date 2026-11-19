"""Check immutable SH035 meshes, material inputs and independent controls."""
import contextlib
import hashlib
from io import StringIO
import json
from math import hypot
import os
from pathlib import Path
import runpy
import shutil
import stat
import struct
import sys
import tempfile
import tomllib
import unittest
from unittest import mock

import prepare_stag_liner_geometry as stag_liner
import verify_stag_liner_geometry as proof
import prepare_stag_liner_studio as studio


class TestStagLinerSources(unittest.TestCase):
    def test_packaging_reproduces_both_metre_meshes(self):
        for name, data in stag_liner.prepare(stag_liner.ASSETS).items():
            with self.subTest(name=name):
                self.assertEqual((stag_liner.ASSETS / name).read_bytes(), data)
        manifest = json.loads((stag_liner.ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())
        self.assertEqual(manifest['metresPerSourceUnit'], 1)
        self.assertEqual((manifest['native']['faces'], manifest['native']['faceLimit']), (216, 512))

    def test_each_original_normal_material_uv_and_shared_map(self):
        source = tomllib.loads((stag_liner.ASSETS / 'resource.toml').read_text())
        shared = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())['originalTextures']
        for model_name, file, face_count, counts in (
            ('design', 'visual.bin', 3492, [1216, 256, 780, 1240]),
            ('nativeDesign', 'native-visual.bin', 216, [128, 24, 24, 40]),
        ):
            with self.subTest(mesh=model_name):
                self.check_visual(source[model_name], file, face_count, counts, shared)

    def check_visual(self, mesh, file, face_count, counts, shared):
        visual = (stag_liner.ASSETS / file).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'STAGVIZ1', 3 * face_count, 56))
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
            folder = Path(temp) / 'sh035'
            shutil.copytree(stag_liner.ASSETS, folder)
            args = [stag_liner.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(stag_liner.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(stag_liner.__file__, run_name='__main__')
                (folder / 'native-visual.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed SH035 encoding'):
                    runpy.run_path(stag_liner.__file__, run_name='__main__')
            shutil.copyfile(stag_liner.ROOT / 'assets/fleet/sh010/resource.toml', folder / 'resource.toml')
            with self.assertRaisesRegex(ValueError, 'changed SH035 original'):
                stag_liner.prepare(folder)

    def test_legacy_activation_and_physical_contract_pin_the_metre_native_source(self):
        evidence = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        activation, physical = evidence['legacyMetreAuthority']['files']
        a, p = [(proof.ROOT / row['artifact']).read_bytes() for row in (activation, physical)]
        for item, raw in zip((activation, physical), (a, p), strict=True):
            self.assertEqual(hashlib.sha256(raw).hexdigest(), item['sha256'])
        self.assertIn(b'static let id = "SH035"', a)
        self.assertIn(b'physical.metresPerSourceUnit == 1', a)
        self.assertIn(stag_liner.PINS['contract.toml'].encode(), a)
        section = p.split(b'private static func calibrationCase11(', 1)[1].split(b'@inline(never)', 1)[0]
        self.assertIn(stag_liner.NATIVE_PATH.encode(), section)
        self.assertIn(stag_liner.SOURCE_SHA.encode(), section)
        self.assertIn(b'metresPerSourceUnit = 1', section)

    def test_opposed_channels_structural_joints_and_station_controls_replay(self):
        result = json.loads((proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(json.loads(json.dumps(proof.verify())), result)
        self.assertEqual(list(result['clearanceMillimetres'].values()), [1440, 834, 748, 1440, 834, 748, 918])
        self.assertEqual(result['alignedMillimetres'], 33278)
        self.assertEqual(result['offsetStartMillimetres'], 979)
        self.assertEqual(result['sourceLipWitness']['fraction'], .5)
        self.assertEqual(len(result['originalJoints']), 8)
        for row in result['originalJoints']:
            self.assertEqual(row['sourceFaceHitCountsAboveBelow'], [[1, 1]] * 4)
        with contextlib.redirect_stdout(StringIO()) as out:
            runpy.run_path(proof.__file__, run_name='__main__')
        self.assertEqual(json.loads(out.getvalue()), result)

    def test_hull_studio_retains_source_ordered_meshes_and_witness_probes(self):
        source = tomllib.loads((stag_liner.ASSETS / 'resource.toml').read_text())
        definition = json.loads(studio.OUTPUT.read_bytes())
        criteria = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        self.assertEqual(definition, studio.definition())
        self.assertEqual(hashlib.sha256(studio.OUTPUT.read_bytes()).hexdigest(),
                         criteria['hullStudio']['sha256'])
        self.assertRegex(criteria['hullStudio']['installedExport']['testedConfigurationGitRevision'],
                         r'^[0-9a-f]{40}$')
        for field, original, membership in (
            ('detail', source['design'], (0, stag_liner.DETAIL[0])),
            ('native', source['nativeDesign'], (stag_liner.DETAIL[0], None)),
        ):
            with self.subTest(mesh=field):
                mesh = definition[field]
                self.assertEqual(mesh['vertices'], original['vertices'])
                self.assertEqual(mesh['triangles'], original['faces'])
                groups = (stag_liner.ASSETS / 'source-piece-membership.bin').read_bytes()
                self.assertEqual(mesh['groups'], list(groups[slice(*membership)]))
                self.assertEqual(set(i for face in mesh['triangles'] for i in face),
                                 set(range(len(mesh['vertices']))))
        self.assertEqual(definition['id'], 'SH035')
        self.assertIsNone(definition['engineering'])
        self.assertEqual(definition['flight']['maxSpeed'], 35.0)
        self.assertEqual(definition['faceMaterials'], source['design']['faceMaterials'])
        clearance = proof.verify()['clearanceMillimetres']
        fixtures = json.loads((proof.INTEGRATION / 'FIXTURES.json').read_bytes())
        self.assertEqual(len(definition['clearances']), 7)
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
                with self.assertRaisesRegex(ValueError, 'changed Stag Liner Hull Studio definition'):
                    runpy.run_path(studio.__file__, run_name='__main__')

    def test_studio_output_refuses_invalid_paths_before_generating_or_reading(self):
        with tempfile.TemporaryDirectory() as temp:
            scratch = Path(temp)
            directory = scratch / 'directory.json'
            directory.mkdir()
            with mock.patch.object(studio, 'definition', side_effect=AssertionError('generated before validation')):
                for selected, reason in (
                    (scratch / '..' / 'result.json', 'Invalid'),
                    (scratch / 'result.txt', 'Invalid'),
                    (proof.INTEGRATION / 'CRITERIA.json', 'temporary JSON file'),
                    (scratch / 'missing' / 'result.json', 'directory is missing'),
                    (directory, 'regular file'),
                ):
                    for mode in ([], ['--check']):
                        with self.subTest(path=selected, mode=mode), self.assertRaisesRegex(ValueError, reason):
                            studio.main(['--output', str(selected), *mode])
        self.assertEqual(studio.checked_output(studio.OUTPUT), studio.OUTPUT)

    def test_studio_output_refuses_file_and_directory_redirects(self):
        with tempfile.TemporaryDirectory() as temp:
            scratch = Path(temp)
            target = scratch / 'target.json'
            target.write_bytes(b'keep')
            file_link = scratch / 'file.json'
            directory_link = scratch / 'redirect'
            try:
                file_link.symlink_to(target)
                directory_link.symlink_to(scratch, target_is_directory=True)
            except OSError as error:
                self.skipTest(f'symbolic link unavailable: {error}')
            for selected, reason in ((file_link, 'regular file'), (directory_link / 'target.json', 'redirects')):
                for mode in ([], ['--check']):
                    with self.subTest(path=selected, mode=mode), self.assertRaisesRegex(ValueError, reason):
                        studio.main(['--output', str(selected), *mode])
            self.assertEqual(target.read_bytes(), b'keep')

    def test_studio_generation_replaces_hard_link_without_changing_other_name(self):
        with tempfile.TemporaryDirectory() as temp:
            scratch = Path(temp)
            original = scratch / 'original.json'
            output = scratch / 'output.json'
            original.write_bytes(b'keep')
            if os.name != 'nt':
                original.chmod(0o640)
            try:
                os.link(original, output)
            except OSError as error:
                self.skipTest(f'hard link unavailable: {error}')
            mode = stat.S_IMODE(original.stat().st_mode)
            args = ['--output', str(output)]
            with mock.patch.object(studio, 'definition', return_value={'id': 'SH035'}):
                with self.assertRaisesRegex(ValueError, 'changed Stag Liner Hull Studio definition'):
                    studio.main(args + ['--check'])
                self.assertTrue(output.samefile(original))
                self.assertEqual(original.read_bytes(), b'keep')
                studio.main(args)
                self.assertFalse(output.samefile(original))
                self.assertEqual(original.read_bytes(), b'keep')
                self.assertEqual(output.read_bytes(), b'{"id":"SH035"}\n')
                self.assertEqual(stat.S_IMODE(output.stat().st_mode), mode)
                with mock.patch.object(studio, 'write_definition', side_effect=AssertionError('check wrote output')):
                    studio.main(args + ['--check'])
            self.assertEqual({entry.name for entry in scratch.iterdir()}, {'original.json', 'output.json'})

    def test_studio_atomic_write_cleans_up_after_failure_and_keeps_destination(self):
        with tempfile.TemporaryDirectory() as temp:
            scratch = Path(temp)
            output = scratch / 'output.json'
            output.write_bytes(b'keep')
            for error_at in ('fsync', 'replace'):
                owner = os if error_at == 'fsync' else Path
                with (self.subTest(error_at=error_at),
                      mock.patch.object(owner, error_at, side_effect=OSError('injected failure')),
                      self.assertRaisesRegex(OSError, 'injected failure')):
                    studio.write_definition(output, b'replacement')
                self.assertEqual(output.read_bytes(), b'keep')
                self.assertEqual(list(scratch.iterdir()), [output])
            new_output = scratch / 'new.json'
            studio.write_definition(new_output, b'new')
            self.assertEqual(new_output.read_bytes(), b'new')
            if os.name != 'nt':
                self.assertEqual(stat.S_IMODE(new_output.stat().st_mode), 0o600)

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
        for kind, low, upper in (('detailed', 685, 705), ('native', 3035, 3038)):
            self.assertEqual(summary[kind]['largestSample']['certifiedLowerMillimetres'], low)
            self.assertEqual(summary[kind]['fullUpperMillimetres'], upper)

        delta = json.loads((proof.INTEGRATION / 'ORIGINAL-DIFFERENCE.json').read_bytes())
        self.assertEqual(delta['detailed']['strictLowerMillimetres'], 89)
        self.assertEqual(delta['native']['strictLowerMillimetres'], 3016)
