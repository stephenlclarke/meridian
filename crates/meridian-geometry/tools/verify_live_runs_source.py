"""Check immutable sources and old assertions; never run geometry or a compiler."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_live_runs import restore_runs
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    frozen = CRATE / 'fixtures/live-runs-reference'
    pins = json.loads((frozen / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (frozen / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['sections.rs', 'section_arrangement.rs']:
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name]).decode()
        assert old == (frozen / ('before-' + name)).read_text()
        current = (CRATE / 'src' / name).read_text()
        assert restore_runs(name, current) == old
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
            signature = 'fn ' + match[1] + '('
            assert body(current, signature) == body(old, signature), signature
    old = (frozen / 'before-section_arrangement.rs').read_text()
    expected = '\n'.join(line for line in old.splitlines() if not line.startswith('//!')) + '\n'
    expected = expected.replace('fn build_accounted<F>(', 'fn build_live_runs_reference<F>(', 1)
    assert (frozen / 'compiled-arrangement-control.rs').read_text() == expected
    print('PASS_SOURCE_ONLY: actual-run edits invert to immutable a52 parent; old52 assertions and full compiled arrangement control retained')


if __name__ == '__main__':
    verify()
