# Use typed chunks in mesh decoding

Replace the manual divisibility expression with is_multiple_of and decode fixed-size vertex and float arrays with as_chunks. The existing nonempty and multiple-of-72 validation ensures both the 24-byte vertex and 4-byte float chunk operations have no remainder.

At parent 78cfaad, the Dell Linux workspace check and all 33 Rust tests pass, while Clippy fails on the three reported lints. This correction passes formatting and Markdown checks. Corrected-head workspace check, tests and Clippy are pending. The existing malformed/nonfinite and reference-mesh tests remain unchanged; imported asset bytes and rendering behavior are unchanged.

See [problem](ISSUE-mesh-decoder-lints.md).
