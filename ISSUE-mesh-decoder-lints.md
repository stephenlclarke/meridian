# Mesh decoder lint failures

The Dell's Linux workspace check and all 33 Rust tests pass at 78cfaad, but Clippy with warnings denied fails on manual_is_multiple_of and two chunks_exact_to_as_chunks findings in the mesh decoder.

Use the pinned toolchain's typed slice chunks and divisibility predicate. Preserve empty input, complete-triangle and finite-value validation, byte order and decoded output. Do not suppress lints or weaken tests.

See [change details](PR-mesh-decoder-lints.md).
