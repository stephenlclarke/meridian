"""Check exact cell/node/source restoration and all old tests without execution."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_cell_index import restore_cell_index
from strip_budget_context import body
from restore_separated_sections import restore_separated

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    f = CRATE / 'fixtures/cell-index-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['certificate.rs', 'spatial_index.rs']:
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name], text=True)
        assert old == (f / ('before-' + name)).read_text()
        assert restore_cell_index(name, (CRATE / 'src' / name).read_text()) == old
    before = (f / 'before-certificate.rs').read_text()
    raw = body(before, 'fn reconstruct_records(') + '\n'
    assert (f / 'publisher-control.rs.txt').read_text() == raw
    assert (f / 'compiled-publisher-control.rs').read_text() == raw.replace('fn reconstruct_records(', 'fn reference_cell_index_publisher(', 1)
    current = restore_separated('certificate.rs', (CRATE / 'src/certificate.rs').read_text())
    main = body(current, 'fn reconstruct_records(')
    restored = main.replace('FacetBoundsIndex::cell_memberships(originals.len())', 'FacetBoundsIndex::memberships(originals.len())', 1).replace('FacetBoundsIndex::build_cells(originals, b)', 'FacetBoundsIndex::build(originals, b)', 1)
    assert restored == body(before, 'fn reconstruct_records('), 'main changed beyond index constructor/admission'
    for signature in ['fn build(', 'fn build_range(', 'fn memberships(', 'fn candidates(', 'fn query_ray(', 'fn ray_intersections(']:
        assert body(current, signature) == body(before, signature), signature
    old = (f / 'before-spatial_index.rs').read_text()
    new = (CRATE / 'src/cell_index.rs').read_text()
    expected = body(old, 'fn spatial_range(').replace('fn spatial_range(', 'fn cell_range(', 1).replace('        scratch: &mut [usize],\n', '').replace('            self.sort_range(start, end, depth % 3, scratch, b)?;\n', '').replace('self.spatial_range(originals, start, middle, depth + 1, scratch, b)?', 'self.cell_range(originals, start, middle, depth + 1, b)?').replace('self.spatial_range(originals, middle, end, depth + 1, scratch, b)?', 'self.cell_range(originals, middle, end, depth + 1, b)?')
    assert body(new, 'fn cell_range(') == expected, 'original exact leaf bounds, owner proof, union or price changed'
    count = 0
    for path in subprocess.check_output(['git', '-C', str(REPO), 'ls-tree', '-r', '--name-only', pins['parent'], '--', 'crates/meridian-geometry/src', 'crates/meridian-geometry/tests'], text=True).splitlines():
        if not path.endswith('.rs'):
            continue
        original = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + path], text=True)
        after = (REPO / path).read_text()
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', original):
            signature = 'fn ' + match[1] + '('
            assert body(after, signature) == body(original, signature), (path, signature)
            count += 1
        if Path(path).name not in ['certificate.rs', 'spatial_index.rs']:
            assert restore_separated(Path(path).name, after) == original, path
    assert count == 76, count
    print('PASS_SOURCE_ONLY: all76 old bodies byte exact; complete main restores by two index-name edits; original bounds/query/owner/geometry bodies unchanged; node builder exact without recursive sorts; two-file inverse; complete frozen4f publisher; no compiler or geometry executed')


if __name__ == '__main__':
    verify()
