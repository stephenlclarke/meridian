# feat(freight): model confirmed G02 allocation and settlement

The existing freight unit demonstration lacked a connected consent and account boundary. This change adds the fixed original G02 source, actor/location/content checks, reviewed commands, identified custody, receiving receipts, wallet settlement and first-source clue in a synchronous World-owned transaction. Allocation and transfer failures preserve the complete prior domain state.

Only the freight module and its documentation change. The original isolated API remains separate. No shared World, station, desktop, Cargo or lock files are edited. See [G02-CONNECTED.md](G02-CONNECTED.md) for exact provenance, semantics and integration criteria and [the issue](ISSUE-g02-connected-freight.md) for the problem.

Validation: eleven domain tests authored; compiler, formatter, Clippy and test execution UNRUN on this source. The Mac compiler lane belongs to G1. The Dell owner must validate the final joined source. This branch is source-review material, not a playable acceptance or save-compatibility claim. Actual World admission, contact, navigation, equipment and persistence joins remain required. No main merge is requested before those joins and their verification.

Successor on main `7aee6b6d80d7c9ac32c52148a2134cc26d6ef62b` retains the frozen `50140cbd` packet, strengthens birth into actual private standard-item ownership and rejects duplicate/zero IDs before publishing an account. No caller-supplied ownership flag, paid receipt or special kit grant is added. Two further domain tests are authored, still UNRUN.
