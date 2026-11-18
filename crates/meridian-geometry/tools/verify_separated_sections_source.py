"""Validate frozen full publisher, old controls and all80 prior bodies only."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_separated_sections import restore_separated
from strip_budget_context import body
from restore_section_control import restore_section_control

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def normalized(value):
    return re.sub(r"\s+", "", value)


def verify():
    f = CRATE / 'fixtures/separated-section-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['certificate.rs', 'spatial_index.rs', 'sections.rs', 'convex_sections.rs']:
        original = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name], text=True)
        assert original == (f / ('before-' + name)).read_text()
        assert restore_separated(name, (CRATE / 'src' / name).read_text()) == original
    before = (f / 'before-certificate.rs').read_text()
    raw = body(before, 'fn reconstruct_records(') + '\n'
    assert (f / 'publisher-control.rs.txt').read_text() == raw
    assert (f / 'compiled-publisher-control.rs').read_text() == raw.replace('fn reconstruct_records(', 'fn reference_separated_section_publisher(', 1)
    current = (CRATE / 'src/certificate.rs').read_text()
    main = body(current, 'fn reconstruct_records(')
    assert 'build_cells(' not in main and 'cell_memberships(' not in main
    expected = body(before, 'fn reconstruct_records(').replace('FacetBoundsIndex::cell_memberships(', 'FacetBoundsIndex::memberships(', 1).replace('FacetBoundsIndex::build_cells(', 'FacetBoundsIndex::build(', 1)
    restored = main.replace('Rows::build_separating(', 'Rows::build(', 1).replace('Frame::collect_separating(', 'Frame::collect_rows(', 1).replace('rows.occupied_facet(', 'rows.occupied(', 1)
    assert normalized(restored) == normalized(expected), 'main changed beyond withdrawal and three new source entry names'
    assert '#[cfg(test)]\n#[path = "cell_index.rs"]' in current
    sections = (CRATE / 'src/sections.rs').read_text()
    old_sections = (f / 'before-sections.rs').read_text()
    for signature in ['fn joint_section(', 'fn collect_rows_admitted<', 'pub(super) fn collect_rows<', 'pub(super) fn occupied(', 'fn crosses(', 'pub(super) fn fields_visible(', 'fn collection_with_support(']:
        assert body(sections, signature) == body(old_sections, signature), signature
    rows = (CRATE / 'src/convex_sections.rs').read_text()
    old_rows = (f / 'before-convex_sections.rs').read_text()
    for signature in ['pub(super) fn build(', 'fn build_admitted(', 'pub(super) fn occupied(']:
        actual = normalized(body(rows, signature)).replace('separated:false,', '')
        assert actual == normalized(body(old_rows, signature)), signature
    for signature in ['fn build(', 'fn build_range(', 'fn memberships(', 'fn query_ray(', 'fn ray_intersections(']:
        assert body(current, signature) == body(before, signature), signature
    count = 0
    for path in subprocess.check_output(['git', '-C', str(REPO), 'ls-tree', '-r', '--name-only', pins['parent'], '--', 'crates/meridian-geometry/src', 'crates/meridian-geometry/tests'], text=True).splitlines():
        if not path.endswith('.rs'):
            continue
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + path], text=True)
        after = (REPO / path).read_text()
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
            signature = 'fn ' + match[1] + '('
            assert body(after, signature) == body(old, signature), (path, signature)
            count += 1
        if Path(path).name not in ['certificate.rs', 'spatial_index.rs', 'sections.rs', 'convex_sections.rs']:
            assert restore_section_control(Path(path).name, after) == old, path
    assert count == 80, count
    print('PASS_SOURCE_ONLY: all80 old bodies byte exact incl cell prototype; four-file inverse; frozen completec776 publisher; original rows/collector/joint/ray/fields/box logic retained as independent controls; main only constructor withdrawal and three new entry names; no compiler or geometry executed')


if __name__ == '__main__':
    verify()
