"""Discover and replay feature-owned exact inverses for protected core sources."""
from dataclasses import dataclass
from graphlib import CycleError, TopologicalSorter
from hashlib import sha256
from itertools import combinations
import json
from pathlib import Path
import re


if not __debug__:
    raise RuntimeError("Protected source verification requires Python assertions")

CORE_SOURCE = Path('crates/meridian-core/src')
LIB_SOURCE = 'lib.rs'
ENTRY_FILE = 'PROTECTED-CORE-ENTRY.json'
INVERSE_FILE = 'PROTECTED-CORE-INVERSE.json'
FROZEN_ENTRIES = Path('integration/sh001-mule/FROZEN-PROTECTED-CORE-ENTRIES.json')
FROZEN_SHA256 = '5261ff703823d06bce2cd27b1cfe12f7f0e1224bd7454bd25fdd98d098031823'
OWNER = re.compile(r'[a-z][a-z0-9]*(?:-[a-z0-9]+)*')
MODULE = re.compile(r'[a-z][a-z0-9_]*')
HEX_DIGEST = re.compile(r'[0-9a-f]{64}')
COMMIT = re.compile(r'[0-9a-f]{40}')


def sha(data):
    return sha256(data).hexdigest()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        assert key not in result, f'duplicate JSON key: {key}'
        result[key] = value
    return result


def read_bytes(root, relative):
    path = root
    for part in relative.parts:
        path = path / part
        assert not path.is_symlink(), f'symlink in protected binding: {relative}'
    return path.read_bytes()


def read_inverse(root, protected, path, expected_sha, base, label):
    manifest = read_bytes(root, path)
    assert sha(manifest) == expected_sha, label + ' inverse manifest'
    inverse = json.loads(manifest, object_pairs_hook=unique_object)
    assert set(inverse) == {'base', 'files'} and inverse['base'] == base, label + ' inverse base'
    files = inverse['files']
    assert isinstance(files, dict) and files and files.keys() <= protected.keys(), label + ' inverse paths'
    return files


@dataclass(frozen=True)
class Declaration:
    before: bytes
    line: bytes
    after: bytes

    def undo(self, data, relative):
        span = self.before + self.line + self.after
        offset = data.find(span)
        assert data.count(self.line) == 1 and data.count(span) == 1, relative
        assert offset == 0 or data[offset - 1:offset] == b'\n', relative
        return data.replace(span, self.before + self.after, 1)

    def commutes(self, other):
        # An exact deletion can affect another exact match only through shared
        # context. Single-line anchors may share an outer boundary without
        # sharing either removal or the adjacency which that removal checks.
        ours_context = self.before + self.line + self.after
        theirs_context = other.before + other.line + other.after
        if self.line in theirs_context or other.line in ours_context:
            return False
        ours = ours_context.splitlines(keepends=True)
        theirs = theirs_context.splitlines(keepends=True)
        common = set(ours) & set(theirs)
        if not common:
            return True
        if len(ours) != 3 or len(theirs) != 3:
            return False
        return (len(common) == 1 and
                (ours[0] == theirs[2] or ours[2] == theirs[0]) and
                ours[0] != theirs[0] and ours[2] != theirs[2])


@dataclass(frozen=True)
class Entry:
    name: str
    undo_before: tuple
    undo_after: tuple
    files: dict
    declaration: Declaration | None = None

    @property
    def paths(self):
        return {LIB_SOURCE} if self.declaration else self.files.keys()

    def undo(self, relative, data):
        if self.declaration:
            return self.declaration.undo(data, relative) if relative == LIB_SOURCE else data
        return undo_flight_computer(relative, data, self.files)


def dependency_names(names, name):
    assert isinstance(names, list) and all(isinstance(item, str) and OWNER.fullmatch(item) for item in names), name
    assert len(set(names)) == len(names), f'duplicate dependency: {name}'
    return tuple(names)


def parse_entry(root, protected, name, entry, frozen):
    common = {'schema', 'id', 'kind', 'undoBefore', 'undoAfter'}
    assert common <= entry.keys(), f'{name}: missing protected entry fields'
    assert type(entry['schema']) is int and entry['schema'] == 1 and entry['id'] == name, name
    before = dependency_names(entry['undoBefore'], name)
    after = dependency_names(entry['undoAfter'], name)
    assert name not in before + after and set(before).isdisjoint(after), f'invalid dependency: {name}'
    if entry['kind'] == 'declaration':
        assert set(entry) == common | {'module', 'before', 'after'}, name
        module = entry['module']
        assert isinstance(module, str) and MODULE.fullmatch(module), name
        assert all(isinstance(entry[key], str) and '\r' not in entry[key] and
                   (not entry[key] or entry[key].endswith('\n')) for key in ('before', 'after')), name
        assert entry['before'] and (entry['after'] or frozen), name
        if not frozen:
            sources = [CORE_SOURCE / f'{module}.rs', CORE_SOURCE / module / 'mod.rs']
            present = [path for path in sources if (root / path).is_file()]
            assert len(present) == 1, f'{name}: missing or ambiguous module file for {module}'
            relative = present[0]
            assert not any((root / path).is_symlink() for path in (relative, *relative.parents)), name
            assert relative.relative_to(CORE_SOURCE).as_posix() not in protected, name
        transform = Declaration(entry['before'].encode(), f'pub mod {module};\n'.encode(), entry['after'].encode())
        assert transform.line not in transform.before + transform.after, name
        return Entry(name, before, after, {}, transform)

    assert entry['kind'] == 'inverse' and set(entry) == common | {'manifest', 'sha256', 'base', 'label'}, name
    path = Path(entry['manifest'])
    assert entry['manifest'] == path.as_posix() and not path.is_absolute() and '..' not in path.parts, name
    assert frozen or path == Path('integration') / name / INVERSE_FILE, f'{name}: inverse must be owned by the feature'
    assert HEX_DIGEST.fullmatch(entry['sha256']) and COMMIT.fullmatch(entry['base']), name
    assert isinstance(entry['label'], str) and entry['label'], name
    files = read_inverse(root, protected, path, entry['sha256'], entry['base'], entry['label'])
    return Entry(name, before, after, files)


def discover_entries(root, protected):
    frozen_bytes = read_bytes(root, FROZEN_ENTRIES)
    assert sha(frozen_bytes) == FROZEN_SHA256, 'frozen protected entry index'
    index = json.loads(frozen_bytes, object_pairs_hook=unique_object)
    assert set(index) == {'schema', 'entries'} and index['schema'] == 1
    pins = index['entries']
    entries = {}
    declarations = set()
    for path in sorted((root / 'integration').glob(f'*/{ENTRY_FILE}')):
        name = path.parent.name
        assert OWNER.fullmatch(name), f'invalid protected entry owner: {name}'
        raw = read_bytes(root, path.relative_to(root))
        if name in pins:
            assert sha(raw) == pins[name], f'{name}: frozen protected entry'
        record = json.loads(raw, object_pairs_hook=unique_object)
        assert isinstance(record, dict), name
        entry = parse_entry(root, protected, name, record, name in pins)
        if entry.declaration:
            line = entry.declaration.line
            assert line not in declarations, f'duplicate protected declaration: {name}'
            declarations.add(line)
        entries[name] = entry
    assert pins.keys() <= entries.keys(), f'missing frozen protected entries: {sorted(pins.keys() - entries.keys())}'
    return entries


def order_entries(entries):
    predecessors = {name: set() for name in entries}
    for name, entry in entries.items():
        for other in entry.undo_before:
            assert other in entries, f'{name}: missing undo dependency {other}'
            predecessors[other].add(name)
        for other in entry.undo_after:
            assert other in entries, f'{name}: missing undo dependency {other}'
            predecessors[name].add(other)

    graph = {name: sorted(predecessors[name]) for name in sorted(entries)}
    try:
        order = tuple(TopologicalSorter(graph).static_order())
    except CycleError as error:
        raise AssertionError(f'cyclic protected undo dependencies: {error.args[1]}') from error

    ancestors = {}
    for name in order:
        ancestors[name] = predecessors[name].union(*(ancestors[prior] for prior in predecessors[name]))
    for first, second in combinations(order, 2):
        left, right = entries[first], entries[second]
        if first in ancestors[second] or not (left.paths & right.paths):
            continue
        assert (left.declaration and right.declaration and left.declaration.commutes(right.declaration)), (
            f'ambiguous protected undo order: {first}, {second}'
        )
    return tuple(entries[name] for name in order)


def undo_flight_computer(relative, data, inverse):
    """Restore exact pre-slice bytes, without fuzzy matching or exclusions."""
    if relative not in inverse:
        return data
    record = inverse[relative]
    assert sha(data) == record['afterSHA256'], relative
    assert record['hunks'], relative
    restored = bytearray()
    cursor = 0
    for hunk in record['hunks']:
        start = hunk['offset']
        current = hunk['after'].encode('utf-8')
        previous = hunk['before'].encode('utf-8')
        assert cursor <= start <= len(data) and current != previous, relative
        end = start + len(current)
        assert end <= len(data) and data[start:end] == current, relative
        restored.extend(data[cursor:start])
        restored.extend(previous)
        cursor = end
    restored.extend(data[cursor:])
    result = bytes(restored)
    assert sha(result) == record['beforeSHA256'], relative
    return result
