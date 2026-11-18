"""Verify metadata-only test-control gate; never execute compiler or geometry."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_section_control import restore_section_control
from strip_budget_context import body
from restore_vertex_distances import restore_vertex_distances

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    f = CRATE / 'fixtures/section-control-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    old = (f / 'before-source.rs').read_bytes()
    pin = pins['files']['before-source.rs']
    assert len(old) == pin['bytes'] and hashlib.sha256(old).hexdigest() == pin['sha256']
    assert old == subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/source.rs'])
    current = (CRATE / 'src/source.rs').read_text()
    assert restore_section_control('source.rs', current) == old.decode(), 'change beyond two cfg gates and import partition'
    assert body(current, 'pub(crate) fn section(') == body(old.decode(), 'pub(crate) fn section('), 'control body, price or guard changed'
    prod = current[:current.index('    #[cfg(test)]\n    pub(crate) fn section(')]
    assert len(re.findall(r'\badd\b', current)) == len(re.findall(r'\bscale\b', current)) == 2
    assert 'use crate::exact::{axis, checked, cross, dot, orient, point, project, sub};' in prod
    assert re.search(r'&sub\(', prod), 'sub must remain a production import'
    assert '#[allow' not in current and '#[expect' not in current
    count = 0
    for path in subprocess.check_output(['git', '-C', str(REPO), 'ls-tree', '-r', '--name-only', pins['parent'], '--', 'crates/meridian-geometry/src', 'crates/meridian-geometry/tests'], text=True).splitlines():
        if not path.endswith('.rs'):
            continue
        original = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + path], text=True)
        after = (REPO / path).read_text()
        if path.endswith('/source.rs'):
            assert restore_section_control('source.rs', after) == original
        else:
            assert restore_vertex_distances(Path(path).name, after) == original, path
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', original):
            signature = 'fn ' + match[1] + '('
            assert body(after, signature) == body(original, signature), (path, signature)
            count += 1
    assert count == 84, count
    print('PASS_SOURCE_ONLY: exact metadata/import inverse; original physical control body/48guard/charges retained; add+scale test-only, sub production; all84old bodies and every other src/test file byte exact; no suppression/compiler/geometry execution')


if __name__ == '__main__':
    verify()
