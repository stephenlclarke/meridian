"""Prove the bounded dispatch metadata correction without compilation."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_vertex_dispatch import restore_vertex_dispatch
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    fixture = CRATE / 'fixtures/vertex-dispatch-reference'
    freeze = json.loads((fixture / 'FROZEN.json').read_text())
    for name, pin in freeze['files'].items():
        data = (fixture / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256']
    count = 0
    for path in subprocess.check_output(['git', '-C', str(REPO), 'ls-tree', '-r', '--name-only', freeze['parent'], '--', 'crates/meridian-geometry/src', 'crates/meridian-geometry/tests'], text=True).splitlines():
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', freeze['parent'] + ':' + path], text=True)
        after = (REPO / path).read_text()
        assert restore_vertex_dispatch(Path(path).name, after) == old, path
        if path.endswith('.rs'):
            for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
                signature = 'fn ' + match[1] + '('
                assert body(after, signature) == body(old, signature), (path, signature)
                count += 1
    assert count == 88, count
    current = (CRATE / 'src/sections.rs').read_text()
    before = (fixture / 'before-sections.rs').read_text()
    for name in ['collection_candidates', 'collection_with_support']:
        signature = 'fn ' + name + '('
        assert body(current, signature) == body(before, signature)
        assert '    #[cfg(test)]\n    ' + signature not in current
    assert body(current, 'fn collect_cached_admitted<') == body(before, 'fn collect_cached_admitted<')
    print('PASS_SOURCE_ONLY: exact two-cfg inverse; both complete joint-dispatch bodies/prices/owner/source-order logic retained; cached collector unchanged; all88testbodies and every other src/test file byte exact; no compiler or geometry executed')


if __name__ == '__main__':
    verify()
