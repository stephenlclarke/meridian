"""Check complete leaf eligibility controls and inverses without geometry execution."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_exact_leaf import restore_exact_leaf
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def norm(value):
    return re.sub(r"\s+", "", value)


def verify():
    f = CRATE / 'fixtures/exact-leaf-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    count = 0
    for path in subprocess.check_output(['git', '-C', str(REPO), 'ls-tree', '-r', '--name-only', pins['parent'], '--', 'crates/meridian-geometry/src', 'crates/meridian-geometry/tests'], text=True).splitlines():
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + path], text=True)
        after = (REPO / path).read_text()
        assert restore_exact_leaf(Path(path).name, after) == old, path
        if path.endswith('.rs'):
            for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
                signature = 'fn ' + match[1] + '('
                assert body(after, signature) == body(old, signature), (path, signature)
                count += 1
    assert count == 88, count
    before = (f / 'before-sections.rs').read_text()
    current = (CRATE / 'src/sections.rs').read_text()
    for signature in ['fn collection_candidates(', 'fn collection_with_support(', 'fn joint_section_distances_admitted(', 'fn joint_section_source_admitted(']:
        assert body(current, signature) == body(before, signature)
    for signature in ["pub(super) fn collect_cached<'a>(", "fn collect_cached_admitted<'a>("]:
        old = body(before, signature)
        control_sig = signature.replace('collect_cached', 'collect_cached_control')
        assert body(current, control_sig).replace('collect_cached_control', 'collect_cached') == old
    query = body(before, 'fn collection_with_support(')
    expected = query.replace('fn collection_with_support(', 'fn collection_cached(', 1).replace("        certificates: Option<&super::strict_interior::InteriorCertificates<'_>>,", "        distances: &mut vertex_distances::Plane<'_, '_, '_>,", 1).replace('Self::stack_members(self.facets.len()) + 8', 'Self::stack_members(self.facets.len()) + 16', 1)
    a = expected.index('            // Axis-aligned'); z = expected.index('            // Four fields', a)
    expected = expected[:a] + expected[z:]
    expected = expected.replace("""                        if empty_own && owner == face.id().component {
                            false
                        } else {
                            log_cost(owners.len(), b)?;
                            owners.contains_key(&owner)
                        }""", """                        log_cost(owners.len(), b)?;
                        owners.contains_key(&owner)""", 1)
    expected = expected.replace("""if outward_possible
                            && !(empty_own
                                && originals[ordinal].id().component == face.id().component)
                        {""", 'if outward_possible {', 1)
    expected = expected.replace('owner_allowed && plane.meets(bounds, b)?', 'owner_allowed && b.with_retained(96, |b| distances.outward(ordinal, b))?', 1)
    assert norm(body(current, 'fn collection_cached(')) == norm(expected), 'traversal/node/owner/source-order or residual prices changed'
    old_collector = body(before, "fn collect_cached_admitted<'a>(")
    new_collector = body(current, "fn collect_cached_admitted<'a>(")
    assert norm(new_collector.replace('index.collection_cached(face, finite, &frame.owners, originals, &mut plane, b)?', 'index.collection_candidates(face, finite, &frame.owners, originals, b)?')) == norm(old_collector)
    old = (f / 'before-vertex_distances.rs').read_text()
    new = (CRATE / 'src/vertex_distances.rs').read_text()
    section = body(old, 'pub(in crate::certificate) fn section(')
    a = section.index("        let originals:"); z = section.index('        let face = self.face;')
    expected_prepare = 'fn prepare(&mut self, ordinal: usize, b: &mut Budget) -> Result<[usize; 3], Failure> {' + section[a:z] + '        Ok(ids)\n    }'
    assert norm(body(new, 'fn prepare(')) == norm(expected_prepare)
    expected_section = section[:a] + "        let ids = self.prepare(ordinal, b)?;\n        let originals: &'a [Facet] = self.cache.originals;\n        let other = &originals[ordinal];\n" + section[z:]
    assert norm(body(new, 'pub(in crate::certificate) fn section(')) == norm(expected_section)
    for signature in ['pub(in crate::certificate) fn build(', 'fn build_admitted(', 'pub(in crate::certificate) fn plane<']:
        assert body(new, signature) == body(old, signature)
    raw = body((f / 'before-certificate.rs').read_text(), 'fn reconstruct_records(') + '\n'
    assert (f / 'publisher-control.rs.txt').read_text() == raw
    assert (f / 'compiled-publisher-control.rs').read_text() == raw.replace('fn reconstruct_records(', 'fn reference_exact_leaf_publisher(', 1).replace('sections::Frame::collect_cached(', 'sections::Frame::collect_cached_control(', 1)
    print('PASS_SOURCE_ONLY: two-file inverse; all88oldbodies and other source/tests exact; independent complete parent collector/publisher; leaf query retains original traversal, fixed node/owner logic and source order; distance initialization and joint kernel prices/geometry exact after helper extraction; no compiler or geometry executed')


if __name__ == '__main__':
    verify()
