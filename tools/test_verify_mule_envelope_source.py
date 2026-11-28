"""Exercise the independent source checks, including mismatches in protected inputs."""
from dataclasses import replace
import json
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import generate_flight_computer_inverse as generator
import protected_core_entries as records
import verify_mule_envelope_source as verifier


class MuleSourceVerification(unittest.TestCase):
    def copy_protected_sources(self, root):
        core = records.CORE_SOURCE
        shutil.copytree(verifier.ROOT / core, root / core)
        files = [verifier.ROOT / verifier.PROTECTED_CORE, verifier.ROOT / records.FROZEN_ENTRIES]
        for entry in (verifier.ROOT / 'integration').glob(f'*/{records.ENTRY_FILE}'):
            files.append(entry)
            data = json.loads(entry.read_bytes())
            if data['kind'] == 'inverse':
                files.append(verifier.ROOT / data['manifest'])
        for path in files:
            target = root / path.relative_to(verifier.ROOT)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, target)
        return root / core

    def equipment(self, root, name):
        return records.discover_entries(root, verifier.protected_core_pins(root))[f'equipment-{name}']

    def assert_inverse_remains_pinned(self, name, manifest_name, expected_count, message):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = self.copy_protected_sources(root)
            inverse = self.equipment(root, name).files
            self.assertEqual(len(inverse), expected_count)
            for relative in inverse:
                with self.subTest(relative=relative):
                    source = core / relative
                    data = source.read_bytes()
                    source.write_bytes(data + b'\n')
                    with self.assertRaisesRegex(AssertionError, relative):
                        verifier.verify_protected_core(root)
                    source.write_bytes(data)
            manifest = root / 'integration/sh001-mule' / manifest_name
            manifest.write_bytes(manifest.read_bytes() + b'\n')
            with self.assertRaisesRegex(AssertionError, message + ' inverse manifest'):
                verifier.verify_protected_core(root)

    def test_bundled_assets_core_inverse_and_reference_bindings_agree(self):
        result = verifier.verify()
        self.assertEqual(result['protectedCoreFilesChecked'], 115)
        self.assertEqual(result['protectedCoreFilesWithMuleHooks'], 6)
        self.assertEqual(result['protectedCoreFilesWithFlightComputerChanges'], 17)
        self.assertEqual(result['protectedCoreFilesWithHyperspaceDriveChanges'], 12)
        self.assertEqual(result['protectedCoreFilesWithStockShieldChanges'], 16)
        self.assertEqual(result['protectedCoreFilesWithFuelStorageChanges'], 22)
        self.assertEqual(result['protectedCoreFilesWithSolarScoopsChanges'], 16)
        self.assertEqual(result['protectedCoreFilesWithStockEcmChanges'], 16)
        self.assertEqual(result['protectedCoreFilesWithStockEcmFieldChanges'], 12)
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
            inverse = self.equipment(root, 'flight-computer').files
            relative = next(name for name in verifier.protected_core_pins(root) if name not in inverse)
            source = core / relative
            source.write_bytes(source.read_bytes() + b'\n')
            with self.assertRaisesRegex(AssertionError, relative):
                verifier.verify_protected_core(root)

    def test_altered_inverse_cannot_authorize_an_unrelated_edit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.copy_protected_sources(root)
            inverse = root / 'integration/sh001-mule/FLIGHT-COMPUTER-INVERSE.json'
            inverse.write_bytes(inverse.read_bytes() + b'\n')
            with self.assertRaisesRegex(AssertionError, 'flight computer inverse manifest'):
                verifier.verify_protected_core(root)

    def test_hyperspace_inverse_is_pinned_and_each_changed_source_remains_exact(self):
        self.assert_inverse_remains_pinned('hyperspace-drive', 'HYPERSPACE-DRIVE-INVERSE.json', 12,
                                           'hyperspace drive')

    def test_stock_shield_inverse_is_pinned_and_each_changed_source_remains_exact(self):
        self.assert_inverse_remains_pinned('stock-shield', 'STOCK-SHIELD-INVERSE.json', 16,
                                           'stock shield')

    def test_component_triangle_may_not_use_a_neighbouring_part_vertex(self):
        part = dict(index=0, name='body primary pressure envelope', vertexStart=5,
                    vertexCount=3, faceStart=0, faceCount=1)
        assigned = dict(part, component=0, piece=0)
        verifier.verify_component('design', part, assigned, [(5, 6, 7)], {})
        with self.assertRaises(AssertionError):
            verifier.verify_component('design', part, assigned, [(5, 6, 8)], {})

    def test_fuel_storage_inverse_is_pinned_and_each_changed_source_remains_exact(self):
        self.assert_inverse_remains_pinned('fuel-storage', 'FUEL-STORAGE-INVERSE.json', 22,
                                           'fuel storage')

    def test_solar_scoops_inverse_is_pinned_and_each_changed_source_remains_exact(self):
        self.assert_inverse_remains_pinned('solar-scoops', 'SOLAR-SCOOPS-INVERSE.json', 16,
                                           'solar scoops')

    def test_hull_publications_must_each_be_removed_exactly_once(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = self.copy_protected_sources(root)
            source = core / verifier.LIB_SOURCE
            original = source.read_bytes()
            entries = records.discover_entries(root, verifier.protected_core_pins(root))
            declarations = [entry.declaration.line for entry in entries.values() if entry.declaration]
            for publication in [b'pub mod stag_liner;\n\n', *declarations]:
                for altered in (original.replace(publication, b''), original + publication):
                    with self.subTest(publication=publication):
                        source.write_bytes(altered)
                        with self.assertRaisesRegex(AssertionError, 'lib.rs'):
                            verifier.verify_protected_core(root)
            for declaration in declarations:
                with self.subTest(relocated=declaration):
                    source.write_bytes(original.replace(declaration, b'') + declaration)
                    with self.assertRaisesRegex(AssertionError, 'lib.rs'):
                        verifier.verify_protected_core(root)

    def test_stock_ecm_inverse_is_pinned_and_every_changed_source_remains_exact(self):
        self.assert_inverse_remains_pinned('stock-ecm', 'STOCK-ECM-INVERSE.json', 16,
                                           'stock ECM')

    def test_stock_ecm_field_inverse_is_pinned_and_every_changed_source_remains_exact(self):
        self.assert_inverse_remains_pinned('stock-ecm-field', 'STOCK-ECM-FIELD-INVERSE.json', 12,
                                           'stock ECM field')

    def test_scanner_preview_inverse_restores_the_main_hull_publications_first(self):
        root = verifier.ROOT
        baseline = verifier.protected_core_pins(root)
        entries = records.discover_entries(root, baseline)
        plan = records.order_entries(entries)
        scanner = entries['scanner-preview']
        self.assertEqual(len(scanner.files), 40)
        names = [entry.name for entry in plan]
        self.assertLess(names.index('scanner-preview'), names.index('sh066-rabbit-explorer'))
        self.assertLess(names.index('sh066-rabbit-explorer'), names.index('sh065-vulture-raider'))
        self.assertLess(names.index('sh065-vulture-raider'), names.index('sh064-arna-cutter'))
        current = (root / records.CORE_SOURCE / verifier.LIB_SOURCE).read_bytes()
        main = verifier.restore_entries(verifier.LIB_SOURCE, current, plan[:names.index('sh066-rabbit-explorer')])
        for name in ('sh066-rabbit-explorer', 'sh065-vulture-raider', 'sh064-arna-cutter'):
            self.assertEqual(main.count(entries[name].declaration.line), 1)
        self.assertEqual(records.sha(main), scanner.files[verifier.LIB_SOURCE]['beforeSHA256'])

    def test_scanner_preview_inverse_rejects_source_drift_wrong_parent_and_forged_preimage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = self.copy_protected_sources(root)
            baseline = verifier.protected_core_pins(root)
            entries = records.discover_entries(root, baseline)
            scanner = entries['scanner-preview']
            for relative in scanner.files:
                with self.subTest(relative=relative):
                    path = core / relative
                    original = path.read_bytes()
                    path.write_bytes(original + b'\n')
                    with self.assertRaisesRegex(AssertionError, relative):
                        verifier.verify_protected_core(root)
                    path.write_bytes(original)

            folder = root / 'integration/scanner-preview'
            manifest = folder / records.INVERSE_FILE
            raw = manifest.read_bytes()
            manifest.write_bytes(raw + b'\n')
            with self.assertRaisesRegex(AssertionError, 'scanner preview integration inverse manifest'):
                verifier.verify_protected_core(root)

            inverse = json.loads(raw)
            entry_path = folder / records.ENTRY_FILE
            entry_raw = entry_path.read_bytes()
            entry = json.loads(entry_raw)
            self.assertEqual(entry['base'], '344b2bf1b46bcf7c2ab62094e7200f8abc7da9c3')
            inverse['base'] = '98ebd952a2b76f2c9dba424ff77037381cffb7a0'
            changed = json.dumps(inverse).encode('utf-8')
            manifest.write_bytes(changed)
            entry['sha256'] = records.sha(changed)
            entry_path.write_bytes(json.dumps(entry).encode('utf-8'))
            with self.assertRaisesRegex(AssertionError, 'scanner preview integration inverse base'):
                verifier.verify_protected_core(root)

            manifest.write_bytes(raw)
            entry_path.write_bytes(entry_raw)
            relative = next(iter(scanner.files))
            plan = records.order_entries(entries)
            current = verifier.restore_entries(relative, (core / relative).read_bytes(),
                                               plan[:plan.index(scanner)])
            before = scanner.undo(relative, current)
            forged = {**scanner.files, relative: generator.file_inverse(before + b'\n', current)}
            approved = {**entries, scanner.name: replace(scanner, files=forged)}
            with patch.object(verifier, 'discover_entries', return_value=approved):
                with self.assertRaisesRegex(AssertionError, relative):
                    verifier.verify_protected_core(root)

    def test_historical_scanner_preview_receipts_are_preserved_separately_from_main(self):
        path = verifier.ROOT / 'integration/scanner-preview/HISTORICAL-PREVIEW.json'
        raw = path.read_bytes()
        self.assertEqual(records.sha(raw), '70fb923e8c797f8df3882951605e812312dceec8bfab796e200f9f20b7fee6b8')
        binding = json.loads(raw)
        self.assertEqual(binding['protectedFiles'], 115)
        self.assertEqual(len(binding['receipts']), 8)
        for name, expected in binding['receipts'].items():
            with self.subTest(receipt=name):
                self.assertEqual(records.sha((verifier.ROOT / name).read_bytes()), expected)
        scanner = json.loads((verifier.ROOT / 'integration/sh001-mule/SCANNER-TARGET-CYCLE-INVERSE.json').read_bytes())
        self.assertEqual(scanner['base'], '98ebd952a2b76f2c9dba424ff77037381cffb7a0')
        self.assertEqual(len(scanner['files']), 6)

    def test_historical_stag_check_still_refuses_a_forged_preimage(self):
        # Simulate even a mistakenly approved new inverse with bad historical bytes.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core = self.copy_protected_sources(root)
            relative = verifier.LIB_SOURCE
            entries = records.discover_entries(root, verifier.protected_core_pins(root))
            plan = records.order_entries(entries)
            flight_computer = entries['equipment-flight-computer']
            current = verifier.restore_entries(relative, (core / relative).read_bytes(),
                                               plan[:plan.index(flight_computer)])
            before = flight_computer.undo(relative, current)
            addition = b'pub mod stag_liner;\n\n'
            self.assertEqual(before.count(addition), 1)
            for altered in (before.replace(addition, b''), before + addition):
                files = {**flight_computer.files, relative: generator.file_inverse(altered, current)}
                approved = {**entries, flight_computer.name: replace(flight_computer, files=files)}
                with patch.object(verifier, 'discover_entries', return_value=approved):
                    with self.assertRaisesRegex(AssertionError, 'lib.rs'):
                        verifier.verify_protected_core(root)

    def test_each_protected_file_restores_to_its_immutable_blob(self):
        root = verifier.ROOT
        baseline = verifier.protected_core_pins(root)
        plan = records.order_entries(records.discover_entries(root, baseline))
        self.assertEqual(len(baseline), 115)
        for relative, digest in baseline.items():
            with self.subTest(path=relative):
                current = (root / records.CORE_SOURCE / relative).read_bytes()
                data = verifier.restore_historical(relative, verifier.restore_entries(relative, current, plan))
                self.assertEqual(records.sha(data), digest)


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
                self.assertEqual(call.args[0], ['git', 'cat-file', 'blob', '--',
                    verifier.FLIGHT_COMPUTER_BASE + ':crates/meridian-core/src/' + relative])

    def test_generator_rejects_noncanonical_base_before_git_or_reading_pins(self):
        invalid_bases = (None, '', 'a' * 39, 'a' * 41, 'A' * 40, 'g' * 40,
                         '-' + 'a' * 39, 'a' * 39 + '\n')
        with patch.object(generator, 'protected_core_pins') as pins, \
             patch.object(generator.subprocess, 'check_output') as git:
            for base in invalid_bases:
                with self.subTest(base=base), self.assertRaisesRegex(ValueError, 'Git commit SHA'):
                    generator.generate(verifier.ROOT, base)
            pins.assert_not_called()
            git.assert_not_called()
