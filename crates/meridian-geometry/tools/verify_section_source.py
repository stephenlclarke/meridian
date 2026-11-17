"""Verify source identities only; does not execute geometry or a compiler."""
import hashlib
import json
import re
import subprocess
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[1]
PARENT = "44bb43552432ceb66d8c175e08dc6380ed017273"


def original(relative):
    return subprocess.check_output(
        ["git", "-C", str(REPO), "show", f"{PARENT}:crates/meridian-geometry/{relative}"]
    )


def normalized(value):
    return re.sub(r"\s+", "", value)


def verify():
    for directory in [CRATE / "fixtures", *sorted((CRATE / "fixtures").glob("*-reference"))]:
        frozen = directory / "FROZEN.json"
        if not frozen.exists():
            continue
        for name, pin in json.loads(frozen.read_text())["files"].items():
            data = (directory / name).read_bytes()
            assert hashlib.sha256(data).hexdigest() == pin["sha256"], name
            if "bytes" in pin:
                assert len(data) == pin["bytes"], name
    control = CRATE / "fixtures/section-reference"
    assert (control / "before-certificate.rs").read_text() == (
        original("src/certificate.rs").decode().split("#[cfg(test)]")[0].rstrip() + "\n"
    )
    freeze = json.loads((control / "FROZEN.json").read_text())
    for relative in [*freeze["unchangedSharedAuthority"], "src/pruning.rs", "src/adapter.rs", "src/region.rs", "src/bin/original_five.rs", "tests/adapter.rs", "tests/original_inputs.rs"]:
        assert (CRATE / relative).read_bytes() == original(relative), relative

    expected = original("src/arrangement.rs").decode()
    start = expected.index("pub type Segment =")
    end = expected.index("#[derive(Debug)]\nstruct Run")
    expected = expected[:start] + expected[end:]
    expected = expected.replace(
        "use crate::{Budget, Failure, FailureKind, Point2, Rational, integer};",
        "use crate::{Arrangement, ArrangementWork, Budget, Cell, Failure, FailureKind, Point2, Rational, Segment, integer,};",
    ).replace("pub fn build<F>(", "pub(crate) fn build_accounted<F>(")
    old_callback = """                if !visible(&sample, b)? {
                    continue;
                }"""
    new_callback = """                let exposed = b.with_retained(
                    base + 16 * (members + levels.len() + following.len() + active.len() + 1),
                    |local| visible(&sample, local),
                )?;
                if !exposed {
                    continue;
                }"""
    assert expected.count(old_callback) == 1
    expected = expected.replace(old_callback, new_callback)
    actual = (CRATE / "src/section_arrangement.rs").read_text()
    actual = actual[actual.index("use crate::exact") :]
    assert normalized(actual) == normalized(expected), "accounted arrangement derivation"

    before = original("src/certificate.rs").decode()
    after = (CRATE / "src/certificate.rs").read_text()
    assert after.startswith('#[path = "sections.rs"]\nmod sections;\n\n')
    after = after.split("\n", 3)[3]
    begin = before.index("        let arrangement = b.with_retained(coplanar.len()")
    end = before.index("        drop(barriers);", begin)
    new_begin = after.index("        // Whole-owner one-sided sections")
    new_end = after.index("        drop(barriers);", new_begin)
    restored = after[:new_begin] + before[begin:end] + after[new_end:]
    assert restored == before, "certificate outside section collection changed"
    # Both paths retain the exact complete field and source-tie clauses.
    clause = before[before.index("    let mut winner = face.id();") :]
    clause = clause[:clause.index("\n}\n") + 2]
    current = (CRATE / "src/sections.rs").read_text()
    start = current.index("    let mut winner = face.id();")
    assert normalized(current[start : current.index("\n}\n", start) + 2]) == normalized(clause)
    tests = sum(path.read_text().count("#[test]") for top in ["src", "tests"] for path in (CRATE / top).rglob("*.rs"))
    assert tests == 42, tests
    print(json.dumps({"status": "PASS_SOURCE_ONLY", "parent": PARENT, "currentRustTests": tests, "compilerOrGeometryExecuted": False}))


if __name__ == "__main__":
    verify()
