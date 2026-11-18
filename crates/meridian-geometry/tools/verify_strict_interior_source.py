"""Verify actual before-SUT publisher, all prior assertions and index withdrawal."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from restore_strict_interior import restore_interior
from restore_convex_sections import restore_convex_sections
from strip_budget_context import body

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def verify():
    f = CRATE / 'fixtures/strict-interior-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], name
    for name in ['certificate.rs', 'spatial_index.rs']:
        old = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':crates/meridian-geometry/src/' + name]).decode()
        assert old == (f / ('before-' + name)).read_text()
        current = (CRATE / 'src' / name).read_text()
        assert restore_interior(name, current) == old
        for match in re.finditer(r'#\[test\]\s*fn\s+(\w+)\(', old):
            signature = 'fn ' + match[1] + '('
            assert body(current, signature) == body(old, signature), signature
    old = (f / 'before-certificate.rs').read_text()
    raw = body(old, 'fn reconstruct_records(') + '\n'
    assert (f / 'reconstruct-control.rs.txt').read_text() == raw
    assert (f / 'compiled-reconstruct-control.rs').read_text() == raw.replace('fn reconstruct_records(', 'fn reference_reconstruct_records(', 1)
    current = restore_convex_sections('certificate.rs', (CRATE / 'src/certificate.rs').read_text())
    main = body(current, 'fn reconstruct_records(')
    assert 'FacetBoundsIndex::build_spatial(' not in main
    assert 'FacetBoundsIndex::spatial_memberships(' not in main
    assert main.index('audit_original_sheets(') < main.index('InteriorCertificates::build(')
    assert main.index('strictly_covers(') < main.index('let mut domain =')
    old = (f / 'before-spatial_index.rs').read_text()
    current = (CRATE / 'src/spatial_index.rs').read_text()
    for signature in ['fn spatial_memberships(', 'fn build_spatial(', 'fn spatial_range(', 'fn sort_range(', 'fn source_ordinal(', 'fn source_order<']:
        assert body(current, signature) == body(old, signature), signature
    print('PASS_SOURCE_ONLY: cd0/actual6abd publisher exact frozen control; old63 assertions; spatial bodies preserved test-only and production constructor withdrawn; source audits precede strict proof')


if __name__ == '__main__':
    verify()
