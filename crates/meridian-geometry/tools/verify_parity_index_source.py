"""Check immutable before-SUT bytes, old assertions and original crossing oracle."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_parity_index import restore_parity
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    frozen = CRATE / 'fixtures/parity-index-reference'
    pins = json.loads((frozen / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (frozen / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['certificate.rs', 'sections.rs']:
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name]).decode()
        assert old == (frozen / ('before-' + name)).read_text()
        current = (CRATE / 'src' / name).read_text()
        assert restore_parity(name, current) == old
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
            signature = 'fn ' + match[1] + '('
            assert body(current, signature) == body(old, signature), signature
    old = (frozen / 'before-sections.rs').read_text()
    current = (CRATE / 'src/sections.rs').read_text()
    for signature in ['pub(super) fn occupied(', 'fn crosses(']:
        assert body(current, signature) == body(old, signature), signature
    print('PASS_SOURCE_ONLY: exact5602/actual27c parent integration inversion, old58 assertions, unchanged full-scan/crossing oracle')


if __name__ == '__main__':
    verify()
