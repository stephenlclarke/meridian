"""SH036 original-data, envelope and tangible source-contact regressions."""
import json
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
import shutil
import struct
from subprocess import CompletedProcess
import tempfile
import unittest
from unittest.mock import patch

import prepare_zebu_studio as generator
import verify_zebu_studio_export as verifier
from prepare_zebu_studio import (
    ASSETS, CONTACTS, COUNTS, INTEGRATION, NAMES, authenticate, clearance_bound,
    first_hit, prepare,
)


class ZebuStudioTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.streams, cls.artifacts = prepare()
        cls.definition = json.loads(cls.artifacts['ZEBU-STUDIO.json'])
        cls.witnesses = json.loads(cls.artifacts['CONTACT-WITNESSES.json'])
        cls.resource, _ = authenticate(ASSETS)

    def test_all_generated_outputs_are_exact_committed_replays(self):
        for folder, files in ((ASSETS, self.streams), (INTEGRATION, self.artifacts)):
            for name, data in files.items():
                with self.subTest(name=name):
                    self.assertEqual((folder / name).read_bytes(), data)

    def test_identity_units_and_unknown_operational_authority(self):
        d = self.definition
        self.assertEqual((d['id'], d['name'], d['metresPerUnit'], d['roundingMetres']), ('SH036', 'Zebu', 1, .02))
        self.assertEqual(d['flight'], {'maxSpeed': 33.0})
        self.assertIsNone(d['engineering'])
        self.assertEqual(d['provenance']['physicalStatus'], 'authored empty-dry references only; no operational qualification')
        self.assertEqual(len(NAMES), 24)
        for key, legacy in (('detail', 'design'), ('native', 'nativeDesign')):
            mesh = d[key]
            self.assertEqual(mesh['vertices'], self.resource[legacy]['vertices'])
            self.assertEqual(mesh['triangles'], self.resource[legacy]['faces'])
            self.assertTrue(all(len({mesh['groups'][i] for i in face}) == 1 for face in mesh['triangles']))

    def test_packed_source_and_original_visual_corners(self):
        detailed, native = COUNTS
        points = self.definition['detail']['vertices'] + self.definition['native']['vertices']
        self.assertEqual(self.streams['source.vertices'][:16], struct.pack('<8sII', b'MSH036V1', detailed[0], native[0]))
        self.assertEqual(list(struct.iter_unpack('<3d', self.streams['source.vertices'][16:])), list(map(tuple, points)))
        faces = self.definition['detail']['triangles'] + [[v + detailed[0] for v in f] for f in self.definition['native']['triangles']]
        self.assertEqual(list(struct.iter_unpack('<3I', self.streams['source-triangles.bin'])), list(map(tuple, faces)))
        self.assertEqual(self.streams['source-piece-membership.bin'], bytes(self.definition['detail']['groups'] + self.definition['native']['groups']))
        for file_name, mesh_key in (('visual.bin', 'design'), ('native-visual.bin', 'nativeDesign')):
            source = self.resource[mesh_key]
            packed = self.streams[file_name]
            self.assertEqual(packed[:16], struct.pack('<8sII', b'ZEBUVIZ1', len(source['faces'])*3, 56))
            corners = list(struct.iter_unpack('<12fII', packed[16:]))
            self.assertEqual(len(corners), 3 * len(source['faces']))
            for i, (face, mat, mapping) in enumerate(zip(source['faces'], source['faceMaterials'], source['faceTextures'], strict=True)):
                material = source['materials'][mat]
                for j, vertex in enumerate(face):
                    row = corners[3*i+j]
                    self.assertEqual(struct.pack('<3f', *row[:3]), struct.pack('<3f', *source['vertices'][vertex]))
                    self.assertEqual(struct.pack('<4f', *row[6:10]), struct.pack('<4f', *((material['colour'] >> shift & 255)/255 for shift in (24,16,8,0))))
                    self.assertEqual(struct.pack('<2f', *row[10:12]), struct.pack('<2f', *mapping['uv'][j]))
                    self.assertEqual(row[12:], (mapping['texture'] & 0xffffffff, int(material['emissive'])))

    def test_openings_are_clear_and_contact_probes_cross_original_source(self):
        points = self.definition['detail']['vertices'] + self.definition['native']['vertices']
        memberships = self.definition['detail']['groups'] + self.definition['native']['groups']
        groups = [[p for p, n in zip(points, memberships, strict=True) if n == i] for i in range(len(NAMES))]
        for result, bound in zip(self.witnesses['clearances'], (.029, .029, .179), strict=True):
            self.assertEqual(result['conservativeNetClearanceMetres'], bound)
            self.assertGreater(clearance_bound(result['route'], groups), bound)
        expected = ((169, 19), (911, 79), (1645, 137))
        for contact, (dface, nface) in zip(CONTACTS, expected, strict=True):
            self.assertEqual(first_hit(self.resource['design'], contact)['face'], dface)
            self.assertEqual(first_hit(self.resource['nativeDesign'], contact)['face'], nface)
        for opening in self.definition['clearances']:
            with self.assertRaisesRegex(ValueError, 'No contact'):
                first_hit(self.resource['design'], opening)
        bad = {**self.definition['clearances'][0], 'radiusMetres': 1}
        self.assertLess(clearance_bound(bad, groups), 0)

    def test_wrong_original_bytes_are_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary) / 'sh036'
            shutil.copytree(ASSETS, folder)
            source = folder / 'native-export.json'
            source.write_bytes(source.read_bytes() + b' ')
            with self.assertRaisesRegex(ValueError, 'Changed SH036 original native-export.json'):
                authenticate(folder)

    def test_generator_commands_and_changed_output_refusal(self):
        with tempfile.TemporaryDirectory() as temporary:
            assets = Path(temporary) / 'source'
            integration = Path(temporary) / 'integration'
            with patch.object(generator, 'ASSETS', assets), patch.object(generator, 'INTEGRATION', integration), patch.object(generator, 'prepare', return_value=(self.streams, self.artifacts)):
                with patch('sys.argv', ['prepare_zebu_studio.py']):
                    generator.main()
                with patch('sys.argv', ['prepare_zebu_studio.py', '--check']):
                    generator.main()
                    (assets / 'PIECES.json').write_bytes(b'changed')
                    with self.assertRaisesRegex(ValueError, 'Changed generated Zebu PIECES.json'):
                        generator.main()

    def test_execution_binding_checks_report_identity_and_refuses_wrong_contacts(self):
        """Use a fake process boundary only; actual signed CLI evidence is separate."""
        source = self.artifacts['ZEBU-STUDIO.json']
        groups = {}
        for key in ('detail', 'native'):
            mesh = self.definition[key]
            for point, group in zip(mesh['vertices'], mesh['groups'], strict=True):
                groups.setdefault(group, []).append(point)
        support = [{'id': i, 'points': groups[i], 'roundingMetres': .02} for i in sorted(groups)]
        clearances = [{'name': r['route']['name'], 'passed': True} for r in self.witnesses['clearances']]
        contacts = [r['route']['name'] for r in self.witnesses['contacts']]
        positive = {'valid': True, 'supportGroups': support, 'diagnostics': [], 'clearances': clearances}
        negative = {'valid': False, 'supportGroups': support, 'diagnostics': [{'message': f'Clearance {name} is blocked or indeterminate'} for name in contacts], 'clearances': clearances + [{'name': name, 'passed': False} for name in contacts]}

        def fake_run(args, stdout, stderr, check):
            if '--export' in args:
                Path(args[args.index('--export') + 1]).write_bytes(source.rstrip(b'\n'))
                stdout.write(json.dumps(positive).encode())
                return CompletedProcess(args, 0)
            Path(args[args.index('--report') + 1]).write_bytes(json.dumps(negative).encode())
            return CompletedProcess(args, 1)

        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / 'evidence'
            with patch.object(verifier.subprocess, 'run', side_effect=fake_run), patch('sys.argv', ['verify_zebu_studio_export.py', '--studio', 'studio', '--validator', 'validator', '--output', str(target)]), redirect_stdout(StringIO()) as stdout:
                verifier.main()
                self.assertEqual(len(json.loads(stdout.getvalue())), 4)
            with patch('sys.argv', ['verify_zebu_studio_export.py', '--check', '--output', str(target)]), redirect_stdout(StringIO()):
                verifier.main()
            negative['clearances'][-1]['passed'] = True
            (target / 'BLOCKED-REPORT.json').write_bytes(json.dumps(negative).encode())
            with self.assertRaisesRegex(ValueError, 'contact routes'):
                verifier.compare_reports(target, source, self.witnesses)
            with patch('sys.argv', ['verify_zebu_studio_export.py', '--output', str(target)]), redirect_stderr(StringIO()), self.assertRaises(SystemExit):
                verifier.main()


if __name__ == '__main__':
    unittest.main()
