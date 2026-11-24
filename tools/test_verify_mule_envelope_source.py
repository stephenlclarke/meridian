"""Exercise the independent source checks, including mismatches in protected inputs."""
import json
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import generate_flight_computer_inverse as generator
import verify_mule_envelope_source as verifier


class MuleSourceVerification(unittest.TestCase):
    def copy_protected_sources(self, root):
        core = Path('crates/meridian-core/src')
        shutil.copytree(verifier.ROOT / core, root / core)
        for record in (verifier.PROTECTED_CORE, verifier.FLIGHT_COMPUTER_INVERSE):
            (root / record).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(verifier.ROOT / record, root / record)
        return root / core

    def test_bundled_assets_core_inverse_and_reference_bindings_agree(self):
        result = verifier.verify()
        self.assertEqual(result['protectedCoreFilesChecked'], 115)
        self.assertEqual(result['protectedCoreFilesWithMuleHooks'], 6)
        self.assertEqual(result['protectedCoreFilesWithFlightComputerChanges'], 17)
        self.assertEqual(result['geometryCoreTests'], 1)
        self.assertEqual(result['restoreCoreTests'], 1)
        self.assertEqual(result['totalMuleCoreTests'], 24)
        self.assertEqual(result['vertexEncoding'], 4360 + 984)
        self.assertEqual(result['triangleEncoding'], 7236 + 1772)
        self.assertFalse(result['constructorsOrContactOrContainmentOrCompilerExecuted'])

    def test_unrelated_edit_in_nested_protected_source_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = self.copy_protected_sources(root)
            with (core / 'flight/emp/tests.rs').open('ab') as stream:
                stream.write(b'\n')
            with self.assertRaisesRegex(AssertionError, 'flight/emp/tests'):
                verifier.verify_protected_core(root)

    def test_unrelated_edit_outside_new_inverse_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = self.copy_protected_sources(root)
            inverse = verifier.flight_computer_inverse(root, verifier.protected_core_pins(root))
            relative = next(name for name in verifier.protected_core_pins(root) if name not in inverse)
            source = core / relative
            source.write_bytes(source.read_bytes() + b'\n')
            with self.assertRaisesRegex(AssertionError, relative):
                verifier.verify_protected_core(root)

    def test_altered_inverse_cannot_authorize_an_unrelated_edit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.copy_protected_sources(root)
            inverse = root / verifier.FLIGHT_COMPUTER_INVERSE
            inverse.write_bytes(inverse.read_bytes() + b'\n')
            with self.assertRaisesRegex(AssertionError, 'flight computer inverse manifest'):
                verifier.verify_protected_core(root)

    def test_component_triangle_may_not_use_a_neighbouring_part_vertex(self):
        part = dict(index=0, name='body primary pressure envelope', vertexStart=5,
                    vertexCount=3, faceStart=0, faceCount=1)
        assigned = dict(part, component=0, piece=0)
        verifier.verify_component('design', part, assigned, [(5, 6, 7)], {})
        with self.assertRaises(AssertionError):
            verifier.verify_component('design', part, assigned, [(5, 6, 8)], {})

    def test_hull_publications_must_each_be_removed_exactly_once(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = self.copy_protected_sources(root)
            source = core / verifier.LIB_SOURCE
            original = source.read_bytes()
            for publication in (
                b'pub mod stag_liner;\n\n', b'pub mod saker;\n', b'pub mod eagle_cruiser;\n',
                b'pub mod harpy_assault;\n',
            ):
                for altered in (original.replace(publication, b''), original + publication):
                    with self.subTest(publication=publication):
                        source.write_bytes(altered)
                        with self.assertRaisesRegex(AssertionError, 'lib.rs'):
                            verifier.verify_protected_core(root)
            for declaration in (b'pub mod saker;\n', b'pub mod eagle_cruiser;\n', b'pub mod harpy_assault;\n'):
                source.write_bytes(original.replace(declaration, b'') + declaration)
                with self.assertRaisesRegex(AssertionError, 'lib.rs'):
                    verifier.verify_protected_core(root)

    def test_historical_stag_check_still_refuses_a_forged_preimage(self):
        # Simulate even a mistakenly approved new inverse with bad historical bytes.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = self.copy_protected_sources(root)
            manifest = root / verifier.FLIGHT_COMPUTER_INVERSE
            inverse = json.loads(manifest.read_bytes())
            relative = verifier.LIB_SOURCE
            current = verifier.undo_harpy_publication(relative, (core / relative).read_bytes())
            current = verifier.undo_eagle_publication(relative, current)
            current = verifier.undo_saker_publication(relative, current)
            before = verifier.undo_flight_computer(relative, current, inverse['files'])
            addition = b'pub mod stag_liner;\n\n'
            self.assertEqual(before.count(addition), 1)
            for altered in (before.replace(addition, b''), before + addition):
                inverse['files'][relative] = generator.file_inverse(altered, current)
                raw = json.dumps(inverse).encode('utf-8')
                manifest.write_bytes(raw)
                with patch.object(verifier, 'FLIGHT_COMPUTER_INVERSE_SHA256', verifier.sha(raw)):
                    with self.assertRaisesRegex(AssertionError, 'lib.rs'):
                        verifier.verify_protected_core(root)


class FlightComputerInverse(unittest.TestCase):
    def test_exact_offsets_restore_multiple_changed_spans(self):
        before = b'old\nkeep\ndelete\nkeep2\nold final'
        after = b'new\nkeep\nkeep2\ninsert\nnew final'
        record = generator.file_inverse(before, after)
        self.assertGreater(len(record['hunks']), 1)
        inverse = {'source': record}
        self.assertEqual(verifier.undo_flight_computer('source', after, inverse), before)
        self.assertEqual(verifier.undo_flight_computer('unrelated', after, inverse), after)
        for altered in (after + b'\n', after.replace(b'keep2', b'changed')):
            with self.assertRaisesRegex(AssertionError, 'source'):
                verifier.undo_flight_computer('source', altered, inverse)

    def test_wrong_offsets_and_original_hash_refuse(self):
        before = b'keep\nold\n'
        after = b'keep\nnew\n'
        record = generator.file_inverse(before, after)
        record['hunks'][0]['offset'] = 0
        with self.assertRaisesRegex(AssertionError, 'source'):
            verifier.undo_flight_computer('source', after, {'source': record})
        record = generator.file_inverse(before, after)
        record['beforeSHA256'] = verifier.sha(b'unrelated')
        with self.assertRaisesRegex(AssertionError, 'source'):
            verifier.undo_flight_computer('source', after, {'source': record})

    def test_generator_only_records_changed_protected_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = root / 'crates/meridian-core/src'
            core.mkdir(parents=True)
            sources = {'a.rs': b'unchanged\n', 'b.rs': b'now\n', 'outside.rs': b'ignored\n'}
            for relative, data in sources.items():
                (core / relative).write_bytes(data)
            with patch.object(generator, 'protected_core_pins', return_value={'a.rs': '', 'b.rs': ''}), \
                 patch.object(generator.subprocess, 'check_output', side_effect=[sources['a.rs'], b'old\n']) as git:
                inverse = generator.generate(root)
            self.assertEqual(inverse['base'], verifier.FLIGHT_COMPUTER_BASE)
            self.assertEqual(set(inverse['files']), {'b.rs'})
            self.assertEqual(verifier.undo_flight_computer('b.rs', sources['b.rs'], inverse['files']), b'old\n')
            self.assertEqual(git.call_count, 2)
            for call, relative in zip(git.call_args_list, ('a.rs', 'b.rs'), strict=True):
                self.assertEqual(call.args[0], ['git', 'cat-file', 'blob',
                    verifier.FLIGHT_COMPUTER_BASE + ':crates/meridian-core/src/' + relative])
