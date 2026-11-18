"""Check exact scalar-cache wiring and independent controls without execution."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_vertex_distances import restore_vertex_distances
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def normalized(value):
    return re.sub(r"\s+", "", value)


def verify():
    f = CRATE / 'fixtures/vertex-distance-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['certificate.rs', 'sections.rs', 'convex_sections.rs']:
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name], text=True)
        assert old == (f / ('before-' + name)).read_text()
        assert restore_vertex_distances(name, (CRATE / 'src' / name).read_text()) == old
    original = (f / 'before-certificate.rs').read_text()
    raw = body(original, 'fn reconstruct_records(') + '\n'
    assert (f / 'publisher-control.rs.txt').read_text() == raw
    assert (f / 'compiled-publisher-control.rs').read_text() == raw.replace('fn reconstruct_records(', 'fn reference_vertex_distance_publisher(', 1)
    before = (f / 'before-sections.rs').read_text()
    current = (CRATE / 'src/sections.rs').read_text()
    for signature in ['fn joint_section_source_admitted(', 'pub(super) fn joint_section_source(', 'fn collect_separating_admitted<', 'pub(super) fn collect_separating<', 'fn collect_rows_admitted<', 'pub(super) fn occupied(', 'fn crosses(', 'pub(super) fn fields_visible(']:
        assert body(current, signature) == body(before, signature), signature
    old_kernel = body(before, 'fn joint_section_source_admitted(')
    expected = old_kernel.replace('fn joint_section_source_admitted(', 'fn joint_section_distances_admitted(', 1).replace('    b: &mut Budget,\n', '    distances: [&crate::Rational; 3],\n    b: &mut Budget,\n', 1)
    a = expected.index('    let mut distances ='); z = expected.index('    if distances.iter()', a)
    expected = expected[:a] + expected[z:]
    expected = expected.replace('distances.iter().all(Zero::is_zero)', 'distances.iter().all(|v| v.is_zero())').replace('&distances[i] / (&distances[i] - &distances[j])', 'distances[i] / (distances[i] - distances[j])')
    assert normalized(body(current, 'fn joint_section_distances_admitted(')) == normalized(expected), 'kernel geometry, guards or residual prices changed'
    copied = body(current, "fn collect_cached_admitted<'a>(")
    old = body(before, "fn collect_rows_admitted<'a>(")
    copied = copied.replace('fn collect_cached_admitted<', 'fn collect_rows_admitted<', 1)
    a = copied.index('        source:'); z = copied.index('    ) ->', a)
    oa = old.index('        originals:'); oz = old.index('    ) ->', oa)
    copied = copied[:a] + old[oa:oz] + copied[z:]
    a = copied.index('        let originals ='); z = copied.index('        b.retain(24)?;', a)
    copied = copied[:a] + copied[z:]
    copied = copied.replace('plane.section(ordinal, physical, outward, b)?', 'joint_section(face, other, physical, outward, b)?', 1)
    assert normalized(copied) == normalized(old), 'collector constraints, source ties, ordering or guard work changed'
    rows = (CRATE / 'src/convex_sections.rs').read_text()
    old_rows = (f / 'before-convex_sections.rs').read_text()
    assert rows.replace('    pub(super) fn build(', '    #[cfg(test)]\n    pub(super) fn build(', 1).replace('    fn build_admitted(', '    #[cfg(test)]\n    fn build_admitted(', 1).replace('    pub(super) fn occupied(', '    #[cfg(test)]\n    pub(super) fn occupied(', 1) == old_rows
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
        if Path(path).name not in ['certificate.rs', 'sections.rs', 'convex_sections.rs']:
            assert after == old, path
    assert count == 84, count
    main = body((CRATE / 'src/certificate.rs').read_text(), 'fn reconstruct_records(')
    assert main.index('drop(vertex_distances);') < main.index('drop(interior);') < main.index('drop(owner_bounds);')
    assert 'FacetBoundsIndex::build_cells(' not in main
    print('PASS_SOURCE_ONLY: all84oldbodies exact; complete frozen692d publisher; three-file inverse; cached kernel exact original after only borrowed-distance substitution; collector exact old body after cache wiring; both row classifiers/finite-gap path unchanged; no compiler or geometry executed')


if __name__ == '__main__':
    verify()
