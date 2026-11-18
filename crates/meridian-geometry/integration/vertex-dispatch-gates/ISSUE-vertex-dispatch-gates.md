# Issue: production joint helpers are test-only

## Motivation and required behavior

G2 finding `G2-GEOMETRY-VERTEX-DISTANCES-1` identifies two `cfg(test)` attributes that exclude required helpers from normal library compilation. Production collection must retain both full dispatch branches and their original semantics.

## Correction and validation

Remove only those two attributes. Retain every method body, price, guard, source order and all 88 tests. See [the contract](CONTRACT.md) for exact inverse proof, whole-review custody and G1 normal compilation qualification. Current compiler and runtime validation remain UNRUN. Tracking uses the task slug; no GitHub issue number is assigned.
