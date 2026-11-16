"""Host binding regressions. These never start a Rust process or construct a hull."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('whole_five_host', Path(__file__).with_name('run_original_five.py'))
HOST = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HOST)


class SourceBindingTests(unittest.TestCase):
    def fixture(self, root):
        names = ['Cargo.toml', 'Cargo.lock', 'crates/meridian-geometry/Cargo.toml',
                 'crates/meridian-geometry/src/bin/original_five.rs',
                 'crates/meridian-geometry/tools/run_original_five.py',
                 'crates/meridian-geometry/fixtures/ORIGINAL-BINDINGS.json',
                 'crates/meridian-geometry/integration/WHOLE-FIVE-CRITERIA.json']
        names += ['crates/meridian-geometry/src/' + name + '.rs' for name in
                  ('lib', 'adapter', 'arrangement', 'budget', 'certificate', 'exact', 'source')]
        names += ['crates/meridian-geometry/fixtures/' + ident + '.facets' for ident in
                  ('SH010', 'SH011', 'SH016', 'SH017', 'SH166')]
        for name in names:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(name.encode())
        return {'commit': '1' * 40, 'tree': '2' * 40,
                'files': {name: {'sha256': HOST.digest(root / name)} for name in names}}

    def test_exact_commit_tree_files_and_helper_binding(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            manifest = self.fixture(root)
            with patch.object(HOST, '__file__', str(root / 'crates/meridian-geometry/tools/run_original_five.py')):
                self.assertEqual(HOST.verify_source(root, manifest), ('1' * 40, '2' * 40))

    def test_missing_library_or_input_binding_refuses(self):
        for name in ('crates/meridian-geometry/src/certificate.rs',
                     'crates/meridian-geometry/fixtures/SH016.facets'):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as folder:
                root = Path(folder).resolve()
                manifest = self.fixture(root)
                del manifest['files'][name]
                with self.assertRaisesRegex(ValueError, 'binding misses'):
                    HOST.verify_source(root, manifest)

    def test_changed_source_bytes_refuse(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            manifest = self.fixture(root)
            (root / 'crates/meridian-geometry/fixtures/SH011.facets').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'binding mismatch'):
                HOST.verify_source(root, manifest)

    def test_escaped_or_symlink_source_refuses(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            manifest = self.fixture(root)
            manifest['files']['../outside'] = {'sha256': hashlib.sha256(b'').hexdigest()}
            with self.assertRaisesRegex(ValueError, 'Unsafe source'):
                HOST.verify_source(root, manifest)
            del manifest['files']['../outside']
            name = 'crates/meridian-geometry/fixtures/SH010.facets'
            path = root / name
            path.unlink()
            path.symlink_to(root / 'Cargo.toml')
            with self.assertRaisesRegex(ValueError, 'escapes frozen'):
                HOST.verify_source(root, manifest)

class TerminalBindingTests(unittest.TestCase):
    def terminal(self):
        ids = ('SH010', 'SH011', 'SH016', 'SH017', 'SH166')
        names = ('bound_original_decode', 'original_construction', 'source_and_affine_fields',
                 'complete_facet_certificates', 'closed_topology_and_limits')
        exports = {ident: str(index) * 64 for index, ident in enumerate(ids)}
        cases = []
        for ident in ids:
            bodies = [{'name': name, 'sourceAsset': ident, 'sourceInputSHA256': exports[ident],
                       'status': status, 'chargedOperationDelta': None if status == 'UNRUN' else 10,
                       'peakStorageAtBodyTerminal': None if status == 'UNRUN' else 20}
                      for name, status in zip(names, ('PASS', 'REFUSED', 'UNRUN', 'UNRUN', 'UNRUN'), strict=True)]
            cases.append({'id': ident, 'bodies': bodies, 'completeCandidateReturned': False,
                          'allCandidateChecksPassed': False, 'fleetAdmission': False})
        return {'declaredBodies': 25, 'cases': cases, 'bodiesPass': 5, 'bodiesRefused': 5,
                'bodiesFail': 0, 'bodiesUnrun': 15, 'all171Accepted': False,
                'nativeAcceptance': False}, exports

    def test_typed_refusal_and_unrun_terminal_binds_every_body(self):
        terminal, exports = self.terminal()
        HOST.validate_terminal(terminal, 1, exports)

    def test_candidate_or_dependent_pass_after_refusal_is_rejected(self):
        terminal, exports = self.terminal()
        terminal['cases'][0]['completeCandidateReturned'] = True
        with self.assertRaisesRegex(ValueError, 'candidate disagrees'):
            HOST.validate_terminal(terminal, 1, exports)
        terminal, exports = self.terminal()
        body = terminal['cases'][0]['bodies'][2]
        body.update(status='PASS', chargedOperationDelta=0, peakStorageAtBodyTerminal=20)
        with self.assertRaisesRegex(ValueError, 'failed dependency'):
            HOST.validate_terminal(terminal, 1, exports)


if __name__ == '__main__':
    unittest.main()
