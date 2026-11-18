"""Verify the material adapter edit and restore context-only source changes."""
from pathlib import Path
import gzip
import re

from strip_budget_context import body, remove_statements, strip_context


def normalized(text):
    return re.sub(r"\s+", "", text)


def verify():
    crate = Path(__file__).resolve().parents[1]
    reference = crate / "fixtures/adapter-context-reference"
    for name in ["sections.rs", "section_arrangement.rs", "region.rs", "certificate.rs"]:
        old = (reference / ("before-" + name)).read_text()
        restored = strip_context((crate / "src" / name).read_text())
        if name == "certificate.rs":
            start = restored.index("pub fn reconstruct(")
            end = restored.index("fn reconstruct_records(", start)
            restored = restored[:start] + restored[end:].replace(
                "fn reconstruct_records(", "pub fn reconstruct(", 1
            )
        assert normalized(old) == normalized(restored), name

    old_budget = (reference / "before-budget.rs").read_text()
    current_budget = (crate / "src/budget.rs").read_text()
    for signature in [
        "pub fn step(",
        "pub fn guard(",
        "pub(crate) fn retain(",
        "pub(crate) fn with_retained<T>(",
        "pub(crate) fn release(",
    ]:
        old = body(old_budget, signature)
        restored = remove_statements(
            body(current_budget, signature), r"\bself\.record_rejection\("
        )
        restored = restored.replace("let before = self.operations;", "")
        restored = restored.replace("let caller = std::panic::Location::caller();", "")
        restored = re.sub(
            r"self.phase_operations\[self.phase as usize\]\s*=.*?;",
            "",
            restored,
            flags=re.S,
        )
        restored = re.sub(
            r"\.ok_or_else\(\|\|\s*\{\s*(self.fail\(.*?\))\s*\}\s*\)",
            r".ok_or_else(||\1)",
            restored,
            flags=re.S,
        )
        assert normalized(old) == normalized(restored), signature

    assert current_budget[:current_budget.index("/// Fixed control state")] == (
        old_budget[:old_budget.index("#[derive(Debug)]\npub struct Budget")]
    )
    constructor = body(current_budget, "pub fn new(")
    for field in [
        "phase: BudgetPhase::Other,",
        "state: [0; 8],",
        "state_at_operations: 0,",
        "phase_operations: [0; 18],",
        "rejection: None,",
    ]:
        constructor = constructor.replace(field, "")
    assert normalized(constructor) == normalized(body(old_budget, "pub fn new("))

    adapter = (reference / "before-adapter.rs").read_text()
    compiled = adapter.split("#[cfg(test)]")[0]
    for asset in ["SH010", "SH011", "SH016", "SH017", "SH166"]:
        compiled = compiled.replace(
            f'include_bytes!("../fixtures/{asset}.facets")',
            f'include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/{asset}.facets"))',
        )
    frozen_compile = gzip.decompress((reference / "compiled-before-adapter.rs.gz").read_bytes()).decode()
    assert compiled == frozen_compile
    assert (reference / "normalized-before-adapter.rs").read_text() == frozen_compile.rstrip() + "\n"

    restored = normalized(strip_context((crate / "src/adapter.rs").read_text()))
    wrapper = normalized(
        """pub fn decode(self, bytes: &[u8], b: &mut Budget) -> Result<Vec<Facet>, Failure> {
            self.decode_records(bytes, b)
        }
        fn decode_records"""
    )
    assert restored.count(wrapper) == 1
    restored = restored.replace(wrapper, "pubfndecode", 1)
    replacements = {
        "Vec<(&str,Arc<str>)>": "Vec<Arc<str>>",
        "style_members+4*styles.len()+line.len()": "style_members+styles.len()+line.len()",
        "+4*(styles.len()+1)": "+styles.len()+1",
        "letmutshared_style=None;for(encoded,style)in&styles{":
            "lettext_style=decode_style(fields[2],b)?;letmutshared_style=None;forstylein&styles{",
        "if*encoded==fields[2]{": "ifstyle.as_ref()==text_style.as_str(){",
        "None=>{lettext_style=decode_style(fields[2],b)?;b.guard(":
            "None=>{b.guard(",
        "styles.push((fields[2],Arc::clone(&style)));":
            "styles.push(Arc::clone(&style));",
    }
    for current, original in replacements.items():
        assert restored.count(current) == 1, current
        restored = restored.replace(current, original, 1)
    assert restored == normalized(adapter), "material adapter edit exceeded its contract"

    for name in ["adapter.rs", "bin/original_five.rs"]:
        old = (reference / ("before-" + name.replace("/", "-"))).read_text()
        current = (crate / "src" / name).read_text()
        old_tests = old.split("#[cfg(test)]\nmod tests", 1)[1]
        current_tests = current.split("#[cfg(test)]\nmod tests", 1)[1]
        assert current_tests.startswith(old_tests[:old_tests.rfind("}")]), name

    print(
        "PASS: complete geometry and primitive charges restore from context annotations; "
        "adapter restores from the specified canonical-style edit; 47 prior tests preserved."
    )


if __name__ == "__main__":
    verify()
