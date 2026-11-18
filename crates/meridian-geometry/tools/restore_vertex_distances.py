"""Restore per-plane vertex-distance integration before historical proofs."""
import json
from pathlib import Path
from restore_vertex_dispatch import restore_vertex_dispatch

CRATE = Path(__file__).resolve().parents[1]


def restore_vertex_distances(name, source):
    source = restore_vertex_dispatch(name, source)
    edits = json.loads((CRATE / 'integration/vertex-distances/INVERSE-EDITS.json').read_text())
    if name not in edits:
        return source
    lines = source.splitlines(True)
    for edit in reversed(edits[name]):
        a, z = edit['afterStart'], edit['afterEnd']
        assert ''.join(lines[a:z]) == edit['after'], (name, a)
        lines[a:z] = edit['before'].splitlines(True)
    result = ''.join(lines)
    assert result == (CRATE / 'fixtures/vertex-distance-reference' / ('before-' + name)).read_text(), name
    return result
