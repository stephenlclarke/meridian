"""Verify actual before-SUT source, old assertions and the contiguous builder."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_spatial_index import restore_spatial
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def normalized(source):
    return re.sub(r'\s+', '', source)


def verify():
    f = CRATE / 'fixtures/spatial-index-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['certificate.rs', 'sections.rs']:
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name]).decode()
        assert old == (f / ('before-' + name)).read_text()
        current = (CRATE / 'src' / name).read_text()
        assert restore_spatial(name, current) == old
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
            signature = 'fn ' + match[1] + '('
            assert body(current, signature) == body(old, signature), signature
    old = (f / 'before-certificate.rs').read_text()
    current = (CRATE / 'src/certificate.rs').read_text()
    restored = body(current, 'fn build(').replace('order: None,', '')
    assert normalized(restored) == normalized(body(old, 'fn build('))
    for signature in ['fn build_range(', 'fn memberships(', 'fn stack_members(', 'fn node_limit(', 'fn meets(', 'fn ray_may_meet(', 'fn ray_intersects(']:
        assert body(current, signature) == body(old, signature), signature
    print('PASS_SOURCE_ONLY: a91/actual7a complete inversion, old60 assertions, original contiguous builder/guards/predicates unchanged save empty ordering metadata')


if __name__ == '__main__':
    verify()
