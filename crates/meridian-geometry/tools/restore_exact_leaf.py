"""Restore exact triangle leaf integration before historical source proofs."""
import json
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]


def restore_exact_leaf(name, source):
    edits = json.loads((CRATE / 'integration/exact-leaf/INVERSE-EDITS.json').read_text())
    if name not in edits:
        return source
    lines = source.splitlines(True)
    for edit in reversed(edits[name]):
        a, z = edit['afterStart'], edit['afterEnd']
        assert ''.join(lines[a:z]) == edit['after'], (name, a)
        lines[a:z] = edit['before'].splitlines(True)
    result = ''.join(lines)
    assert result == (CRATE / 'fixtures/exact-leaf-reference' / ('before-' + name)).read_text(), name
    return result
