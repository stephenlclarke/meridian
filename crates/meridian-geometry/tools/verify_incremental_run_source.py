"""Verify original charged geometry visits and unchanged tests without execution."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_incremental_run import restore_incremental
from strip_budget_context import body, end_delimiter

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def normalized(text):
    return re.sub(r'\s+', '', re.sub(r'//[^\n]*', '', text))


def verify():
    frozen = CRATE / 'fixtures/incremental-run-reference'
    pins = json.loads((frozen / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (frozen / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256']
    for name in ['section_arrangement.rs', 'live_run_tests.rs']:
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name]).decode()
        assert old == (frozen / ('before-' + name)).read_text()
        current = (CRATE / 'src' / name).read_text()
        assert restore_incremental(name, current) == old
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
            signature = 'fn ' + match[1] + '('
            assert body(current, signature) == body(old, signature), signature
    baseline = (CRATE / 'fixtures/live-runs-reference/before-section_arrangement.rs').read_text()
    current = (CRATE / 'src/section_arrangement.rs').read_text()
    for signature in ['fn extend(', 'fn finish(']:
        original = body(baseline, signature)
        restored = body(current, signature)
        restored = restored.replace('    members: &mut usize,\n', '')
        restored = re.sub(r'\s*\*members\s*[+-]=\s*1;', '', restored)
        if signature == 'fn finish(':
            start = restored.index('result.dedup_by(')
            opening = restored.index('(', start)
            end = end_delimiter(restored, opening) + 2
            restored = restored[:start] + 'result.dedup();' + restored[end:]
        assert normalized(restored) == normalized(original), signature + ' geometry and original charges'
    print('PASS_SOURCE_ONLY: frozen736-matched parent, old55test bodies; original extend/finish geometry and charged visits exact after metadata-only normalization')


if __name__ == '__main__':
    verify()
