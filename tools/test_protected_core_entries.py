"""Check protected entry discovery, exact replay and dependency failures."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import generate_flight_computer_inverse as generator
import protected_core_entries as records


class EntryDiscovery(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.protected = {'lib.rs': '', 'flight.rs': ''}
        core = self.root / records.CORE_SOURCE
        core.mkdir(parents=True)
        self.frozen = {}
        self.freeze_entries()

    def freeze_entries(self):
        index = self.root / records.FROZEN_ENTRIES
        index.parent.mkdir(parents=True, exist_ok=True)
        raw = json.dumps({'schema': 1, 'entries': self.frozen}).encode()
        index.write_bytes(raw)
        self.index_pin = records.sha(raw)

    def entry(self, name, **fields):
        path = self.root / 'integration' / name / records.ENTRY_FILE
        path.parent.mkdir(parents=True, exist_ok=True)
        fields = {'schema': 1, 'id': name, 'undoBefore': [], 'undoAfter': [], **fields}
        path.write_text(json.dumps(fields), encoding='utf-8')
        return path

    def declare(self, name, module, before='pub mod start;\n', after='pub mod end;\n', **fields):
        (self.root / records.CORE_SOURCE / f'{module}.rs').touch()
        return self.entry(name, kind='declaration', module=module, before=before, after=after, **fields)

    def discover(self):
        with patch.object(records, 'FROZEN_SHA256', self.index_pin):
            return records.discover_entries(self.root, self.protected)

    def test_frozen_inventory_requires_original_owner_and_bytes(self):
        entry = self.declare('one', 'one')
        original = entry.read_bytes()
        self.frozen['one'] = records.sha(original)
        self.freeze_entries()
        self.assertEqual(set(self.discover()), {'one'})

        entry.unlink()
        with self.assertRaisesRegex(AssertionError, 'missing frozen protected entries'):
            self.discover()
        self.declare('other', 'other').write_bytes(original)
        with self.assertRaisesRegex(AssertionError, 'other'):
            self.discover()
        (self.root / 'integration/other' / records.ENTRY_FILE).unlink()

        entry.write_bytes(original.replace(b'pub mod start', b'pub mod forged'))
        with self.assertRaisesRegex(AssertionError, 'frozen protected entry'):
            self.discover()
        entry.write_bytes(original)
        self.frozen.clear()
        self.freeze_entries()
        with patch.object(records, 'FROZEN_SHA256', '0' * 64):
            with self.assertRaisesRegex(AssertionError, 'frozen protected entry index'):
                records.discover_entries(self.root, self.protected)

    def test_duplicate_declaration_and_json_keys_refuse(self):
        self.declare('one', 'one')
        second = self.declare('two', 'one')
        with self.assertRaisesRegex(AssertionError, 'duplicate protected declaration'):
            self.discover()
        second.write_bytes(b'{"id":"two","id":"one"}')
        with self.assertRaisesRegex(AssertionError, 'duplicate JSON key: id'):
            self.discover()

    def test_missing_current_module_refuses(self):
        self.declare('one', 'one')
        module = self.root / records.CORE_SOURCE / 'one.rs'
        module.unlink()
        with self.assertRaisesRegex(AssertionError, 'missing or ambiguous module'):
            self.discover()

    def test_linked_entry_refuses(self):
        entry = self.declare('one', 'one')
        other = entry.with_name('file.json')
        entry.rename(other)
        try:
            entry.symlink_to(other)
        except OSError as error:
            self.skipTest(f'symlink unavailable: {error}')
        with self.assertRaisesRegex(AssertionError, 'symlink in protected binding'):
            self.discover()

    def test_new_inverse_is_owner_bound_and_byte_pinned(self):
        entry = self.root / 'integration/flight-upgrade' / records.INVERSE_FILE
        entry.parent.mkdir(parents=True)
        base = '1' * 40
        old = b'old\n'
        new = b'new\n'
        manifest = {'base': base, 'files': {'flight.rs': generator.file_inverse(old, new)}}
        raw = json.dumps(manifest).encode()
        entry.write_bytes(raw)
        record = dict(kind='inverse', manifest=entry.relative_to(self.root).as_posix(),
                      sha256=records.sha(raw), label='flight upgrade', base=base)
        self.entry('flight-upgrade', **record)
        plan = records.order_entries(self.discover())
        self.assertEqual(plan[0].undo('flight.rs', new), old)
        with self.assertRaisesRegex(AssertionError, 'flight.rs'):
            plan[0].undo('flight.rs', b'another state\n')

        entry.write_bytes(raw + b'\n')
        with self.assertRaisesRegex(AssertionError, 'flight upgrade inverse manifest'):
            self.discover()
        entry.write_bytes(raw)
        self.entry('flight-upgrade', **{**record, 'base': '2' * 40})
        with self.assertRaisesRegex(AssertionError, 'flight upgrade inverse base'):
            self.discover()
        self.entry('flight-upgrade', **{**record, 'manifest': 'integration/another/PROTECTED-CORE-INVERSE.json'})
        with self.assertRaisesRegex(AssertionError, 'owned by the feature'):
            self.discover()


class EntryOrdering(unittest.TestCase):
    def declaration(self, name, before, after, undo_before=(), undo_after=()):
        rule = records.Declaration(before, f'pub mod {name};\n'.encode(), after)
        return records.Entry(name, undo_before, undo_after, {}, rule)

    def inverse(self, name, path, before, after, undo_before=(), undo_after=()):
        files = {path: generator.file_inverse(before, after)}
        return records.Entry(name, undo_before, undo_after, files)

    def order(self, *entries):
        return records.order_entries({entry.name: entry for entry in entries})

    def test_fails_for_missing_duplicate_self_opposing_or_cyclic_dependencies(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            record = dict(schema=1, id='owner', kind='declaration', module='one',
                          before='start\n', after='end\n', undoBefore=['two', 'two'], undoAfter=[])
            with self.assertRaisesRegex(AssertionError, 'duplicate dependency'):
                records.parse_entry(root, {}, 'owner', record, True)
            record['undoBefore'] = ['owner']
            with self.assertRaisesRegex(AssertionError, 'invalid dependency'):
                records.parse_entry(root, {}, 'owner', record, True)
            record['undoBefore'] = record['undoAfter'] = ['two']
            with self.assertRaisesRegex(AssertionError, 'invalid dependency'):
                records.parse_entry(root, {}, 'owner', record, True)

        first = self.declaration('first', b'a\n', b'b\n', undo_before=('second',))
        with self.assertRaisesRegex(AssertionError, 'missing undo dependency second'):
            self.order(first)
        second = self.declaration('second', b'c\n', b'd\n', undo_before=('first',))
        with self.assertRaisesRegex(AssertionError, 'cyclic protected undo dependencies'):
            self.order(first, second)

    def test_independent_declarations_can_share_an_outer_anchor(self):
        left = self.declaration('left', b'a\n', b'b\n')
        right = self.declaration('right', b'b\n', b'c\n')
        source = b'a\npub mod left;\nb\npub mod right;\nc\n'
        self.assertEqual(self.order(left, right), self.order(right, left))
        self.assertEqual(left.undo('lib.rs', right.undo('lib.rs', source)), b'a\nb\nc\n')
        self.assertEqual(right.undo('lib.rs', left.undo('lib.rs', source)), b'a\nb\nc\n')

    def test_same_gap_and_consumed_context_need_explicit_order(self):
        anchor = b'pub mod navigation;\n'
        end = b'pub mod saiga;\n'
        rabbit = self.declaration('rabbit', anchor, end)
        rhino = self.declaration('rhino', rabbit.declaration.line, end)
        with self.assertRaisesRegex(AssertionError, 'ambiguous protected undo order'):
            self.order(rhino, rabbit)
        competing = self.declaration('competing', anchor, end)
        with self.assertRaisesRegex(AssertionError, 'ambiguous protected undo order'):
            self.order(competing, rabbit)
        rhino = self.declaration('rhino', rabbit.declaration.line, end, undo_before=('rabbit',))
        with_rabbit = anchor + rabbit.declaration.line + end
        source = anchor + rabbit.declaration.line + rhino.declaration.line + end
        self.assertEqual(self.order(rabbit, rhino), (rhino, rabbit))
        self.assertEqual(rhino.undo('lib.rs', source), with_rabbit)
        self.assertEqual(rabbit.undo('lib.rs', with_rabbit), anchor + end)
        with self.assertRaisesRegex(AssertionError, 'lib.rs'):
            rabbit.undo('lib.rs', source)

    def test_whole_blob_overlap_requires_order_and_checks_intermediate_bytes(self):
        old, intermediate, current = b'old\n', b'intermediate\n', b'current\n'
        first = self.inverse('first', 'flight.rs', old, intermediate)
        second = self.inverse('second', 'flight.rs', intermediate, current)
        separate = self.inverse('separate', 'lib.rs', b'left\n', b'right\n')
        self.assertEqual(set(entry.name for entry in self.order(first, separate)), {'first', 'separate'})
        with self.assertRaisesRegex(AssertionError, 'ambiguous protected undo order'):
            self.order(first, second)
        first = self.inverse('first', 'flight.rs', old, intermediate, undo_after=('second',))
        self.assertEqual(self.order(first, second), (second, first))
        self.assertEqual(second.undo('flight.rs', current), intermediate)
        self.assertEqual(first.undo('flight.rs', intermediate), old)
        with self.assertRaisesRegex(AssertionError, 'flight.rs'):
            first.undo('flight.rs', current)

        forged = self.inverse('second', 'flight.rs', b'forged\n', current)
        self.assertEqual(self.order(forged, first), (forged, first))
        with self.assertRaisesRegex(AssertionError, 'flight.rs'):
            first.undo('flight.rs', forged.undo('flight.rs', current))
