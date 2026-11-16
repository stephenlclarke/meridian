# Safe-gunnery attempt closure and retry

## Current source and delivery boundary

[Gunnery Attempts Preview v0.2.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.2.0-preview.1) delivers reviewed closure, fresh consent and bounded retained attempt history in save v9. Qualified main `16d6ddb` and compiled runtime `8d035a8` share all 131 tracked blobs. Mac, Linux and genuine native Windows passed 208 top-level Rust tests and 74 Python tests (54 importer plus 20 publisher), strict Clippy and release builds; exact-main CI and Sonar passed with zero open findings. The signed local Mac app and public macOS/Windows packages are delivered. Public downloads and the extracted Mac Developer ID signature were independently verified. Mac Metal evidence covers affected closure/retry/history/HUD and Save/Continue using copied owned fixtures, with a later affected panel check; exact source boundaries and excluded captures remain in [implementation status](IMPLEMENTATION-STATUS.md). Use the supplied dedicated saves-v9 launcher. Earlier previews and real profiles remain separate. macOS is not notarized; Windows is unsigned and interactive Windows gameplay remains unobserved.

This is a scoped attempt lifecycle. It does not establish full T02, a genuine live player hit, whole original-game parity or 64-mission acceptance.

## Immutable source

The implementation follows the board and retained-state rules in immutable L337 commit `a11fe16924d87d06021816f9037952afcbac08db`: `NativeSafeGunneryBoard.swift`, `NativeSafeGunneryReception.swift`, `NativeSafeGunneryState.swift`, `NativeSafeGunnerySave.swift`, `NativeSafeGunneryRuntime.swift`, `NativeSafeGunneryProgress.swift`, `NativePracticeState.swift`, `NativePracticeSave.swift`, `OriginalServiceReservationPolicy.swift`, `ScriptClueJournal.swift` and `docs/meridian/missions/gameplay/T02-SAFE-GUNNERY.md`. Their exact Git-blob hashes are retained in the source packet. No human working checkout or player profile is used.

The board provides consent, verified reception and close/cancel. A cancel confirms failed when the attempt has an ordinary hit and abandoned otherwise. All settlements are zero cash. Only verified completion can record the first trusted family trace. The journal keeps that first award when later attempts complete. For this requested slice, failure remains on the open attempt until explicit reviewed Avenor closure.

## Commands and review validity

`review_practice` and `prepare_practice_confirmation` remain the consent boundary. An open attempt prevents another acceptance. A completed or explicitly closed attempt permits a fresh review; confirming it archives the previous full record and allocates a distinct opaque `PracticeAttemptId`. Old records retain their original consent, range IDs, first-hit origins, ordinary failure, closure/reception and timestamps.

`review_practice_closure` and `prepare_practice_closure` require the current actual Avenor main-station berth, original owner/carrier and an open attempt. The reviewed outcome is descriptive: the World derives failed or abandoned from its retained ordinary-hit state. The caller cannot choose an outcome, submit a contact or request a refund.

All three review paths bind actual dock, owner/carrier, current tick admission, session epoch, attempt identity and a fingerprint of the retained practice graph, journal and relevant account/freight/weapon state. A stale or foreign confirmation refuses. Pause refuses review and preparation. Cold Continue invalidates old UI sequences. Consent, reception and closure each return the existing exclusive-borrow `PreparedPracticeConfirmation`: publish its checked bytes atomically, then commit. Dropping it changes no live state. Qualified desktop host tests cover actual write failure and prior-file preservation; native scope is recorded separately in implementation status.

Completion already constitutes a separately confirmed terminal operation. Closing an incomplete attempt records abandoned; closing an ordinary-hit attempt records failed. Neither grants money or a clue. Terminal records have a separate outcome tag checked against their exact closure or reception receipt. Dropping a receipt, contradicting its outcome or resurrecting an archived record is rejected. Launching after closure without new consent does not reopen a provider.

## Capacity and identity

The Avenor board retains at most 16 accepted offers, including completed, failed and abandoned attempts. IDs use owner plus the retained zero-based attempt ordinal. Records must remain contiguous, ordered and terminal before the next consent. Each actual new sortie also has distinct original range IDs through its new navigation visit; no range exists for an attempt closed before launch.

The original limits of 128 native mission records and three active Original contracts/services are checked using the owners currently implemented: actual retained G02 allocations plus practice records, and actual active G02 custody plus the open practice attempt. Those two owners can occupy at most two active slots; the 16-offer board bound is reached before the 128-record bound. No unimplemented mission or service occupancy is fabricated to exercise a capacity claim.

Retry retains the current ship, cash, cargo, paid gear, legal value and weapon state. It issues no weapons or ammunition. The existing three physical starter rounds and three globally retained projectile slots span all attempts. A new consent does not reset, refund, replace or recycle any of them. A physical berth retains its existing cooldown-reset behavior; all item and ended-projectile history remains.

## Version 9 and invariant joins

Version 9 adds a bounded archive of earlier full practice records, the explicit first-trace attempt pointer, and each practice's attempt ordinal, terminal outcome and optional closure receipt. The current practice remains available through existing provider/reception snapshots, and `practice_ledger_snapshot` exposes immutable current/history outcomes, remaining offer capacity and derived occupancy. The first trace comes from the saved journal pointer to its original verified reception; snapshots do not award clues by scanning completions.

Version 4-8 rules headers and wire readers remain explicit. The exact version 8 account, ordnance, combat and resource codecs remain unchanged. Earlier formats contain at most their one existing practice record, which receives ordinal zero. An existing saved completion already carried its trusted-trace proof; the reader retains that terminal state and pointer from the proof. It invents no failed/abandoned closure, additional offer, missing missile origin or used-v7 migration. Version 9 is required to write retry history.

Each old projectile is validated against its own retained attempt and range, rather than the newest practice. First-hit, pending/end quiet and ordinary incident checks are scoped to that same attempt. All global pylon ownership, projectile/emission identity, motion/contact evidence, law and resource history stay in the existing World-owned controllers. Historical damage is separated from damage since the last physical berth when checking current pools and cooldown.

The reader validates bounded counts, ordered IDs and chronology, actual historical docks, terminal receipts/outcomes, the first journal pointer and all linked ordnance before replacing a live World. Bad counts, missing or duplicated records, stale/foreign targets, forged closures and sequence exhaustion refuse without repair or event replay. The checksum is corruption detection, not authentication against a party rewriting an entire coherent history.

## Authored coverage and execution criteria

Nine new tests exercise an actually market-funded paid hull, missile station hit, law, swept Avenor return, failed closure and fresh missile range with old custody retained; dropped unpublished preparations and stale/pause/foreign confirmations; two completed attempts with the same first clue and zero extra money; all 16 real board offers and actual shared G02 occupancy; wrong-berth refusal with public charged travel; checksummed forged history/outcome/capacity/time refusals; genuine Continue sequence exhaustion; exact v8 live/completed upgrades; and inert closed attempts launched without fresh consent. Existing tests and assertions remain in place. Only test-helper visibility is widened for reuse of their genuine public flight and funding paths.

The desktop Session/Profile join binds accept/receive/close rows and confirmation details to the actual active attempt, shows closed history and the retained first clue, and keeps the existing lease, temporary write/flush and atomic replacement rules. Use isolated saves-v9 profiles. Qualify new-process checkpoints after failed/incomplete closure, fresh consent, live retry projectile and second verified completion. Retain the original profile and bind every check, output and candidate executable to the reviewed joined commit/tree. G1 remains the sole Mac compiler/native owner; the existing Dell owners provide Linux and native Windows execution after whole review. Source formatting and authored tests are not execution acceptance.

## Desktop controls

F7 reviews closure of the current open attempt at the actual Avenor berth. The review names failed or abandoned, explains retained history and no reward/refund/refill, and requires Enter to save and confirm; B cancels. The same atomic publisher handles consent, reception and closure. A failed publication leaves the live World unchanged and consumes the pending review, requiring a fresh review.

After a terminal attempt, F10 reviews fresh consent subject to current World capacity and berth checks. F8 remains reception for the current attempt. An old global trace never makes a new attempt appear complete. I shows bounded outcome counts, remaining offers and the original once-only trace status. Normal flight retains its existing HUD row budget. Current host tests author a genuine child-process abandoned closure and explicit fresh consent, prior valid slot preservation on failed closure publication, and closure/history presentation. Failed-flight closure, live retry and second reception have scoped native/cold evidence on their recorded qualified sources; this does not qualify unrelated full T02 behavior.
