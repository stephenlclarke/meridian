"""Verify actual-source binding, old assertions and unchanged compiled oracles."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_proved_visits import restore_proved
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    f = CRATE / 'fixtures/proved-visits-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['certificate.rs', 'sections.rs', 'section_arrangement.rs']:
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name]).decode()
        assert old == (f / ('before-' + name)).read_text()
        current = (CRATE / 'src' / name).read_text()
        assert restore_proved(name, current) == old
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
            signature = 'fn ' + match[1] + '('
            assert body(current, signature) == body(old, signature), signature
    old = (f / 'before-sections.rs').read_text()
    start = old.index('impl FacetBoundsIndex {\n    fn collection_candidates(')
    end = old.index('\ntype JointSection', start)
    raw = old[start:end].strip() + '\n'
    assert (f / 'collection-control.rs.txt').read_text() == raw
    assert (f / 'compiled-collection-control.rs').read_text() == raw.replace('fn collection_candidates(', 'fn owner_inheritance_reference(', 1)
    old = (f / 'before-section_arrangement.rs').read_text()
    raw = body(old, 'fn key(') + '\n' + body(old, 'fn clip(') + '\n'
    assert (f / 'clip-control.rs.txt').read_text() == raw
    expected = raw.replace('fn key(', 'fn frozen_key(', 1).replace('fn clip(', 'fn frozen_clip(', 1).replace('Some(key(result))', 'Some(frozen_key(result))', 1)
    assert (f / 'compiled-clip-control.rs').read_text() == expected
    print('PASS_SOURCE_ONLY: complete984/actual8d-parent inversion; unchanged56bodies; compiled original joint traversal and clip exact save function names')


if __name__ == '__main__':
    verify()
