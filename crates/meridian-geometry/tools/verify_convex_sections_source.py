"""Validate frozen controls and all old tests, without executing geometry."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_convex_sections import restore_convex_sections
from strip_budget_context import body
from restore_cell_index import restore_cell_index

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    f = CRATE / 'fixtures/convex-section-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['certificate.rs', 'sections.rs', 'strict_interior.rs']:
        original = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name], text=True)
        assert original == (f / ('before-' + name)).read_text()
        assert restore_convex_sections(name, (CRATE / 'src' / name).read_text()) == original
    original = (f / 'before-certificate.rs').read_text()
    raw = body(original, 'fn reconstruct_records(') + '\n'
    assert (f / 'publisher-control.rs.txt').read_text() == raw
    assert (f / 'compiled-publisher-control.rs').read_text() == raw.replace('fn reconstruct_records(', 'fn reference_convex_sections_publisher(', 1)
    assert body((CRATE / 'src/certificate.rs').read_text(), 'fn candidates(') == body(original, 'fn candidates('), 'original finite traversal changed'
    old_sections = (f / 'before-sections.rs').read_text()
    current_sections = (CRATE / 'src/sections.rs').read_text()
    for signature in ['fn joint_section(', 'pub(super) fn occupied(', 'fn crosses(', 'pub(super) fn fields_visible(', 'fn collection_with_support(', 'fn collect_admitted<']:
        assert body(current_sections, signature) == body(old_sections, signature), signature
    copied = body(current_sections, 'fn collect_rows_admitted<')
    old = body(old_sections, 'fn collect_admitted<')
    copied = copied.replace('fn collect_rows_admitted<', 'fn collect_admitted<', 1).replace('rows: &super::convex_sections::Rows,', "certificates: Option<&super::strict_interior::InteriorCertificates<'_>>,")
    copied = copied.replace('if finite.meets(&group.bounds, b)? && !rows.contains(*owner, b)? {', 'if finite.meets(&group.bounds, b)? {', 1)
    start = copied.index('            let candidates =')
    end = copied.index('            for (ordinal, physical, outward)', start)
    a = old.index('            let candidates ='); z = old.index('            for (ordinal, physical, outward)', a)
    copied = copied[:start] + old[a:z] + copied[end:]
    assert re.sub(r'\s+', '', copied) == re.sub(r'\s+', '', old), 'collector physical, field, ordering, prices or rollback changed'
    count = 0
    paths = subprocess.check_output(['git', '-C', str(REPO), 'ls-tree', '-r', '--name-only', pins['parent'], '--', 'crates/meridian-geometry/src', 'crates/meridian-geometry/tests'], text=True).splitlines()
    for path in paths:
        if not path.endswith('.rs'):
            continue
        before = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + path], text=True)
        after = (REPO / path).read_text()
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', before):
            signature = 'fn ' + match[1] + '('
            assert body(after, signature) == body(before, signature), (path, signature)
            count += 1
    assert count == 71, count
    main = body(restore_cell_index('certificate.rs', (CRATE / 'src/certificate.rs').read_text()), 'fn reconstruct_records(')
    assert main.index('audit_original_sheets(') < main.index('InteriorCertificates::build_sections(') < main.index('Rows::build(') < main.index('collect_rows(')
    assert main.index('drop(rows);') < main.index('let arrangement = arrangement_result?;')
    print('PASS_SOURCE_ONLY: all71 old bodies byte exact; frozen complete116 publisher; three-file inverse exact; independent collector matches old physical/coplanar/order/charged work; original joint/parity/field paths unchanged; no compiler or geometry executed')


if __name__ == '__main__':
    verify()
