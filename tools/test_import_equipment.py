"""Independent original equipment content/fixture checks; no core or native runtime tests."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('import_equipment', ROOT / 'tools/import_equipment.py')
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


def manifest():
    return json.loads((ROOT / 'assets/legacy/equipment-content-manifest.json').read_bytes())


def reference():
    return json.loads((ROOT / 'tools/fixtures/sh025-cargo-bay.reference.json').read_bytes())


def schema_inputs():
    # Parsed-schema fixtures are intentionally small, not provenance or a native oracle.
    authoring = {'version': 'l337-world-1', 'equipment': [
        {'id': 'cargo-bay', 'name': 'Large Cargo Bay', 'priceCR': 2800.0, 'technology': 4}]}
    ship = {'id': 'SH025', 'game': {'cargo': 92, 'cargoExpansion': 35}, 'profiles': [
        {'group': 'project-refit-v1', 'fields': [
            {'key': 'cargo_expansion_tonnes', 'number': 35},
            {'key': 'equipment_capacity_units', 'number': 24}]}]}
    r3 = {'id': 'SH025', 'schema': 'l337.fleet-design/2',
          'decisionStatus': 'source schema fixture, not gameplay acceptance',
          'performance': {'cargoTonnes': 92, 'cargoExpansionTonnes': 35},
          'fitting': {'roleKitIncludedInBareHull': False, 'utilityMounts': 3, 'equipmentBudgetUnits': 24}}
    worlds = {'version': 'l337-world-1', 'systems': [
        {'key': 'G01-S007', 'galaxy': 0, 'system': 7, 'name': 'Avenor', 'technology': 12},
        {'key': 'G01-S185', 'galaxy': 0, 'system': 185, 'name': 'Renas', 'technology': 13}]}
    worlds['systems'].extend({'key': 'unselected'} for _ in range(2046))
    return authoring, ship, r3, worlds


def toml_bytes(document):
    # Only encode the bounded schema fixture; production parses the actual pinned TOMLs.
    if 'equipment' in document:
        row = document['equipment'][0]
        return ('version = "l337-world-1"\n[[equipment]]\n' + '\n'.join(
            key + ' = ' + json.dumps(value) for key, value in row.items()) + '\n').encode()
    if 'game' in document:
        return b'id="SH025"\n[game]\ncargo=92\ncargoExpansion=35\n[[profiles]]\ngroup="project-refit-v1"\n[[profiles.fields]]\nkey="cargo_expansion_tonnes"\nnumber=35\n[[profiles.fields]]\nkey="equipment_capacity_units"\nnumber=24\n'
    return b'id="SH025"\nschema="l337.fleet-design/2"\ndecisionStatus="source schema fixture, not gameplay acceptance"\n[performance]\ncargoTonnes=92\ncargoExpansionTonnes=35\n[fitting]\nroleKitIncludedInBareHull=false\nutilityMounts=3\nequipmentBudgetUnits=24\n'


def schema_blobs():
    authoring, ship, r3, worlds = schema_inputs()
    raw = {key: '\n'.join(IMPORTER.WITNESSES.get(key, [])).encode() for key in IMPORTER.SOURCES}
    raw.update({'authoring': toml_bytes(authoring), 'ship': toml_bytes(ship),
                'r3': toml_bytes(r3), 'worlds': json.dumps(worlds).encode()})
    raw['game'] += b'\n.init(name: "Large Cargo Bay", price: 28000, technology: 3)'
    r3_sha = hashlib.sha256(raw['r3']).hexdigest()
    raw['fleet'] = ('private static let sh025: fixture\nsourceSHA256: "' + r3_sha
                    + '"\ncargo: 92, cargoExpansion: 35, equipmentUnits: 24, utilityMounts: 3\n'
                    + 'private static let sh026: next').encode()
    pins = {key: (IMPORTER.SOURCES[key][0], hashlib.sha256(value).hexdigest()) for key, value in raw.items()}
    return raw, pins


class EquipmentImportTests(unittest.TestCase):
    def test_independent_binary_decoder_checks_every_field_actual_site_and_exact_end(self):
        raw = (ROOT / 'assets/legacy/sh025-cargo-bay.equipment').read_bytes()
        self.assertEqual(len(raw), 107)
        self.assertEqual(hashlib.sha256(raw).hexdigest(), '2e99546d65fec686e65275599c0ac529a17551655678fd65e4b794e74104b83d')
        self.assertEqual(struct.unpack_from('<8sIBHHBHBB', raw),
                         (b'MEQP0001', 28000, 4, 92, 35, 3, 24, 1, 1))
        offset = 22
        for expected in ['cargo-bay', 'Large Cargo Bay', 'utility-cargoExpansion', 'utility.cargoExpansion', 'SH025']:
            length = raw[offset]
            offset += 1
            self.assertEqual(raw[offset:offset + length].decode(), expected)
            offset += length
        self.assertEqual(raw[offset], 2)
        offset += 1
        for expected in [(7, 12), (185, 13)]:
            self.assertEqual(struct.unpack_from('<HB', raw, offset), expected)
            offset += 3
        self.assertEqual(offset, len(raw))
        self.assertEqual(manifest()['definition'], reference()['definition'])

    def test_source_output_origin_receipt_and_notice_are_bound(self):
        meta = manifest()
        self.assertEqual(meta['sourceRevision'], IMPORTER.REVISION)
        self.assertEqual(meta['sourceTree'], IMPORTER.TREE)
        for key, pin in IMPORTER.SOURCES.items():
            self.assertEqual((meta['sources'][key]['path'], meta['sources'][key]['sha256']), pin)
        raw = (ROOT / meta['output']).read_bytes()
        self.assertEqual((len(raw), hashlib.sha256(raw).hexdigest()), (meta['outputBytes'], meta['outputSHA256']))
        origin = meta['origin']
        receipt = (ROOT / origin['equipmentBudgetsAudit']).read_bytes()
        self.assertEqual(hashlib.sha256(receipt).hexdigest(), origin['equipmentBudgetsAuditSHA256'])
        self.assertTrue((ROOT / meta['notice']).is_file())
        self.assertIn('implementation-and-playtesting-pending', origin['preservedR3DecisionStatus'])

    def test_fixed_input_set_and_blob_allowlist_refuse_before_subprocess(self):
        with self.assertRaisesRegex(ValueError, 'Incomplete'):
            IMPORTER.convert({})
        with self.assertRaisesRegex(ValueError, 'hash mismatch'):
            IMPORTER.convert({key: b'foreign' for key in IMPORTER.SOURCES})
        for identifier in ['../equipment', '--output=unexpected', 'game;echo injected']:
            with patch.object(IMPORTER.subprocess, 'check_output') as call:
                with self.assertRaisesRegex(ValueError, 'Unselected'):
                    IMPORTER.read_blob(ROOT, identifier)
                call.assert_not_called()

    def test_pinned_git_command_uses_cwd_disables_helpers_and_authenticates_each_blob(self):
        value = b'isolated object fixture'
        with tempfile.TemporaryDirectory(prefix='--equipment-source-') as directory:
            repository = Path(directory).resolve()
            for key, (path, _) in IMPORTER.SOURCES.items():
                with patch.dict(IMPORTER.SOURCES, {key: (path, hashlib.sha256(value).hexdigest())}):
                    with patch.object(IMPORTER.subprocess, 'check_output', return_value=value) as call:
                        self.assertEqual(IMPORTER.read_blob(repository, key), value)
                        call.assert_called_once_with(
                            ['git', '--no-pager', 'show', '--no-ext-diff', '--no-textconv', IMPORTER.REVISION + ':' + path],
                            cwd=repository)
        with patch.object(IMPORTER.subprocess, 'check_output', return_value=b'wrong') as call:
            with self.assertRaisesRegex(ValueError, 'directory'):
                IMPORTER.read_blob(Path(__file__), 'game')
            call.assert_not_called()
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                IMPORTER.read_blob(ROOT, 'game')

    def test_original_definition_reconciles_exact_price_and_zero_to_one_based_technology(self):
        authoring, _, _, _ = schema_inputs()
        game = '.init(name: "Large Cargo Bay", price: 28000, technology: 3)'
        result = IMPORTER.selected_definition(authoring, game)
        self.assertEqual((result['priceTenthsCR'], result['requiredTechnologyLevel']), (28000, 4))
        for field, value in [('priceCR', True), ('priceCR', 2800.01), ('priceCR', float('nan')),
                             ('priceCR', -1), ('priceCR', 2 ** 32), ('technology', True),
                             ('name', 'Replacement Bay')]:
            broken = copy.deepcopy(authoring)
            broken['equipment'][0][field] = value
            with self.subTest(field=field, value=value):
                with self.assertRaises(ValueError):
                    IMPORTER.selected_definition(broken, game)
        for replacement in [game.replace('28000', '4000'), game.replace('technology: 3', 'technology: 4'), game + game]:
            with self.assertRaisesRegex(ValueError, 'differs'):
                IMPORTER.selected_definition(authoring, replacement)
        authoring['equipment'].append(copy.deepcopy(authoring['equipment'][0]))
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            IMPORTER.selected_definition(authoring, game)

    def test_hull_capacity_matches_original_r3_active_mechanics_profile_and_runtime(self):
        raw, pins = schema_blobs()
        _, ship, r3, _ = schema_inputs()
        with patch.dict(IMPORTER.SOURCES, pins):
            fields = IMPORTER.hull_fields(ship, r3, raw['fleet'].decode())
            self.assertEqual((fields['baseHoldTonnes'], fields['extraHoldTonnes']), (92, 35))
            for field, value in [('cargoExpansion', 0), ('cargo', True), ('cargo', 91)]:
                broken = copy.deepcopy(ship)
                broken['game'][field] = value
                with self.subTest(field=field):
                    with self.assertRaises(ValueError):
                        IMPORTER.hull_fields(broken, r3, raw['fleet'].decode())
            broken = copy.deepcopy(ship)
            broken['profiles'][0]['fields'][0]['number'] = 36
            with self.assertRaisesRegex(ValueError, 'profile'):
                IMPORTER.hull_fields(broken, r3, raw['fleet'].decode())
            for field, value in [('roleKitIncludedInBareHull', True), ('utilityMounts', 0)]:
                broken = copy.deepcopy(r3)
                broken['fitting'][field] = value
                with self.assertRaises(ValueError):
                    IMPORTER.hull_fields(ship, broken, raw['fleet'].decode())
            with self.assertRaisesRegex(ValueError, 'runtime fitting'):
                IMPORTER.hull_fields(ship, r3, 'unbound SH025')

    def test_real_station_selection_refuses_missing_duplicate_foreign_or_boolean_id(self):
        _, _, _, worlds = schema_inputs()
        self.assertEqual([(s['system'], s['technology']) for s in IMPORTER.selected_sites(worlds)], [(7, 12), (185, 13)])
        for field, value in [('key', 'G02-S007'), ('galaxy', False), ('system', True),
                             ('name', 'Foreign station'), ('technology', True), ('technology', 0)]:
            broken = copy.deepcopy(worlds)
            broken['systems'][0][field] = value
            with self.subTest(field=field):
                with self.assertRaises(ValueError):
                    IMPORTER.selected_sites(broken)
        worlds['systems'][2] = copy.deepcopy(worlds['systems'][0])
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            IMPORTER.selected_sites(worlds)

    def test_each_behavior_witness_is_required_and_records_actual_source_line(self):
        raw, _ = schema_blobs()
        evidence = IMPORTER.witness_lines(raw)
        for key, lines in evidence.items():
            source = raw[key].decode().splitlines()
            for witness in lines:
                self.assertIn(witness['text'], source[witness['line'] - 1])
            broken = dict(raw)
            broken[key] = b'unsupported semantics'
            with self.subTest(key=key):
                with self.assertRaisesRegex(ValueError, 'witness'):
                    IMPORTER.witness_lines(broken)

    def test_full_parsed_schema_conversion_keeps_exact_binary_and_original_constraints(self):
        raw, pins = schema_blobs()
        with patch.dict(IMPORTER.SOURCES, pins):
            outputs = IMPORTER.convert(raw)
        self.assertEqual(outputs['sh025-cargo-bay.equipment'], (ROOT / 'assets/legacy/sh025-cargo-bay.equipment').read_bytes())
        meta = json.loads(outputs['equipment-content-manifest.json'])
        self.assertEqual(meta['definition'], reference()['definition'])
        self.assertEqual(meta['starterAdmission']['before'], {'equipmentUnits': 0, 'utilityMounts': 0})
        self.assertEqual(meta['starterAdmission']['withPaidFittedCargoBay'], {'equipmentUnits': 1, 'utilityMounts': 1})
        definition = dict(meta['definition'])
        definition['name'] = 'x' * 256
        with self.assertRaisesRegex(ValueError, 'text length'):
            IMPORTER.binary(definition, meta['sites'])

    def test_independent_reference_cases_preserve_payment_tl_mount_and_custody_boundaries(self):
        cases = reference()
        definition = cases['definition']
        # Check source-derived numeric boundaries against the independently decoded contract.
        # These calculations do not exercise or replace G3's actual World transactions.
        self.assertEqual(cases['sourceRevision'], IMPORTER.REVISION)
        self.assertEqual(len(cases['starterItems']), 8)
        self.assertEqual(sum(x['equipmentUnits'] for x in cases['starterItems']), 0)
        self.assertEqual(sum(x['utilityMounts'] for x in cases['starterItems']), 0)
        for case in cases['budgetCases']:
            fitted = int(case['bayFitted'])
            self.assertEqual(case['expectedEquipmentUnits'], fitted * definition['itemEquipmentUnitCost'])
            self.assertEqual(case['expectedUtilityMounts'], fitted * definition['itemUtilityMountCost'])
            self.assertEqual(case['expectedCapacityTonnes'], 92 + fitted * 35)
            self.assertLessEqual(case['expectedEquipmentUnits'], 24)
            self.assertLessEqual(case['expectedUtilityMounts'], 3)
        for case in cases['technologyCases']:
            self.assertEqual(case['eligibleByTechnology'], case['technology'] >= 4)
        for case in cases['purchaseCases']:
            allowed = case['creditsBefore'] >= 28000 and not case.get('bayAlreadyOwned', False)
            self.assertEqual(case['allowed'], allowed)
            self.assertEqual(case['expectedCreditsAfter'], case['creditsBefore'] - (28000 if allowed else 0))
            self.assertEqual(case['expectedNewItems'], int(allowed))
        for case in cases['unfitCases']:
            allowed = case['ordinaryTonnes'] + case['identifiedMissionTonnes'] <= 92
            self.assertEqual(case['allowed'], allowed)
            self.assertEqual(case['expectedCapacityTonnes'], 92 if allowed else 127)
        identity = cases['identityCases'][0]
        self.assertEqual((identity['cashDelta'], identity['itemsDelta'], identity['capacitySequenceTonnes']), (0, 0, [127, 92, 127]))

    def test_cli_checks_all_owned_outputs_and_never_overwrites_tampered_files(self):
        raw, pins = schema_blobs()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / 'assets/legacy'
            output.mkdir(parents=True)
            with patch.object(IMPORTER, 'ROOT', root), patch.dict(IMPORTER.SOURCES, pins):
                with patch.object(IMPORTER, 'read_blob', side_effect=lambda _repo, key: raw[key]):
                    with patch.object(sys, 'argv', ['import_equipment', '.']):
                        IMPORTER.main()
                    with patch.object(sys, 'argv', ['import_equipment', '.', '--check']):
                        IMPORTER.main()
                    (output / 'equipment-content-manifest.json').write_bytes(b'tampered')
                    before = {p.name: p.read_bytes() for p in output.iterdir()}
                    with patch.object(sys, 'argv', ['import_equipment', '.', '--check']):
                        with self.assertRaisesRegex(ValueError, 'mismatch'):
                            IMPORTER.main()
                    self.assertEqual({p.name: p.read_bytes() for p in output.iterdir()}, before)


if __name__ == '__main__':
    unittest.main()
