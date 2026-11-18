"""Prove test-only correction and unchanged whole-reviewed production source."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_exact_leaf_tests import restore_exact_leaf_tests
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    fixture = CRATE / 'fixtures/exact-leaf-test-reference'
    pins = json.loads((fixture / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (fixture / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256']
    changed = 'crates/meridian-geometry/src/exact_leaf_tests.rs'
    old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + changed], text=True)
    current = (REPO / changed).read_text()
    assert restore_exact_leaf_tests(current) == old
    count = 0
    for path in subprocess.check_output(['git', '-C', str(REPO), 'ls-tree', '-r', '--name-only', pins['parent'], '--', 'crates/meridian-geometry/src', 'crates/meridian-geometry/tests'], text=True).splitlines():
        before = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + path], text=True)
        after = (REPO / path).read_text()
        if path != changed:
            assert after == before, path
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', before):
            signature = 'fn ' + match[1] + '('
            if match[1] != 'exact_leaf_eligibility_preserves_joint_results_and_reduces_complete_batch_work':
                assert body(after, signature) == body(before, signature), (path, signature)
                count += 1
    assert count == 90, count
    parent_cache = CRATE / 'fixtures/exact-leaf-reference/before-vertex_distances.rs'
    parent_bytes = subprocess.check_output(['git', '-C', str(REPO), 'show', '82a79090e3c4d2160e0046f72d3fa8a01c6a46fe:crates/meridian-geometry/src/vertex_distances.rs'])
    assert parent_cache.read_bytes() == parent_bytes
    assert '#[path = "../fixtures/exact-leaf-reference/before-vertex_distances.rs"]\nmod parent_distance_cache;' in current
    batch = body(current, 'fn exact_leaf_eligibility_preserves_joint_results_and_reduces_complete_batch_work(')
    for required in ['parent_distance_cache::VertexDistances::build(', 'parent_cache.plane(', 'parent_plane.section(', 'old_index.collection_candidates(', 'new_index.collection_cached(', 'let mut oracle = Budget::new', 'joint_section_source(', 'assert_eq!(actual, expected);', 'assert_eq!(expected, uncached);', 'assert!(new.operations() < old.operations());', 'drop(parent_cache);', 'drop_cache(cache, &mut new);', 'old.release(3 * originals.len());', 'new.release(3 * originals.len());']:
        assert re.sub(r"\s+", "", required) in re.sub(r"\s+", "", batch), required
    assert batch.count('Facet::input_memberships(') == 2
    assert batch.count('Bounds::for_facet(') == 2
    assert batch.index('drop(parent_cache);') < batch.index('assert!(new.operations() < old.operations());')
    cavity = body(current, 'fn exact_leaf_uncertified_cavity_publisher_exercises_joint_dispatch_and_preserves_all_outputs(')
    for required in ['shell_cube(0, 8)?', 'shell_cube(2, 6)?', 'vertices.swap(1, 2);', 'assert_eq!(originals.len(), 280);', 'vec![1, 2, 3, 4]', 'assert!(!rows.contains(0, &mut precondition)?);', 'assert!(finite.meets(&cavity.bounds, &mut precondition)?);', 'reference_exact_leaf_publisher(', 'assert_eq!(actual.triangles(), expected.triangles());', 'assert_eq!(actual.witnesses(), expected.witnesses());', 'integer(1360) / integer(3)', 'assert_eq!(new.retained_memberships(), old.retained_memberships());']:
        assert re.sub(r"\s+", "", required) in re.sub(r"\s+", "", cavity), required
    print('PASS_SOURCE_ONLY: all production and other source/tests byte exact; all88prior and two other new test bodies unchanged; complete test-file inverse; independent frozen parent cache and old box collector in full work baseline; uncached oracle separate; original convex publisher fixture retained; new closed cavity precondition/complete output assertions authored; no compiler or geometry executed')


if __name__ == '__main__':
    verify()
