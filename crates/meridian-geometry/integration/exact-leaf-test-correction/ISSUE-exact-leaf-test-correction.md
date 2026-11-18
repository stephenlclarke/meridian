# Issue: exact-leaf tests need current-parent work and joint-branch coverage

## Motivation and required behavior

G2 findings `G2-GEOMETRY-EXACT-LEAF-1` and `G2-GEOMETRY-EXACT-LEAF-2` require an already-cached independent parent baseline and complete publisher coverage of an eligible uncertified owner. The old uncached baseline and disjoint convex fixture do not establish these obligations.

## Correction and validation

Include both complete cached work lanes and keep the uncached semantic oracle separate. Preserve the convex fixture and add a closed cavity fixture with explicit branch preconditions and complete publisher parity. All production code and 90 existing test bodies remain exact. See [the contract](CONTRACT.md) for scope and G1 qualification. All current runtime checks remain UNRUN. Tracking uses the task slug; no GitHub issue number is assigned.
