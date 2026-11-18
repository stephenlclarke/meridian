# fix(geometry): compile joint collection dispatch in production

## Motivation and implementation

Remove `cfg(test)` from `collection_candidates` and `collection_with_support`, which production cached collection calls. Both complete bodies and all other source/test code remain byte exact. Keep uncertified owner/outward dispatch and original pricing.

## Validation and compatibility

Complete frozen-file inversion, all 88 test-body preservation, formatting, Markdownlint, JSON/Python parsing and packet checks are author-only. Compiler, tests, Clippy, build and originals remain UNRUN. G2's completed whole algorithm review is otherwise clear and carries to one bounded closure; G1 owns normal compilation and changed validation. Caps and public APIs are unchanged. See [the issue](ISSUE-vertex-dispatch-gates.md), [contract](CONTRACT.md), [delta](DELTA.json) and [custody](CUSTODY.json). No GitHub PR number is assigned.
