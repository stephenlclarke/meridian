"""Restore parity-index integration before prior source-preservation checks."""
import json
from pathlib import Path
from restore_spatial_index import restore_spatial

CRATE = Path(__file__).resolve().parents[1]


def restore_parity(name, source):
    source = restore_spatial(name, source)
    edits = json.loads((CRATE / 'integration/parity-index/INVERSE-EDITS.json').read_text())
    if name not in edits:
        return source
    lines = source.splitlines(True)
    for edit in reversed(edits[name]):
        start, end = edit['afterStart'], edit['afterEnd']
        assert ''.join(lines[start:end]) == edit['after'], (name, start)
        lines[start:end] = edit['before'].splitlines(True)
    result = ''.join(lines)
    assert result == (CRATE / 'fixtures/parity-index-reference' / ('before-' + name)).read_text(), name
    return result
