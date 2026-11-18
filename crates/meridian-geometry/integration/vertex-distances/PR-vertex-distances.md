# fix(geometry): reuse exact vertex distances per source plane

## Motivation and implementation

Reuse exact original XYZ distances within one exclusive original-source-plane generation for inputs with at least 256 facets. Charge complete ID/map setup, scalar storage, generations, lazy resets and misses. Use the original row/physical collector on this path; retain the qualified finite-gap path below the threshold. Preserve per-facet identity, winding, style, affine UVs and all existing geometry guards.

## Validation

All 84 prior test bodies remain exact. Four new tests cover full setup work, plane isolation, coordinate/resource refusal and complete publisher parity. Source inversion, custody, rustfmt, Markdownlint, JSON/Python parsing and patch checks run on the author lane. All 88 Rust tests, nine host checks, compiler, Clippy, build and changed originals are UNRUN; execution belongs to G1 after G2 review.

## Compatibility, links and remaining risk

Public APIs and all caps remain unchanged. Extra setup/storage may cause earlier refusal; original benefit and acceptance are unproven. See [the issue](ISSUE-vertex-distances.md), [contract](CONTRACT.md), [delta](DELTA.json) and [custody](CUSTODY.json). No GitHub PR number is assigned. Root integrates only the successor since the bound parent and preserves its existing documentation and G3 work.
