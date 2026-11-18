"""Exercise the Mule importer using the exact packaged source records."""
import copy
import hashlib
import io
import json
from pathlib import Path
import runpy
import sys
import tempfile
import tomllib
import unittest
from contextlib import redirect_stdout
from unittest.mock import patch
import zipfile

import import_mule_envelope_source as importer

PACKAGED = Path(__file__).resolve().parents[1] / 'assets/fleet/sh001'
ARCHIVE = Path('fleet-design-r3.zip')
SNAPSHOT = Path('snapshot')
PROPOSAL = Path('proposal')


def archive_fixture(member, data):
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w') as archive:
        archive.writestr(zipfile.ZipInfo(member, (1980, 1, 1, 0, 0, 0)), data)
    return buffer.getvalue()


class SourceImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.records = {name: (PACKAGED / name).read_bytes() for name in importer.PINS}
        cls.manifest = json.loads((PACKAGED / 'MANIFEST.json').read_bytes())
        cls.resource = tomllib.loads(cls.records[importer.RESOURCE].decode())
        cls.original = tomllib.loads(cls.records[importer.ORIGINAL_DESIGN].decode())
        cls.ownership = json.loads(cls.records[importer.OWNERSHIP])

    def setUp(self):
        self.workspace = Path(self.enterContext(tempfile.TemporaryDirectory())).resolve()
        self.approved = self.workspace / 'approved'
        self.output_root = self.workspace / 'output'
        self.destination = self.output_root / 'assets/fleet/sh001'
        self.destination.mkdir(parents=True)
        for name, location, relative in importer.SOURCE_FILES:
            path = self.approved / location / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(self.records[name])
        self.archive = self.approved / ARCHIVE
        self.archive_bytes = archive_fixture(importer.ARCHIVE_MEMBER, self.records['contract.toml'])
        self.archive.write_bytes(self.archive_bytes)
        self.archive_digest = hashlib.sha256(self.archive_bytes).hexdigest()
        self.enterContext(patch.dict('os.environ', {importer.SOURCE_ROOT_ENV: str(self.approved)}))
        self.enterContext(patch.object(importer, 'ROOT', self.output_root))

    def run_importer(self, check=False):
        # CI has the pinned SH001 member, not the complete authority ZIP.
        # Replace only the expected full-archive hash for this test fixture.
        with patch.object(importer, 'ARCHIVE_SHA256', self.archive_digest):
            return importer.import_source(SNAPSHOT, PROPOSAL, ARCHIVE, check)

    def test_generation_matches_every_packaged_output_and_check_writes_nothing(self):
        result = self.run_importer()
        self.assertEqual(set(p.name for p in self.destination.iterdir()),
                         set(self.manifest['files']) | {'MANIFEST.json'})
        for name, pin in self.manifest['files'].items():
            with self.subTest(output=name):
                output = (self.destination / name).read_bytes()
                self.assertEqual(output, (PACKAGED / name).read_bytes())
                self.assertEqual(len(output), pin['bytes'])
                self.assertEqual(hashlib.sha256(output).hexdigest(), pin['sha256'])
        expected_manifest = (PACKAGED / 'MANIFEST.json').read_bytes().replace(
            importer.ARCHIVE_SHA256.encode(), self.archive_digest.encode()
        )
        self.assertEqual((self.destination / 'MANIFEST.json').read_bytes(), expected_manifest)
        self.assertEqual(result, json.loads(expected_manifest))

        with patch.object(Path, 'write_bytes', side_effect=AssertionError('check wrote a file')):
            self.assertEqual(self.run_importer(check=True), result)
        (self.destination / 'LICENSE').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'Changed packaged Mule output: LICENSE'):
            self.run_importer(check=True)
        self.assertEqual((self.destination / 'LICENSE').read_bytes(), b'changed')

    def test_legitimate_cli_check_reads_the_selected_zip_once_and_emits_result(self):
        self.run_importer()
        reads = []
        original_read = Path.read_bytes

        def count_reads(path):
            if path == self.archive:
                reads.append(path)
            return original_read(path)

        argv = ['import_mule_envelope_source.py', '--snapshot', str(self.approved / SNAPSHOT),
                '--proposal', str(PROPOSAL), '--archive', str(self.archive), '--check']
        stdout = io.StringIO()
        with (
            patch.object(sys, 'argv', argv),
            patch.object(Path, 'read_bytes', count_reads),
            patch.object(importer, 'ARCHIVE_SHA256', self.archive_digest),
            redirect_stdout(stdout),
        ):
            importer.main()
        self.assertEqual(reads, [self.archive])
        self.assertEqual(json.loads(stdout.getvalue()), {
            'status': 'SOURCE_BYTES_AND_VERTEX_ENCODING_ONLY',
            'vertices': {'detailed': 4360, 'native': 984},
            'envelopeOrConstructorExecuted': False,
        })

    def test_real_archive_pin_refuses_the_test_zip_before_parsing_or_writing(self):
        self.assertEqual(importer.ARCHIVE_SHA256,
                         '4392dd6eb5a289145c88e5cef3ec3f83552d7920d9dc3bbb3331b8539c11ec9a')
        with (
            patch.object(importer.zipfile, 'ZipFile', side_effect=AssertionError('parsed early')),
            self.assertRaisesRegex(ValueError, 'Changed current R3 archive'),
        ):
            importer.import_source(SNAPSHOT, PROPOSAL, ARCHIVE)
        self.assertEqual(list(self.destination.iterdir()), [])

    def test_archive_gate_and_pinned_member_are_independent(self):
        wrong_member = archive_fixture(importer.ARCHIVE_MEMBER, b'changed original contract')
        self.archive.write_bytes(wrong_member)
        self.archive_digest = hashlib.sha256(wrong_member).hexdigest()
        with self.assertRaisesRegex(ValueError, 'Changed Mule input: contract.toml'):
            self.run_importer()
        self.assertEqual(list(self.destination.iterdir()), [])

    def test_changed_input_and_notice_leave_existing_outputs_intact(self):
        self.run_importer()
        before = {p.name: p.read_bytes() for p in self.destination.iterdir()}
        for name, location, relative in importer.SOURCE_FILES:
            with self.subTest(input=name):
                path = self.approved / location / relative
                path.write_bytes(self.records[name] + b'\n')
                with self.assertRaisesRegex(ValueError, f'Changed Mule input: {name}'):
                    self.run_importer()
                path.write_bytes(self.records[name])
        self.assertEqual({p.name: p.read_bytes() for p in self.destination.iterdir()}, before)

    def test_missing_or_non_directory_source_root_refuses(self):
        with patch.dict('os.environ', {importer.SOURCE_ROOT_ENV: ''}):
            with self.assertRaisesRegex(ValueError, 'approved Mule source directory'):
                self.run_importer()
        with patch.dict('os.environ', {importer.SOURCE_ROOT_ENV: str(self.archive)}):
            with self.assertRaisesRegex(ValueError, 'must be a directory'):
                self.run_importer()

    def test_paths_outside_the_approved_directory_and_wrong_types_refuse_without_reading(self):
        with patch.object(Path, 'read_bytes', side_effect=AssertionError('read unapproved path')):
            with self.assertRaisesRegex(ValueError, 'outside the approved directory'):
                importer.import_source(SNAPSHOT, PROPOSAL, self.workspace / 'outside.zip')
            with self.assertRaisesRegex(ValueError, 'outside the approved directory'):
                importer.import_source(SNAPSHOT, PROPOSAL, Path('../outside.zip'))
            with self.assertRaisesRegex(ValueError, 'wrong file type'):
                importer.import_source(SNAPSHOT, PROPOSAL, SNAPSHOT)

    def test_source_redirect_is_refused_before_reading(self):
        link = self.approved / 'source-alias'
        try:
            link.symlink_to(self.approved / SNAPSHOT, target_is_directory=True)
        except OSError as error:
            self.skipTest(f'symbolic link unavailable: {error}')
        with (
            patch.object(Path, 'read_bytes', side_effect=AssertionError('read redirected path')),
            self.assertRaisesRegex(ValueError, 'must not redirect'),
        ):
            importer.import_source(Path(link.name), PROPOSAL, ARCHIVE)

    def test_output_file_type_preflight_preserves_existing_files(self):
        self.run_importer()
        original = (self.destination / 'MANIFEST.json').read_bytes()
        obstructed = self.destination / 'source-triangles.bin'
        obstructed.unlink()
        obstructed.mkdir()
        with self.assertRaisesRegex(ValueError, 'wrong file type'):
            self.run_importer()
        self.assertEqual((self.destination / 'MANIFEST.json').read_bytes(), original)

    def test_comparison_cardinalities_and_source_licenses_refuse(self):
        resource = copy.deepcopy(self.resource)
        resource['design']['vertices'][0][0] = 0.0
        with self.assertRaisesRegex(ValueError, 'differs from original design'):
            importer.encode_vertices(resource, self.original, self.ownership)
        resource['design']['vertices'].pop()
        with self.assertRaisesRegex(ValueError, 'source cardinality'):
            importer.encode_vertices(resource, resource, self.ownership)
        resource = copy.deepcopy(self.resource)
        resource['design']['appearanceLicense'] = 'changed'
        with self.assertRaisesRegex(ValueError, 'original source notices'):
            importer.encode_vertices(resource, resource, self.ownership)

    def test_component_order_membership_and_coordinate_arity_refuse(self):
        ownership = copy.deepcopy(self.ownership)
        parts = ownership['models']['design']['parts']
        parts.reverse()
        with self.assertRaisesRegex(ValueError, 'not complete and ordered'):
            importer.encode_vertices(self.resource, self.original, ownership)
        parts.reverse()
        parts.pop()
        with self.assertRaisesRegex(ValueError, 'Lost original component membership'):
            importer.encode_vertices(self.resource, self.original, ownership)
        resource = copy.deepcopy(self.resource)
        resource['nativeDesign']['vertices'][0].pop()
        with self.assertRaisesRegex(ValueError, 'coordinate arity'):
            importer.encode_vertices(resource, resource, self.ownership)

    def test_component_group_counts_and_triangle_indices_refuse(self):
        ownership = copy.deepcopy(self.ownership)
        parts = ownership['models']['design']['parts']
        part = next(part for part in parts if part['name'] == 'pannier pressure envelope')
        part['name'] = 'unknown pannier operation'
        with self.assertRaisesRegex(ValueError, 'pannier operation count'):
            importer.component_groups(ownership)
        ownership = copy.deepcopy(self.ownership)
        for part in ownership['models']['design']['parts']:
            if part['name'].startswith('body primary pressure envelope/service-well-'):
                part['name'] = 'unknown body well'
        with self.assertRaisesRegex(ValueError, 'body well assembly count'):
            importer.component_groups(ownership)

        groups, _ = importer.component_groups(self.ownership)
        resource = copy.deepcopy(self.resource)
        resource['design']['faces'][0][0] = len(resource['design']['vertices'])
        with self.assertRaisesRegex(ValueError, 'triangle crosses its creator component range'):
            importer.encode_triangles(resource, groups)

    def test_script_entry_point_exposes_existing_flags_and_approved_root(self):
        stdout = io.StringIO()
        with (
            patch.object(sys, 'argv', ['import_mule_envelope_source.py', '--help']),
            redirect_stdout(stdout),
            self.assertRaises(SystemExit) as exit_result,
        ):
            runpy.run_path(importer.__file__, run_name='__main__')
        self.assertEqual(exit_result.exception.code, 0)
        for word in ('--snapshot', '--proposal', '--archive', '--check', importer.SOURCE_ROOT_ENV):
            self.assertIn(word, stdout.getvalue())


if __name__ == '__main__':
    unittest.main()
