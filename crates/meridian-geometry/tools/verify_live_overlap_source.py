"""Verify source identity and prior tests without executing geometry."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_live_overlap import restore
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]
PARENT = '4fb8005923a04d4a47b6fe2005c093f9f64fdf62'


def verify():
    freeze = CRATE / 'fixtures/live-overlap-reference'
    for name, pin in json.loads((freeze / 'FROZEN.json').read_text())['files'].items():
        data = (freeze / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['certificate.rs', 'sections.rs', 'region.rs']:
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', PARENT + ':crates/meridian-geometry/src/' + name]).decode()
        assert old == (freeze / ('before-' + name)).read_text()
        current = (CRATE / 'src' / name).read_text()
        assert restore(name, current) == old
        # Every old test is checked separately, outside inverse restoration.
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
            signature = 'fn ' + match[1] + '('
            assert body(current, signature) == body(old, signature), signature
    raw = (freeze / 'collection-control.rs.txt').read_text()
    assert (freeze / 'compiled-collection-control.rs').read_text() == raw.replace('fn collection_candidates(', 'fn frozen_collection_candidates(', 1)
    sections = (freeze / 'before-sections.rs').read_text()
    start = sections.index('impl FacetBoundsIndex {\n    fn collection_candidates(')
    end = sections.index('\n}\n\ntype JointSection', start) + 2
    assert raw == sections[start:end] + '\n'
    print('PASS_SOURCE_ONLY: explicit correction inversion, full before-SUT Git identity, unchanged old test bodies and compiled candidate control')


if __name__ == '__main__':
    verify()
