"""Restore only the proved-visit correction before earlier source proofs."""
import json
from pathlib import Path
from restore_parity_index import restore_parity

CRATE = Path(__file__).resolve().parents[1]


def restore_proved(name, source):
    source = restore_parity(name, source)
    edits = json.loads((CRATE / 'integration/proved-visits/INVERSE-EDITS.json').read_text())
    if name not in edits:
        return source
    lines = source.splitlines(True)
    for edit in reversed(edits[name]):
        start, end = edit['afterStart'], edit['afterEnd']
        assert ''.join(lines[start:end]) == edit['after'], (name, start)
        lines[start:end] = edit['before'].splitlines(True)
    result = ''.join(lines)
    assert result == (CRATE / 'fixtures/proved-visits-reference' / ('before-' + name)).read_text(), name
    return result
