"""Restore cell-index dispatch before historical source checks."""
import json
from pathlib import Path
from restore_separated_sections import restore_separated

CRATE = Path(__file__).resolve().parents[1]


def restore_cell_index(name, source):
    source = restore_separated(name, source)
    edits = json.loads((CRATE / 'integration/cell-index/INVERSE-EDITS.json').read_text())
    if name not in edits:
        return source
    lines = source.splitlines(True)
    for edit in reversed(edits[name]):
        a, z = edit['afterStart'], edit['afterEnd']
        assert ''.join(lines[a:z]) == edit['after'], (name, a)
        lines[a:z] = edit['before'].splitlines(True)
    result = ''.join(lines)
    assert result == (CRATE / 'fixtures/cell-index-reference' / ('before-' + name)).read_text(), name
    return result
