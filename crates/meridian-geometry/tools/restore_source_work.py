"""Restore source-work integration before running historical source proofs."""
import json
from pathlib import Path
from restore_convex_sections import restore_convex_sections

CRATE = Path(__file__).resolve().parents[1]


def restore_source_work(name, source):
    source = restore_convex_sections(name, source)
    edits = json.loads((CRATE / 'integration/source-work/INVERSE-EDITS.json').read_text())
    if name not in edits:
        return source
    lines = source.splitlines(True)
    for edit in reversed(edits[name]):
        start, end = edit['afterStart'], edit['afterEnd']
        assert ''.join(lines[start:end]) == edit['after'], (name, start)
        lines[start:end] = edit['before'].splitlines(True)
    result = ''.join(lines)
    assert result == (CRATE / 'fixtures/source-work-reference' / ('before-' + name)).read_text(), name
    return result
