# refactor(tools): extract ship source position validation

`convert` mixed nested triangle expansion with index and finite-position checks, triggering Sonar's cognitive complexity rule. `_checked_position` now performs those checks in their existing order and returns the same source position. The conversion loop keeps the same triangle/material/vertex order, palette conversion and binary packing.

Validation:

- All 17 existing importer tests pass before and after the change; test sources are unchanged. They cover malformed notices/materials/indices/positions, ordered conversion, both sRGB branches, command-line writes/checks and refusal without overwriting.
- Converting the exact pinned SH025 source with both the base and candidate importer produces identical bytes matching the committed mesh: 250,848 bytes, 3,484 triangles, SHA-256 `b85d726da742da00a68643429fc109b28343e164601aa013abb9aa045717c6d7`.
- Source revision, path and hash constants are unchanged. No assets, Rust files, dependencies, licence metadata or provenance claims change.
- Whitespace and scoped Markdown lint pass. Remote Sonar analysis is pending; no rule suppression or quality waiver is added.

See [the issue](ISSUE-import-ship-complexity.md). The change is prepared on an isolated branch from main `e20ec88265700354610c667768886cb00bc36903`; Root owns review and integration.
