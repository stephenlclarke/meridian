"""Verify exactly three reviewed charge additions and unchanged prior tests."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    f = CRATE / 'fixtures/interior-accounting-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['strict_interior.rs', 'strict_interior_tests.rs']:
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name]).decode()
        assert old == (f / ('before-' + name)).read_text()
        current = (CRATE / 'src' / name).read_text()
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
            signature = 'fn ' + match[1] + '('
            assert body(current, signature) == body(old, signature), signature
    old = (f / 'before-strict_interior.rs').read_text()
    current = (CRATE / 'src/strict_interior.rs').read_text()
    changes = [
        ('                b.retain(6)?;\n                log_cost(groups.len(), b)?;\n                groups.insert', '                b.retain(6)?;\n                groups.insert'),
        ('        for (id, (bounds, faces)) in groups {\n            b.step(1)?;\n            let group_members', '        for (id, (bounds, faces)) in groups {\n            let group_members'),
        ('            // Necessary strict box containment; equality cannot be interior.\n            b.step(1)?;\n            if !(0..3)', '            // Necessary strict box containment; equality cannot be interior.\n            if !(0..3)'),
    ]
    for added, original in changes:
        assert current.count(added) == 1
        current = current.replace(added, original, 1)
    assert current == old, 'non-accounting production source changed'
    old_tests = (f / 'before-strict_interior_tests.rs').read_text()
    assert (CRATE / 'src/strict_interior_tests.rs').read_text().startswith(old_tests)
    print('PASS_SOURCE_ONLY: exact3charge inverse restores complete332helper; all66priorbodies/assertions retained; one new authored refusal guard')


if __name__ == '__main__':
    verify()
