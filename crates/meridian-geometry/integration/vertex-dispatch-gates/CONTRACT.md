# Production joint-dispatch compilation gates

## Finding and correction

G2's complete Root `c05156ff` review found one P1 compile defect: production `collect_cached_admitted` calls `collection_candidates`, which delegates to `collection_with_support`, but both methods still had `cfg(test)`. Normal library compilation cannot resolve the methods, even when a runtime input would choose a different branch. This is a source-established defect; no compiler was run by G2 or the author.

Remove only those two attributes. Both complete method bodies, numerical predicates, exact prices, eligibility, owner/outward flags, source order and all other compilation gates remain unchanged. The cached collector and both runtime branches are byte exact. No uncertified-owner branch is bypassed, and physical-only dispatch is not forced. No suppression or control removal is added.

## Preservation and review binding

The complete parent sections file is frozen and hashed. Reinserting the two attributes restores that file byte for byte. All 88 test bodies and every other source/test file remain exact. The inverse runs before the existing distance and historical proofs. No new behavior test is needed for a compilation attribute correction; the normal all-target check is owned by G1.

G2's complete algorithm, math, pricing, admission, lifetime and independent-control review is otherwise clear and explicitly carries to this bounded correction. The review, Root source identity and three full payloads are hashed in custody; all 482 parent crate files match the whole-reviewed Root source. Current review acceptance still requires one bounded delta/unaffected-source closure.

## Qualification

All 88 current Rust tests, nine host checks, compiler/Clippy/build and changed original-five remain UNRUN. Author checks are limited to rustfmt, Markdownlint, JSON/Python parsing, complete source inverse/body proofs, hashes and packet integrity. No author compiler, Cargo, Clippy, probe, SUT, geometry or native execution occurs.

Root joins only the successor since `da8c316`, preserving Root documentation and G3 work. One G2 bounded delta/unaffected-source closure carries the completed whole review. Existing G1 then owns changed 88 Rust/nine host/locked check/strict Clippy/development build, followed by one changed whole-five only after all pass and the sole execution lane is available. Caps remain 2,000,000 operations including decode, 100,000 logical memberships, 4096 bits per coordinate or UV rational and 512 complete triangles. Same five inputs and 25 bodies; broader 202 matrix remains separate. No unchanged or diagnostic retry, new fleet admission or current original benefit/fit/minimum/speed claim. Baseline remains 23/162 ships and 4/6 stations.
