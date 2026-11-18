"""Verify source identities only; does not execute geometry or a compiler."""
import hashlib
import gzip
import json
import re
import subprocess
from strip_budget_context import strip_context
from restore_live_runs import restore_runs
from restore_spatial_index import restore_spatial
from restore_section_control import restore_section_control
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
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_section_control_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_separated_sections_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_cell_index_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_face_contact_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_convex_sections_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_source_work_refusal_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_source_work_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_interior_accounting_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_strict_interior_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_spatial_index_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_parity_index_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_proved_visits_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_incremental_run_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_live_runs_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_live_overlap_source.py")], check=True)
    subprocess.run(["python3", "-B", str(CRATE / "tools/verify_adapter_context_source.py")], check=True)
    context_parent = "41aae99bb2c0e76c21870502349e2bd894aace8f"
    for relative in ["adapter.rs", "budget.rs", "certificate.rs", "section_arrangement.rs", "sections.rs", "region.rs", "bin/original_five.rs"]:
        snapshot = CRATE / "fixtures/adapter-context-reference" / ("before-" + relative.replace("/", "-"))
        expected = subprocess.check_output(["git", "-C", str(REPO), "show", f"{context_parent}:crates/meridian-geometry/src/{relative}"])
        assert snapshot.read_bytes() == expected

    for directory in [CRATE / "fixtures", *sorted((CRATE / "fixtures").glob("*-reference"))]:
        frozen = directory / "FROZEN.json"
        if not frozen.exists():
            continue
        for name, pin in json.loads(frozen.read_text())["files"].items():
            path = directory / name
            data = path.read_bytes() if path.exists() else gzip.decompress(path.with_name(path.name + ".gz").read_bytes())
            assert hashlib.sha256(data).hexdigest() == pin["sha256"], name
            if "bytes" in pin:
                assert len(data) == pin["bytes"], name
    control = CRATE / "fixtures/section-reference"
    assert (control / "before-certificate.rs").read_text() == (
        original("src/certificate.rs").decode().split("#[cfg(test)]")[0].rstrip() + "\n"
    )
    freeze = json.loads((control / "FROZEN.json").read_text())
    for relative in [*freeze["unchangedSharedAuthority"], "src/pruning.rs", "tests/adapter.rs", "tests/original_inputs.rs"]:
        if relative not in ["src/adapter.rs", "src/budget.rs"]:
            assert restore_section_control(Path(relative).name, (CRATE / relative).read_text()).encode() == original(relative), relative

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
    actual = strip_context(restore_runs("section_arrangement.rs", (CRATE / "src/section_arrangement.rs").read_text()))
    actual = actual[actual.index("use crate::exact") :]
    assert normalized(actual) == normalized(expected), "accounted arrangement derivation"

    before = original("src/certificate.rs").decode()
    # Exact geometry restoration is checked by verify_adapter_context_source.
    after = (CRATE / "fixtures/adapter-context-reference/before-certificate.rs").read_text()
    owner_range_parent = "6c9fcb1cf5fa929d0f5d3fe32e8a98331af55235"
    owner_range_before = subprocess.check_output(["git", "-C", str(REPO), "show", f"{owner_range_parent}:crates/meridian-geometry/src/certificate.rs"], text=True)
    assert (CRATE / "fixtures/owner-range-reference/before-certificate.rs").read_text() == owner_range_before
    owner_range_start = after.index("// The caller already authenticated the owner set")
    owner_range_end = after.index("fn connected_link<const N: usize>", owner_range_start)
    after = after[:owner_range_start] + after[owner_range_end:]
    call = "    audit_original_owner_ranges(originals, &owners, b)?;"
    assert after.count(call) == 1
    after = after.replace(call, "    audit_original_components(originals, &owners, b)?;")
    owner_range_module = '\n#[cfg(test)]\n#[path = "owner_range_tests.rs"]\nmod owner_range_tests;\n'
    assert after.endswith(owner_range_module)
    after = after[:-len(owner_range_module)]
    assert after == owner_range_before, "non-owner-range source or 45 old tests changed"
    identity_parent = "dd7451091fdb3636768a3504caf91c9547f94fad"
    identity_before = subprocess.check_output(["git", "-C", str(REPO), "show", f"{identity_parent}:crates/meridian-geometry/src/certificate.rs"], text=True)
    assert (CRATE / "fixtures/identity-lifetime-reference/before-certificate.rs").read_text() == identity_before
    helper_start = after.index("// These records are part of the unchanged 40N+64")
    helper_end = after.index("/// Direct original-component adapter.", helper_start)
    after = after[:helper_start] + after[helper_end:]
    after = after.replace("    let mut input_members = Facet::input_memberships(originals, b)?;", "    let input_members = Facet::input_memberships(originals, b)?;")
    release = """    // Both authenticated identity sets have reached their last use. Their
    // original geometry/style records remain admitted for all later phases.
    input_members -= InputIdentityMaps { ids, owners }.release(b);
"""
    assert after.count(release) == 1
    after = after.replace(release, "")
    module = '\n#[cfg(test)]\n#[path = "identity_tests.rs"]\nmod identity_tests;\n'
    assert after.endswith(module)
    after = after[:-len(module)]
    assert after == identity_before, "non-identity source or 44 old tests changed"

    assert after.startswith('#[path = "sections.rs"]\nmod sections;\n\n')
    after = after.split("\n", 3)[3]
    begin = before.index("        let (barriers, coplanar) = b.with_retained(originals.len()")
    end = before.index("        drop(barriers);", begin)
    new_begin = after.index("        let (barriers, coplanar, sections) = b.with_retained(16,")
    new_end = after.index("        drop(barriers);", new_begin)
    restored = after[:new_begin] + before[begin:end] + after[new_end:]
    restored = restored.replace("    #[cfg(test)]\n    fn candidates(", "    fn candidates(")
    assert restored == before, "certificate outside section collection changed"
    # Both paths retain the exact complete field and source-tie clauses.
    clause = before[before.index("    let mut winner = face.id();") :]
    clause = clause[:clause.index("\n}\n") + 2]
    current = restore_spatial("sections.rs", (CRATE / "src/sections.rs").read_text())
    start = current.index("    let mut winner = face.id();")
    assert normalized(current[start : current.index("\n}\n", start) + 2]) == normalized(clause)
    collection_parent = "9a9f13c9758ab3523f7dde5968daa4f957eacac1"
    old_sections = subprocess.check_output(["git", "-C", str(REPO), "show", f"{collection_parent}:crates/meridian-geometry/src/sections.rs"], text=True)
    assert (CRATE / "fixtures/section-collection-reference/before-sections.rs").read_text() == old_sections.split("#[cfg(test)]")[0].rstrip() + "\n"
    old_tests = old_sections.split("#[cfg(test)]\nmod tests", 1)[1]
    new_tests = current.split("#[cfg(test)]\nmod tests", 1)[1]
    assert new_tests.startswith(old_tests[:old_tests.rfind("}")]), "42 prior tests changed"
    for relative in ["src/section_tests.rs", "src/section_arrangement.rs"]:
        reference = subprocess.check_output(["git", "-C", str(REPO), "show", f"{collection_parent}:crates/meridian-geometry/{relative}"])
        if relative.endswith("section_arrangement.rs"):
            assert normalized(strip_context(restore_runs(Path(relative).name, (CRATE / relative).read_text()))) == normalized(reference.decode())
        else:
            assert (CRATE / relative).read_bytes() == reference
    def body(source, signature):
        start = source.index(signature)
        brace = source.index("{", start)
        depth = 0
        for i in range(brace, len(source)):
            if source[i] == "{": depth += 1
            elif source[i] == "}":
                depth -= 1
                if depth == 0: return source[start:i+1]
        raise AssertionError(signature)
    for signature in ["fn meets_outward_plane(", "fn outward_candidates(", "fn limiting_segment(", "pub(super) fn build(", "fn build_admitted(", "pub(super) fn occupied(", "pub(super) fn memberships(", "pub(super) fn fields_visible("]:
        assert body(current, signature) == body(old_sections, signature), signature
    tests = sum(path.read_text().count("#[test]") for top in ["src", "tests"] for path in (CRATE / top).rglob("*.rs"))
    assert tests == 84, tests
    print(json.dumps({"status": "PASS_SOURCE_ONLY", "parent": PARENT, "currentRustTests": tests, "compilerOrGeometryExecuted": False}))


if __name__ == "__main__":
    verify()
