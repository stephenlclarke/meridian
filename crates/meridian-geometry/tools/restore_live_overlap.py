"""Restore only the explicit reviewed live-overlap edit spans for prior proofs."""
import json
from pathlib import Path
from restore_live_runs import restore_runs

CRATE = Path(__file__).resolve().parents[1]


def restore(name, source):
    source = restore_runs(name, source)
    edits = json.loads((CRATE / 'integration/live-overlap/INVERSE-EDITS.json').read_text())
    if name not in edits:
        return source
    lines = source.splitlines(True)
    for edit in reversed(edits[name]):
        start, end = edit['afterStart'], edit['afterEnd']
        assert ''.join(lines[start:end]) == edit['after'], (name, start)
        lines[start:end] = edit['before'].splitlines(True)
    result = ''.join(lines)
    assert result == (CRATE / 'fixtures/live-overlap-reference' / ('before-' + name)).read_text(), name
    return result
