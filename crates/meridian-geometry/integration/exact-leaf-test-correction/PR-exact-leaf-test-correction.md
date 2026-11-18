# test(geometry): bind exact leaf checks to cached parent and cavity branch

## Motivation and implementation

Correct the new batch test to use a separate cache compiled from frozen parent source, with complete setup, generations, old box collection, cached sections, candidate lifetime and cleanup. Preserve payload assertions, strict work comparison and a separate uncached semantic oracle. Keep the original convex publisher fixture and add a genuine closed uncertified cavity fixture with explicit joint-dispatch eligibility and full fields/witnesses/volume parity.

## Validation and compatibility

Production source and all 88 prior tests plus the existing sign/convex tests remain byte exact. Source inversion, formatting, Markdownlint, JSON/Python parsing, hashes and packet checks run on the author lane. Current 92 Rust/nine host/compiler/Clippy/build/changed-original checks remain UNRUN. G2's otherwise-clear whole production review carries to one bounded closure; G1 owns execution. Caps and public APIs are unchanged. See [the issue](ISSUE-exact-leaf-test-correction.md), [contract](CONTRACT.md), [delta](DELTA.json) and [custody](CUSTODY.json). No GitHub PR number is assigned.
