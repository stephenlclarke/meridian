# Ordinary impacts and T02 failure

This core predecessor starts at exact main `51dcc129351fd216724366f27cd6af474b4a42c5`. It binds the existing actual front Pulse command to ordinary main-station impacts and permanent T02 failure. Source and tests are authored; compilation, tests, strict Clippy and desktop acceptance are unrun in the author lane.

## Immutable original rules

The source is L337 `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`.

| Owner | Retained rule |
| --- | --- |
| `GameplayOrdnance.swift:394-432,489-545` | Reserved practice contacts bypass ordinary combat. Other actual nearest swept contacts use ordinary combat; the owner becomes collidable after muzzle clearance. |
| `GameplayCombat.swift:155-191,293-318` | An actual accepted emission/incident carries owner, victim, physical point/time and legal effects. Player impact selects front at local contact z >= 0, subtracts shield first, then residual energy, including fatal damage. |
| `LaserSpecification.swift:14-20`, `ShieldBooster.swift:76-94`, `OriginalFleetR3Data.swift:1036-1053`, `Game.swift:6935-6942` | Stock SH025 has 300 shield units on each side, 1.8 shield units/second recharge, energy 380 and energy recharge 4.2/second. Laser and damage share that energy pool. Pulse impact damage is 15 units; its one-unit discharge energy cost is separate. |
| `Geometry.swift:115,132`, `NativeStationRuntime.swift:95-125,820-830`, `Game.swift:3534-3545`, `GameplayCombat.swift:535-550` | The current original main station is entity 1/id 0 with no authored `nativeStationIdentity`. The damage guard requires that authored identity, so this original main actor admits no hull damage. Authored structural station profiles are separate actors. |
| `CrimeLedger.swift:488-529,539-543,604-629,775-778` | Station attacks are unlawful, carry a 64-point penalty on the first attack in a window, reuse a window within ten physical seconds inclusive and saturate legal value at 255. |
| `CrimeLedger.swift:230-265,407-413` | A self-hit can be an accepted incident and damage; it is not an eligible crime victim and receives no self-assault penalty. |
| `NativeSafeGunneryState.swift:24-40,162-222`, `NativeSafeGunneryRuntime.swift:105-119` | Original ordinary hit prevents readiness permanently for that attempt, clears its success milestone and quiet interval. Actual pending ordnance blocks quiet credit; pending-origin producer remains the missile successor. |

## Current core behavior

`request_fire_front_pulse` stages its normal resource debit and an actual scene query before publication. A private prepared impact rechecks owner/hull, system/visit, current pose/tick, fitted standard birth item, current resources, actual contact/end, unique emission/incident counters, legal state and session epoch. The final epoch check also rejects a Continue at the same tick. No public method accepts a contact, damage amount, law result or saved receipt as an action capability.

An actual main-station contact records a stable ordinary receipt including victim entity 1, actual fitted item, attacker system/visit, emission/incident sequence and physical point/time. Main-station geometry remains intact. Its attack window uses an airborne physical clock: pause and docked steps cannot age the window. Export legality and 64-point saturated additions share the actual account legal value without changing wallet revisions. Validation accepts their possible interleavings, rather than confusing legal effects with paid transactions.

A current reserved beacon contact stays harmless. It records its original first-hit evidence and normal weapon resources; it creates no ordinary impact, damage or legal penalty. An ordinary station hit after consent retains its actual first failure receipt. Practice snapshots then report `Failed`, the provider stops presenting live practice targets and reception reports zero quiet credit, no readiness and no trace. Later flight, docking and Continue cannot repair the attempt. An already completed reception remains terminal.

`combat_snapshot` exposes actual shield/energy pools and the last ordinary receipt. The private player-contact reducer implements the two source shield sides and residual shared energy for Pulse and standard missile damage. Its direct numerical tests are internal reducer qualification, not evidence that a missile was launched. The current public physical producer is Pulse against the main station; no public command can manufacture a player hit.

## Persistence and successor boundary

Version 7 retains ordinary counters, physical clock, attack windows, legal-penalty chronology, last ordinary receipt and the first T02 ordinary-hit receipt. The source pose is checked against native station geometry; first/last receipt sequence, current identity and same-tick pose join are validated before replacement. Removing a required failure receipt refuses. Version 4, 5 and 6 readers retain their exact old headers/fingerprints and add no past ordinary events. Version 6 hot practice keeps its existing quiet anchor, hits, weapon state and paid property. Version 5 still earns a newly observed quiet interval.

A damaged player pool without the successor's actual ordnance producer is explicitly an unsupported save boundary. `save_bytes` refuses before the host can replace a valid save. It retains an unsupported-impact marker even after recharge or docking refills the pools. It cannot silently repair the resource or fabricate an ammo/ordnance record. The private reducer is not a load factory. Live missile stock consumption, lock selection, projectile identity, muzzle-clearance/swept-player contact, guidance, pending-projectile ceasefire and saving actual live ordnance remain a coherent successor. This predecessor claims none of those outcomes.

## Author qualification and integration

Seven new tests cover actual public station-hit failure/Continue, inclusive law windows and pause, duplicate/foreign/stale/restored plan refusal, harmless actual beacons, side-specific damage/recharge/fatal/refusal arithmetic, corrupt failure/contact/law refusal and exact v6 compatibility. Existing resealed T02 corruption tests now locate the appended combat block while retaining the original five-byte Pulse tail. Tests are authored and unrun.

The joined desktop handles `PracticeStatus::Failed` in `crates/meridian-desktop/src/session.rs` with a plain failed-attempt message and no success reception action. Accepted station strikes show a legal-incident notice. `FireError::SequenceExhausted` already provides its own Display text. The returned `PracticeReceptionSnapshot` adds immutable `ordinary_hit`, and `combat_snapshot` supplies condition and incident presentation. Existing installed previews and profiles stay separate until the complete join is reviewed and qualified.

The executor should first run `cargo test -p meridian-core` and `cargo clippy -p meridian-core --all-targets -- -D warnings` on the exact joined commit. On the reviewed desktop composition, run the normal full `make ci` and the actual station-hit/failed-reception cold Continue path. No authored test is a passing result and no source-only handoff is playable acceptance.
