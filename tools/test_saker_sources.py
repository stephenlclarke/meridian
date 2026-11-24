"""Check immutable SH042 meshes, material inputs and independent controls."""
import contextlib
import hashlib
from io import BytesIO, TextIOWrapper
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

import atomic_asset_output as atomic
import prepare_saker_geometry as saker
import verify_saker_geometry as proof
import prepare_saker_studio as studio


class TestSakerSources(unittest.TestCase):
    def test_packaging_reproduces_both_metre_meshes(self):
        for name, data in saker.prepare(saker.ASSETS).items():
            with self.subTest(name=name):
                self.assertEqual((saker.ASSETS / name).read_bytes(), data)
        manifest = json.loads((saker.ASSETS / 'SOURCE-GEOMETRY.json').read_bytes())
        self.assertEqual(manifest['metresPerSourceUnit'], 1)
        self.assertEqual((manifest['native']['faces'], manifest['native']['faceLimit']), (220, 512))

    def test_each_original_normal_material_uv_and_shared_map(self):
        source = tomllib.loads((saker.ASSETS / 'resource.toml').read_text())
        shared = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())['originalTextures']
        for model_name, file, face_count, counts in (
            ('design', 'visual.bin', 3864, [304, 1296, 920, 1344]),
            ('nativeDesign', 'native-visual.bin', 220, [32, 132, 36, 20]),
        ):
            with self.subTest(mesh=model_name):
                self.check_visual(source[model_name], file, face_count, counts, shared)

    def check_visual(self, mesh, file, face_count, counts, shared):
        visual = (saker.ASSETS / file).read_bytes()
        self.assertEqual(struct.unpack_from('<8sII', visual), (b'SAKRVIZ1', 3 * face_count, 56))
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
            folder = Path(temp) / 'sh042'
            shutil.copytree(saker.ASSETS, folder)
            args = [saker.__file__, '--assets', str(folder)]
            with mock.patch.object(sys, 'argv', args):
                runpy.run_path(saker.__file__, run_name='__main__')
            with mock.patch.object(sys, 'argv', args + ['--check']):
                runpy.run_path(saker.__file__, run_name='__main__')
                (folder / 'native-visual.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'changed SH042 encoding'):
                    runpy.run_path(saker.__file__, run_name='__main__')
            shutil.copyfile(saker.ROOT / 'assets/fleet/sh010/resource.toml', folder / 'resource.toml')
            with self.assertRaisesRegex(ValueError, 'changed SH042 original'):
                saker.prepare(folder)

    def test_frozen_original_files_match_sha256_and_git_blob_identity(self):
        frozen = json.loads((proof.INTEGRATION / 'FROZEN-SOURCES.json').read_bytes())
        self.assertEqual(frozen['revision'], saker.LEGACY)
        self.assertEqual(frozen['tree'], '68fec574df12ddf74acb14747e0dd6839f478642')
        self.assertEqual(len(frozen['paths']), 11)
        for name, row in frozen['paths'].items():
            with self.subTest(source=row['path']):
                raw = (proof.ROOT / name).read_bytes()
                self.assertEqual(len(raw), row['bytes'])
                self.assertEqual(hashlib.sha256(raw).hexdigest(), row['sha256'])
                self.assertEqual(hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest(), row['blob'])

    def test_opposed_channels_structural_joints_and_station_controls_replay(self):
        result = json.loads((proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())
        self.assertEqual(json.loads(json.dumps(proof.verify())), result)
        self.assertEqual(list(result['clearanceMillimetres'].values()), [813, 499, 499, 2248, 2725, 2725, 479])
        self.assertEqual(result['alignedMillimetres'], 41772)
        self.assertEqual(result['offsetStartMillimetres'], 979)
        self.assertEqual(result['sourceLipWitness']['fraction'], .5)
        self.assertEqual(len(result['originalJoints']), 7)
        for row in result['originalJoints']:
            for above, below in row['sourceFaceHitCountsAboveBelow']:
                self.assertEqual((above % 2, below % 2), (1, 1))
        raw = BytesIO()
        with TextIOWrapper(raw, encoding='utf-8', newline='\r\n') as out:
            with contextlib.redirect_stdout(out):
                runpy.run_path(proof.__file__, run_name='__main__')
            self.assertEqual(out.buffer.getvalue(), (proof.INTEGRATION / 'CONTACT-RESULT.json').read_bytes())

    def test_hull_studio_retains_source_ordered_meshes_and_witness_probes(self):
        source = tomllib.loads((saker.ASSETS / 'resource.toml').read_text())
        definition = json.loads(studio.OUTPUT.read_bytes())
        criteria = json.loads((proof.INTEGRATION / 'CRITERIA.json').read_bytes())
        self.assertEqual(definition, studio.definition())
        self.assertEqual(hashlib.sha256(studio.OUTPUT.read_bytes()).hexdigest(),
                         criteria['hullStudio']['sha256'])
        for field, original, membership in (
            ('detail', source['design'], (0, saker.DETAIL[0])),
            ('native', source['nativeDesign'], (saker.DETAIL[0], None)),
        ):
            with self.subTest(mesh=field):
                mesh = definition[field]
                self.assertEqual(mesh['vertices'], original['vertices'])
                self.assertEqual(mesh['triangles'], original['faces'])
                groups = (saker.ASSETS / 'source-piece-membership.bin').read_bytes()
                self.assertEqual(mesh['groups'], list(groups[slice(*membership)]))
                self.assertEqual(set(i for face in mesh['triangles'] for i in face),
                                 set(range(len(mesh['vertices']))))
        self.assertEqual(definition['id'], 'SH042')
        finite = json.loads((saker.ASSETS / 'finite-dry-source.json').read_bytes())
        original = json.loads((saker.ASSETS / 'engineering.json').read_bytes())
        reference = finite['emptyDryReference']
        engineering = definition['engineering']
        self.assertEqual(definition['provenance']['engineeringReference'], reference['id'])
        self.assertEqual(engineering['massKg'], original['dryMassKilograms'])
        self.assertEqual(engineering['massKg'], reference['massKg'])
        self.assertEqual(engineering['centerOfMass'], reference['centreMetres'])
        self.assertNotEqual(engineering['centerOfMass'], original['centreOfMassMetres'])
        self.assertEqual(sum(engineering['inertia'], []), reference['inertiaAboutCentreOfMassKgM2'])
        self.assertEqual(engineering['inertia'], list(map(list, zip(*engineering['inertia'], strict=True))))
        self.assertTrue(finite['emptyStateOnly'])
        self.assertFalse(finite['operatingAuthorityGranted'])
        self.assertEqual(definition['flight']['maxSpeed'], 51.0)
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
                with self.assertRaisesRegex(ValueError, 'changed Saker Hull Studio definition'):
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
            with mock.patch.object(studio, 'definition', return_value={'id': 'SH042'}):
                with self.assertRaisesRegex(ValueError, 'changed Saker Hull Studio definition'):
                    studio.main(args + ['--check'])
                self.assertTrue(output.samefile(original))
                self.assertEqual(original.read_bytes(), b'keep')
                studio.main(args)
                self.assertFalse(output.samefile(original))
                self.assertEqual(original.read_bytes(), b'keep')
                self.assertEqual(output.read_bytes(), b'{"id":"SH042"}\n')
                self.assertEqual(stat.S_IMODE(output.stat().st_mode), mode)
                with mock.patch.object(studio, 'replace_output', side_effect=AssertionError('check wrote output')):
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
                    atomic.replace_output(output, b'replacement')
                self.assertEqual(output.read_bytes(), b'keep')
                self.assertEqual(list(scratch.iterdir()), [output])
            new_output = scratch / 'new.json'
            atomic.replace_output(new_output, b'new')
            self.assertEqual(new_output.read_bytes(), b'new')
            if os.name != 'nt':
                self.assertEqual(stat.S_IMODE(new_output.stat().st_mode), 0o600)

    def test_geometry_checks_every_fixed_destination_before_writing(self):
        with tempfile.TemporaryDirectory() as temp:
            scratch = Path(temp)
            folder = scratch / 'sh042'
            shutil.copytree(saker.ASSETS, folder)
            first = folder / 'source.vertices'
            first.write_bytes(b'prior geometry')
            manifest = folder / 'SOURCE-GEOMETRY.json'
            manifest.unlink()
            target = scratch / 'outside.json'
            target.write_bytes(b'keep')
            alias = scratch / 'redirect'
            try:
                manifest.symlink_to(target)
                alias.symlink_to(folder, target_is_directory=True)
            except OSError as error:
                self.skipTest(f'symbolic link unavailable: {error}')
            for mode in ([], ['--check']):
                with self.assertRaisesRegex(ValueError, 'regular file without redirects'):
                    saker.main(['--assets', str(folder), *mode])
                with self.assertRaisesRegex(ValueError, 'directory is missing or redirects'):
                    saker.main(['--assets', str(alias), *mode])
            self.assertEqual(first.read_bytes(), b'prior geometry')
            self.assertEqual(target.read_bytes(), b'keep')

    def test_geometry_replaces_hard_link_without_writing_its_source(self):
        with tempfile.TemporaryDirectory() as temp:
            scratch = Path(temp)
            folder = scratch / 'sh042'
            shutil.copytree(saker.ASSETS, folder)
            original = folder / 'resource.toml'
            before = original.read_bytes()
            output = folder / 'source.vertices'
            output.unlink()
            try:
                os.link(original, output)
            except OSError as error:
                self.skipTest(f'hard link unavailable: {error}')
            args = ['--assets', str(folder)]
            with self.assertRaisesRegex(ValueError, 'changed SH042 encoding'):
                saker.main(args + ['--check'])
            self.assertTrue(output.samefile(original))
            with (mock.patch.object(Path, 'replace', side_effect=OSError('injected failure')),
                  self.assertRaisesRegex(OSError, 'injected failure')):
                saker.main(args)
            self.assertTrue(output.samefile(original))
            self.assertFalse(list(folder.glob('.asset-output-*')))
            saker.main(args)
            self.assertFalse(output.samefile(original))
            self.assertEqual(original.read_bytes(), before)
            self.assertEqual(output.read_bytes(), (saker.ASSETS / output.name).read_bytes())
            with mock.patch.object(saker, 'replace_output', side_effect=AssertionError('check wrote output')):
                saker.main(args + ['--check'])

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
        fit = json.loads((proof.INTEGRATION / 'SURFACE-FIT.json').read_bytes())
        self.assertEqual(fit['fullUpperMillimetres'], {'detailed': 448, 'native': 2106})
        housing = next(row for row in fit['pieces'] if row['piece'] == 'left-instrument-housing')
        self.assertEqual(housing['upperMillimetres'], {'detailed': 344, 'native': 2106})
        delta = json.loads((proof.INTEGRATION / 'ORIGINAL-DIFFERENCE.json').read_bytes())
        self.assertEqual(delta['detailed']['strictLowerMillimetres'], 88)
        self.assertEqual(delta['native']['strictLowerMillimetres'], 2084)
        vertex = delta['native']['mandatoryOppositeSourceVertex']
        groups = (saker.ASSETS / 'source-piece-membership.bin').read_bytes()
        names = json.loads((saker.ASSETS / 'PIECES.json').read_bytes())['pieces']
        self.assertEqual(names[groups[vertex]], housing['piece'])
        self.assertNotIn(groups[vertex], groups[saker.DETAIL[0]:])
