# Checked refusal extraction

## Finding and correction

G2-GEOMETRY-SOURCE-WORK-1 identified a static compiler requirement and lint violation in seven new refusal assertions. Result::unwrap_err requires its success type to implement Debug; the certified collector returns a tuple containing Frame, which has no such implementation. The workspace also denies unwrap_used. No compiler or lint ran on the reviewed source. The complete G2 review is retained in PARENT-REVIEW.json with its independently checked hash and Root source binding. All other math, work accounting, references, old tests and implementation were clear in that review.

Replace all seven calls with a private generic checked_failure helper in the same test module. An Err returns the same typed Failure unchanged. Every Ok enters an unconditional panic, regardless of its type or value. The helper has no success-type trait bound. Both sides of the invalid-domain comparison separately require Err, so two successful results can never compare equal. No production Debug derive, lint suppression, expect_err, optional-error comparison or waiver is introduced.

## Preservation proof

Freeze the complete reviewed test file from author e2341aa before editing. The source-only verifier removes the one helper and inverts all seven wrappers to their original unwrap_err expressions. After whitespace normalization the entire test file has exactly its original tokens: every argument, refusal kind, comparison, strict work predicate, timing bound, geometry, retention guard, callback and assertion is retained. It separately checks the exact exhaustive helper body, seven call count and four unchanged case names. Every other existing source or test file is byte exact to e2341aa. The full previous source proof still compares all 67 old bodies and frozen controls. This checks the unexpected-success failure branch from source; it does not execute the helper or geometry.

## Review and execution

All 387 parent crate pins match whole-reviewed Root c451a394. Carry the whole G2 review and close this single bounded test extraction finding. The 71 Rust and nine host tests, compiler, strict Clippy, development build and original-five batch remain UNRUN here. After Root joins only this successor delta and G2 closes it, existing G1 owns focused checks and one changed whole-five batch only after every check passes and the sole lane is available. Original 2M operations including decode, 100k memberships, 4096-bit rational and 512 complete output triangles, all 322/108/120 assertions and the separate 202-case matrix remain unchanged. No new candidate, fleet admission, native acceptance or performance claim occurs.
