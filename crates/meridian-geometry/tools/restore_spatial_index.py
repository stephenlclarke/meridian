"""Restore spatial integration before the unchanged earlier geometry proofs."""
import json
from pathlib import Path
from restore_strict_interior import restore_interior

CRATE = Path(__file__).resolve().parents[1]


def restore_spatial(name, source):
    source = restore_interior(name, source)
    edits = json.loads((CRATE / 'integration/spatial-index/INVERSE-EDITS.json').read_text())
    if name not in edits:
        return source
    lines = source.splitlines(True)
    for edit in reversed(edits[name]):
        start, end = edit['afterStart'], edit['afterEnd']
        assert ''.join(lines[start:end]) == edit['after'], (name, start)
        lines[start:end] = edit['before'].splitlines(True)
    result = ''.join(lines)
    assert result == (CRATE / 'fixtures/spatial-index-reference' / ('before-' + name)).read_text(), name
    return result
