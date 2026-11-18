"""Check frozen controls and exact inversions; never execute geometry."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_source_work import restore_source_work
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    f = CRATE / 'fixtures/source-work-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['certificate.rs', 'sections.rs', 'section_arrangement.rs', 'strict_interior.rs']:
        original = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name]).decode()
        assert (f / ('before-' + name)).read_text() == original
        assert restore_source_work(name, (CRATE / 'src' / name).read_text()) == original
    old = (f / 'before-certificate.rs').read_text()
    raw = body(old, 'fn reconstruct_records(') + '\n'
    assert (f / 'publisher-control.rs.txt').read_text() == raw
    assert (f / 'compiled-publisher-control.rs').read_text() == raw.replace('fn reconstruct_records(', 'fn reference_source_work_publisher(', 1)
    old = (f / 'before-section_arrangement.rs').read_text()
    new = (CRATE / 'src/section_arrangement.rs').read_text()
    assert body(new, 'fn clip(') == body(old, 'fn clip(')
    common = body(new, 'fn build_mode<F>(')
    old_common = body(old, 'pub(crate) fn build_accounted<F>(')
    start = common.index('        for (ordinal, segment) in')
    end = common.index('                b.state([', start)
    old_start = old_common.index('        for segment in')
    old_end = old_common.index('                b.state([', old_start)
    restored = common[:start] + old_common[old_start:old_end] + common[end:]
    restored = restored.replace('fn build_mode<F>(', 'pub(crate) fn build_accounted<F>(', 1).replace('        reuse: bool,\n', '', 1)
    assert restored == old_common, 'common false mode changed a predicate, price, guard or cleanup'
    assert 'Self::build_mode(domain, barriers, b, visible, false)' in body(new, 'pub(crate) fn build_accounted<F>(')
    assert 'Self::build_mode(domain, barriers, b, visible, true)' in body(new, 'pub(crate) fn build_source<F>(')
    count = 0
    for top in ['src', 'tests']:
        old_paths = subprocess.check_output(['git', '-C', str(REPO), 'ls-tree', '-r', '--name-only', pins['parent'], '--', 'crates/meridian-geometry/' + top], text=True).splitlines()
        for path in old_paths:
            if not path.endswith('.rs'):
                continue
            old_source = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + path], text=True)
            current = (REPO / path).read_text()
            for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old_source):
                signature = 'fn ' + match[1] + '('
                assert body(current, signature) == body(old_source, signature), (path, signature)
                count += 1
    assert count == 67, count
    current = (CRATE / 'src/certificate.rs').read_text()
    main = body(current, 'fn reconstruct_records(')
    assert main.index('audit_original_sheets(') < main.index('InteriorCertificates::build(') < main.index('collect_certified(')
    assert 'Arrangement::build_source(' in main
    assert 'build_spatial(' not in main
    print('PASS_SOURCE_ONLY: c0 before publisher frozen; original clip and common false mode exact; all67 old test bodies retained; four-file inverse exact; no compiler/geometry executed')


if __name__ == '__main__':
    verify()
