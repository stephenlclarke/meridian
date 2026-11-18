"""Restore the explicit incremental bookkeeping delta for prior source proofs."""
import json
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]


def restore_incremental(name, source):
    edits = json.loads((CRATE / 'integration/incremental-run/INVERSE-EDITS.json').read_text())
    if name not in edits:
        return source
    lines = source.splitlines(True)
    for edit in reversed(edits[name]):
        start, end = edit['afterStart'], edit['afterEnd']
        assert ''.join(lines[start:end]) == edit['after'], (name, start)
        lines[start:end] = edit['before'].splitlines(True)
    result = ''.join(lines)
    assert result == (CRATE / 'fixtures/incremental-run-reference' / ('before-' + name)).read_text(), name
    return result
