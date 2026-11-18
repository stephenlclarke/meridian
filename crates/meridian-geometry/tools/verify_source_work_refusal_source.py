"""Prove the bounded test-only correction without executing Rust or geometry."""
import hashlib
import json
import re
import subprocess
from pathlib import Path
from strip_budget_context import body, end_delimiter

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]


def normalized(source):
    return re.sub(r"\s+", "", source)


def verify():
    f = CRATE / 'fixtures/source-work-refusal-reference'
    pins = json.loads((f / 'FROZEN.json').read_text())
    old = (f / 'before-source_work_tests.rs').read_text()
    data = old.encode()
    pin = pins['files']['before-source_work_tests.rs']
    assert len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256']
    path = 'crates/meridian-geometry/src/source_work_tests.rs'
    assert old == subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + path], text=True)
    current = (REPO / path).read_text()
    helper = body(current, 'fn checked_failure<T>(')
    expected_helper = """fn checked_failure<T>(result: Result<T, Failure>) -> Failure {
        match result {
            Err(failure) => failure,
            Ok(_) => panic!("unexpected success in refusal assertion"),
        }
    }"""
    assert normalized(helper) == normalized(expected_helper), 'all unexpected Ok values must fail; no T:Debug bound'
    assert '.unwrap_err(' not in current and '.expect_err(' not in current and '#[allow' not in current
    restored = current.replace(helper, '', 1)
    calls = list(re.finditer(r'\bchecked_failure\(', restored))
    assert len(calls) == old.count('.unwrap_err()') == 7
    for match in reversed(calls):
        opening = restored.index('(', match.start())
        end = end_delimiter(restored, opening)
        expression = restored[opening + 1:end]
        restored = restored[:match.start()] + expression + '.unwrap_err()' + restored[end + 1:]
    assert normalized(restored) == normalized(old), 'refusal arguments, kinds, comparisons or any other test token changed'
    names = lambda value: re.findall(r'#\[test\]\s*fn\s+(\w+)\(', value)
    assert names(current) == names(old) and len(names(current)) == 4
    for relative in subprocess.check_output(['git', '-C', str(REPO), 'ls-tree', '-r', '--name-only', pins['parent'], '--', 'crates/meridian-geometry/src', 'crates/meridian-geometry/tests'], text=True).splitlines():
        if relative != path:
            expected = subprocess.check_output(['git', '-C', str(REPO), 'show', pins['parent'] + ':' + relative])
            assert (REPO / relative).read_bytes() == expected, relative
    print('PASS_SOURCE_ONLY: seven checked failure wrappers invert to exact parent tokens; helper has no T bound and every Ok panics; paired invalid-domain errors cannot pass on two successes; all other production and test files byte exact; four case names unchanged')


if __name__ == '__main__':
    verify()
