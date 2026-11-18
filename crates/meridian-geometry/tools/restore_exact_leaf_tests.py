"""Restore only the bounded exact-leaf test validation correction."""
import json
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]


def restore_exact_leaf_tests(source):
    edits = json.loads((CRATE / 'integration/exact-leaf-test-correction/INVERSE-EDITS.json').read_text())
    lines = source.splitlines(True)
    for edit in reversed(edits):
        a, z = edit['afterStart'], edit['afterEnd']
        assert ''.join(lines[a:z]) == edit['after'], a
        lines[a:z] = edit['before'].splitlines(True)
    result = ''.join(lines)
    assert result == (CRATE / 'fixtures/exact-leaf-test-reference/before-exact_leaf_tests.rs').read_text()
    return result
