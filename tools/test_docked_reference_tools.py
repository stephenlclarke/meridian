"""Import/integrity contract tests using synthetic schema inputs, not asset clearance."""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / 'crates/meridian-desktop/src/docked_screens/tools'
REVISION = 'a11fe16924d87d06021816f9037952afcbac08db'


def load(name):
    spec = importlib.util.spec_from_file_location(name, TOOLS / (name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


IMPORTER = load('import_reference')
VERIFIER = load('verify_reference')


class DockedReferenceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / 'data').mkdir()
        self.reference = self.root / 'explicit-object-store'
        self.calls = []
        # Exercise the required cardinalities and ordering with independent schema facts.
        # These generated records are never packaged or presented as original content.
        self.worlds = {'systems': [dict(
            galaxy=i // 256, system=i % 256, name=f'Fixture {i}',
            xLY=-1.5 if i == 0 else 2.5, yLY=3.0,
            economyID=1, economy='Fixture economy', governmentID=2,
            government='Fixture government', technology=3,
            populationBillions=4.0, productivityBillionCR=5.0,
            radiusKM=6000, ringed=False, surface='Fixture surface',
            climate='Fixture climate') for i in range(2048)]}
        self.kinds = ['ship'] * 162 + ['station'] * 6 + ['capsule', 'platform', 'other']
        self.source_override = {}
        self.addCleanup(patch.stopall)
        patch.object(IMPORTER, 'ROOT', self.root).start()
        patch.object(VERIFIER, 'ROOT', self.root).start()
        patch.object(subprocess, 'check_output', side_effect=self.git_show).start()
        (self.root / 'tests.rs').write_text('#[test]\nfn first() {}\n#[test]\nfn second() {}\n')
        (self.root / 'INTEGRATION.json').write_text(json.dumps({'authoredTests': ['first', 'second']}))
        (self.root / 'README.md').write_text('[facts](data/worlds.tsv) [external](https://example.invalid)\n')

    def git_show(self, command, cwd):
        self.assertEqual(command[:2], ['git', 'show'])
        self.assertEqual(cwd, self.reference)
        self.assertTrue(command[2].startswith(REVISION + ':'))
        path = command[2].split(':', 1)[1]
        self.calls.append(path)
        if path in self.source_override:
            return self.source_override[path]
        if path.endswith('/catalogue.json'):
            return json.dumps(self.worlds).encode()
        if path.startswith('docs/fleet-design-v2/ships/'):
            i = int(Path(path).stem[2:]) - 1
            return (f'id="SH{i + 1:03}"\nname="Fixture {i}"\nfamily="Test"\n'
                    f'objectKind="{self.kinds[i]}"\nclassification="Fixture"\n'
                    'decisionStatus="Unit schema fixture only"\n'
                    '[access]\nplayerFlyable=false\npurchasePolicy="unavailable"\n'
                    '[performance]\nspeed=100\nworkTools=["mining", "salvage"]\n'
                    '[performance.nested]\nignored=1\n'
                    '[fitting]\nenabled=true\nempty=[]\n').encode()
        return b'Unit schema fixture: source bytes, no licence or gameplay claim.\n'

    def build(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            IMPORTER.main(['--reference', str(self.reference)])
        return json.loads(output.getvalue())

    def rebind_output(self, name, data):
        # Separate integrity rejection from structural validation of correctly hashed input.
        (self.root / name).write_bytes(data)
        path = self.root / 'SOURCE.json'
        manifest = json.loads(path.read_text())
        item = next(row for row in manifest['outputs'] if row['path'] == name)
        item.update(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())
        path.write_text(json.dumps(manifest))

    def test_build_and_verify_complete_ordered_schema_and_immutable_reads(self):
        built = self.build()
        self.assertEqual((built['worlds'], built['ships'], built['objects']), (2048, 162, 171))
        manifest = json.loads((self.root / 'SOURCE.json').read_text())
        self.assertEqual(manifest['revision'], REVISION)
        self.assertEqual(len(self.calls), len(manifest['sources']))
        self.assertEqual(manifest['otherObjectKinds'], {'station': 6, 'capsule': 1, 'platform': 1, 'other': 1})
        self.assertFalse(manifest['hiddenPlughWorldsImported'])
        self.assertEqual(manifest['outputs'][0]['path'], 'data/worlds.tsv')
        self.assertTrue((self.root / 'data/worlds.tsv').read_text().startswith('0\t0\tFixture 0\t-1.5\t'))
        specs = (self.root / 'data/specifications.tsv').read_text()
        self.assertIn('SH001\tperformance\tworkTools\tmining, salvage\n', specs)
        self.assertIn('SH001\tfitting\tenabled\ttrue\n', specs)
        self.assertIn('SH001\tfitting\tempty\t[]\n', specs)
        self.assertNotIn('nested', specs)
        self.assertEqual((self.root / 'ORIGINAL-L337-MIT.txt').read_bytes(), self.git_show(['git', 'show', REVISION + ':LICENSE'], self.reference))
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            VERIFIER.main(['--reference', str(self.reference)])
        result = json.loads(output.getvalue())
        self.assertEqual(result['status'], 'SOURCE_ONLY_PASS;DOES_NOT_EXECUTE_RUST_TESTS')
        self.assertEqual(result['specificationFields'], 171 * 4)
        self.assertEqual(result['authoredRustTests'], 2)
        self.assertTrue(result['negativeOriginalCoordinatesPreserved'])

    def test_generated_bytes_are_utf8_lf_and_match_the_manifest_on_every_platform(self):
        self.worlds['systems'][0]['name'] = 'Fixture \u03a9'
        self.build()
        expected = '0\t0\tFixture \u03a9\t-1.5\t'.encode('utf-8')
        self.assertTrue((self.root / 'data/worlds.tsv').read_bytes().startswith(expected))
        manifest = json.loads((self.root / 'SOURCE.json').read_bytes())
        self.assertNotIn(b'\r', (self.root / 'SOURCE.json').read_bytes())
        for item in manifest['outputs']:
            raw = (self.root / item['path']).read_bytes()
            self.assertNotIn(b'\r', raw)
            self.assertTrue(raw.endswith(b'\n'))
            self.assertEqual(len(raw), item['bytes'])
            self.assertEqual(hashlib.sha256(raw).hexdigest(), item['sha256'])
        self.assertEqual(VERIFIER.verify(self.reference)['worlds'], 2048)

    def test_import_rejects_tsv_separator_world_count_and_object_kind_changes(self):
        for separator in ['\t', '\n', '\r']:
            with self.subTest(separator=repr(separator)):
                self.worlds['systems'][0]['name'] = 'bad' + separator + 'name'
                with self.assertRaisesRegex(ValueError, 'separator'):
                    self.build()
        self.worlds['systems'][0]['name'] = 'restored'
        last = self.worlds['systems'].pop()
        with self.assertRaises(AssertionError):
            self.build()
        self.worlds['systems'].append(last)
        self.kinds[0] = 'station'
        with self.assertRaises(AssertionError):
            self.build()
        self.assertFalse((self.root / 'SOURCE.json').exists())

    def test_immutable_source_and_same_size_output_tampering_refuse(self):
        self.build()
        self.source_override['Sources/L337Core/ScreenNavigation.swift'] = b'tampered source'
        with self.assertRaises(AssertionError):
            VERIFIER.verify(self.reference)
        self.source_override.clear()
        path = self.root / 'data/worlds.tsv'
        original = path.read_bytes()
        path.write_bytes(original.replace(b'Fixture 0', b'Changed 0', 1))
        self.assertEqual(path.stat().st_size, len(original))
        with self.assertRaises(AssertionError):
            VERIFIER.verify(self.reference)

    def test_structural_world_order_and_nonfinite_coordinates_refuse_after_rebinding(self):
        self.build()
        name = 'data/worlds.tsv'
        original = (self.root / name).read_text()
        for invalid in [original.replace('0\t0\t', '0\t1\t', 1),
                        original.replace('\t-1.5\t', '\tnan\t', 1),
                        original.replace('\t-1.5\t', '\tinf\t', 1)]:
            with self.subTest(invalid=invalid[:50]):
                self.rebind_output(name, invalid.encode())
                with self.assertRaises(AssertionError):
                    VERIFIER.verify(self.reference)

    def test_object_identity_and_unknown_specification_refuse_after_rebinding(self):
        self.build()
        name = 'data/objects.tsv'
        original = (self.root / name).read_bytes()
        self.rebind_output(name, original.replace(b'SH001', b'SH999', 1))
        with self.assertRaises(AssertionError):
            VERIFIER.verify(self.reference)
        self.rebind_output(name, original)
        name = 'data/specifications.tsv'
        self.rebind_output(name, (self.root / name).read_bytes().replace(b'SH001', b'SH999', 1))
        with self.assertRaises(AssertionError):
            VERIFIER.verify(self.reference)

    def test_wrong_revision_test_inventory_and_broken_local_link_refuse(self):
        self.build()
        path = self.root / 'SOURCE.json'
        original = path.read_bytes()
        manifest = json.loads(original)
        manifest['revision'] = '0' * 40
        path.write_text(json.dumps(manifest))
        with self.assertRaises(AssertionError):
            VERIFIER.verify(self.reference)
        path.write_bytes(original)
        (self.root / 'INTEGRATION.json').write_text(json.dumps({'authoredTests': ['second', 'first']}))
        with self.assertRaises(AssertionError):
            VERIFIER.verify(self.reference)
        (self.root / 'INTEGRATION.json').write_text(json.dumps({'authoredTests': ['first', 'second']}))
        (self.root / 'README.md').write_text('[missing](missing-file.md)\n')
        with self.assertRaises(AssertionError):
            VERIFIER.verify(self.reference)

    def test_explicit_reference_argument_is_required_by_both_commands(self):
        for module in [IMPORTER, VERIFIER]:
            with self.subTest(module=module.__name__), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as raised:
                    module.main([])
                self.assertEqual(raised.exception.code, 2)
            script = TOOLS / (module.__name__ + '.py')
            with self.subTest(script=script.name), patch.object(sys, 'argv', [str(script)]):
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
                    # Exercise the actual script entry point; absent --reference must refuse
                    # before opening a Git store or writing any generated content.
                    runpy.run_path(str(script), run_name='__main__')
                self.assertEqual(raised.exception.code, 2)


if __name__ == '__main__':
    unittest.main()
