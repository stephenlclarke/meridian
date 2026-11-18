"""Check fixture intent, unchanged assertions and bounded import removal only."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from strip_budget_context import body, end_delimiter
from restore_cell_index import restore_cell_index

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def normalized(value):
    return re.sub(r"\s+", "", value)


def assertions(value):
    result = []
    for match in re.finditer(r'\bassert(?:_eq|_ne)?!\(', value):
        opening = value.index('(', match.start())
        end = end_delimiter(value, opening)
        result.append(normalized(value[match.start():end + 1]))
    return result


def verify():
    f = CRATE / 'fixtures/face-contact-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    inverse = json.loads((CRATE / 'integration/face-contact-correction/INVERSE-EDITS.json').read_text())
    for name, pin in pins['files'].items():
        data = (f / name).read_bytes()
        assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256']
        relative = 'crates/meridian-geometry/src/' + name.removeprefix('before-')
        assert data == subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + relative])
    for name, edits in inverse.items():
        lines = (CRATE / 'src' / name).read_text().splitlines(True)
        for edit in reversed(edits):
            a, z = edit['afterStart'], edit['afterEnd']
            assert ''.join(lines[a:z]) == edit['after']
            lines[a:z] = edit['before'].splitlines(True)
        assert ''.join(lines) == (f / ('before-' + name)).read_text()
    old_math = (f / 'before-convex_sections.rs').read_text()
    math = (CRATE / 'src/convex_sections.rs').read_text()
    assert old_math.count('integer') == 1
    assert old_math.replace('Rational, integer', 'Rational', 1) == math, 'production math or guards changed'
    before = (f / 'before-convex_section_tests.rs').read_text()
    after = (CRATE / 'src/convex_section_tests.rs').read_text()
    names = lambda text: re.findall(r'#\[test\]\s*fn\s+(\w+)\(', text)
    assert names(after)[:4] == names(before) and len(names(after)) == 5
    for name in names(before):
        old = body(before, 'fn ' + name + '(')
        new = body(after, 'fn ' + name + '(')
        assert assertions(old) == assertions(new), ('original assertion changed', name)
        assert 'failure_context(' in new and '"case-return"' in new
    helper = body(after, 'fn failure_context<T>(')
    assert 'run: impl FnOnce() -> Result<T, Failure>' in helper
    assert 'match run()' in helper and 'Err(failure)' in helper and 'Ok(value) => Ok(value)' in helper
    assert 'CONVEX_SECTION_FAILURE fixture={fixture} lane={lane}' in helper
    assert helper.count('Err(failure)') == 2
    neighbour = normalized(body(after, 'fn face_neighbour('))
    assert 'cube(0,8,1,0)?' in neighbour and 'vertex.point[0]+=integer(8);' in neighbour and 'vertex.uv[0]+=integer(8);' in neighbour
    assert neighbour.count('vertex.point[') == neighbour.count('vertex.uv[') == 1
    positive = normalized(body(after, 'fn convex_section_rows_preserve_complete_publisher_fields_contacts_and_volume('))
    assert '_=>face_neighbour()?,' in positive
    assert 'failure_context(label,"reference-publisher",||{' in positive
    assert 'failure_context(label,"candidate-publisher",||{' in positive
    negative = normalized(body(after, 'fn corner_touch_retains_independent_reference_and_candidate_contact_star_refusals('))
    assert 'letmutoriginals=cube(0,8,0,0)?;originals.extend(cube(8,16,1,0)?);' in negative
    assert 'corner_refusal("reference-publisher",reference)' in negative and 'corner_refusal("candidate-publisher",candidate)' in negative
    assert 'failure.kind,FailureKind::UncertifiedContact' in negative
    assert 'failure.source,Some(SourceId{component:1,facet:11})' in negative
    assert 'failure.detail,"disconnectedcontact-starlink"' in negative
    refusal = normalized(body(after, 'fn corner_refusal<T>('))
    assert 'Err(failure)=>failure' in refusal and 'Ok(_)=>panic!' in refusal
    assert '.unwrap' not in after and '.expect' not in after and '#[allow' not in after
    # Every pre-existing file in src/tests except the two bounded files is exact.
    for relative in subprocess.check_output(['git', '-C', str(REPO), 'ls-tree', '-r', '--name-only', pins['parent'], '--', 'crates/meridian-geometry/src', 'crates/meridian-geometry/tests'], text=True).splitlines():
        if Path(relative).name not in inverse:
            assert restore_cell_index(Path(relative).name, (REPO / relative).read_text()).encode() == subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + relative]), relative
    print('PASS_SOURCE_ONLY: complete inverse exact; production only unused import removal; all71old bodies and four prior convex assertion sequences preserved; x-only fullface fixture; exact old corner fixture retained as independent checked reference/candidate refusal; every returned Failure labelled without mutation')


if __name__ == '__main__':
    verify()
