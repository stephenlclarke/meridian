# fix(geometry): filter outward leaves by exact triangle support

## Motivation and implementation

Use the active source-plane distance cache to reject empty outward triangle sections at leaf eligibility, replacing the leaf box support predicate. Keep node boxes, owners, physical flags, source order, complete section geometry and small-input collection. Admit all added scratch and retain original numeric prices. Full prior collector/index/publisher controls remain independent.

## Validation and compatibility

All 88 prior test bodies remain exact. Three new authored tests cover complete setup/query work, all signed patterns/zero contacts/resource guards and full publisher fields/witnesses/volume. Source proofs, formatting, Markdownlint, JSON/Python parsing, custody and packet checks run on the author lane; all 91 Rust/nine host/compiler/Clippy/build/changed-original checks remain UNRUN. Public APIs and caps remain unchanged. Original benefit and admission are unproven. See [the issue](ISSUE-exact-leaf.md), [contract](CONTRACT.md), [delta](DELTA.json) and [custody](CUSTODY.json). No GitHub PR number is assigned.
