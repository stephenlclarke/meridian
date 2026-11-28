# Paid standard missiles and finite fitting

## Author-stage status

This core increment is authored on qualified T04 base `1a4d957ddca6cba5ed483a6c5f325805d71645f1`. Rust tests, compiler, strict Clippy, native play and installed delivery are **UNRUN for this change**. Only direct rustfmt parsing/formatting, static source checks and Markdown lint belong to this author lane. Parent test counts do not qualify this source.

The next preview requires HUD, real docked menus and modular equipment. This is the first paid-ammunition increment, not completion of the full modular system. All lasers, navigation/energy variants, EMP, ECM, work/rescue equipment and their actual fitted effects remain subsequent core work. No unsupported module is advertised or sold here. Full fleet compatibility also remains separate work; this increment uses the existing owned SH025 and verified missile geometry.

## Source and intentional design

The [immutable source manifest](PAID-MISSILES-SOURCES.json) binds original revision `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. Canonical node `missile-standard`, family `ammunition`, display name Missile, costs 180 CR (1,800 tenths) and requires one-based technology level 2. EquipmentTransactions and EquipmentPurchaseProgression create one new paid item at the first free compatible pylon. SH025 has nine missile positions. Its three starting rounds are a separate birth grant; the other six positions begin empty.

The original offer has revision 1 and no mutable finite shop quantity. This implementation retains a finite item for each purchase, one actual wallet debit and permanent consumed-item evidence. It does not invent a finite shop stock counter or repeat the birth grant. Avenor and Renas use their existing verified main-station technologies 12 and 13. Secondary producers do not acquire equipment authority. No price discount is introduced.

Original ownership permits storage and refitting of the same item, validates owner/hull/slot compatibility, increments inventory revision once and removes the actual consumed round. The source ordinary-inventory maximum is 256, with one reserved stock drive exempt; the actual cargo bay, when owned, also uses an ordinary item. The paid history source bound is 100,000. The unchanged ordinary save-base cap of 65,536 bytes will refuse additional history sooner. The existing one-byte projectile ordinal also refuses overflow; no old origin is reused or overwritten.

Meridian uses deterministic owner-local item and transaction sequences, explicit typed actions, one World-owned custody ledger and the existing sole CommanderAccount wallet. It does not clone Swift UUID allocation, actor topology, keyboard/menu implementation or profile storage. Each retained birth item remains in the existing birth ledger until consumed; authoritative ammunition location comes from the World ammunition ledger. Storage changes location without manufacturing another item or money.

## Core boundary

- `World::ammunition_snapshot()` exposes immutable items, paid receipts, canonical node/name, current offer availability and fit revision. Consumed records remain visible as evidence and cannot be stored or refitted.
- `review_ammunition(action, tick)` captures actual current main berth, commander, epoch, account revision and fit revision. Purchase chooses the first free slot. Store/refit operates on the exact item. Incompatible/out-of-range or occupied slots, missing/spent items, no change, stale/foreign reviews, pause, flight, insufficient cash and exhausted identities refuse.
- `prepare_ammunition(review, tick)` validates the complete prospective account, custody, mission/accounting graph and bounded save bytes while exclusively borrowing World. The host publishes `bytes()` atomically, then calls `commit()`. `publish_with(writer)` makes that order explicit and drops the candidate if the writer fails. No core filesystem, GPU, window or wall clock is introduced.
- `World::missile_rounds()` contains all nine physical positions. `World::missile_projectiles()` yields every retained projectile, including origins beyond the first three. The old `MissileSnapshot.rounds/projectiles` fields remain three-entry prefixes; `additional_rounds` contains the other six slots. The App/HUD/mesh join must use the complete APIs.

Discharge selects the actual highest fitted pylon and consumes that exact finite item. It retains every origin, emission, motion, collision, beam/interception and impact receipt. Old mesh/range/speed/guidance/shield/incident predicates remain. A refused discharge restores account, launcher, combat and practice state. Cold continuation validates custody by replaying actual acquisitions/fitting against retained launches; it does not emit a shot, apply damage or refill resources.

Every purchase/store/refit/consumption advances the ammunition fit revision. Existing T01/T02/T04 material checks include actual ammunition/account state. Fitting uses no mission service slot, changes no mission source or reward, and preserves existing shared 3-active/128-record bounds. Historical born ownership remains distinct from current location and consumed custody.

## Current save and continuation

The paid missile first author increment used explicit v12; the following modular-tool source uses draft v13. Root must compose one explicit final equipment/mining format before execution. It adds the account ammunition fit revision, acquisition/fitting events, variable retained projectile records and nine-body T01 sight samples. The v11 fuel-chain-v1 content boundary is retained verbatim; the pre-existing v4-v11 readers remain available. No published preview or existing profile is modified, and no old bytes are relabelled as a new format. Greenfield product work does not require further compatibility expansion. Unsupported/corrupt files refuse without replacing the current World or last valid file.

## Acceptance for the execution owners

The authored cases cover the 180 CR debit and first free slot, three born rounds/nine capacity, complete paid candidate/drop or writer failure, stale/foreign/paused/flight/continued reviews, same-ID storage/refit, no funds, full nine positions using an earned wallet, moved born-item highest-pylon consumption, five retained discharges, paid replacement after all three starters are spent, no spent-item resurrection, current-format cold continuation, checksum-coherent custody tampering, fit-review invalidation, three actual occupied lesson slots and the unchanged save byte bound. One explicit v11 reader regression accompanies the format change. All these cases remain **UNRUN** here.

Root must serially join actual docked purchase/store/refit input and the atomic profile writer, complete-round HUD and all-projectile renderer iteration. G2 reviews the entire joined delta. Existing Mac/Linux/native Windows execution owners then run their real repository checks and bind results to that exact joined commit. Native proof must buy a round at an actual berth, verify paid ownership/location/cash, store/refit it, launch/fire it, dock, quit and cold Continue without changing old origins or resurrecting a spent item. Writer failure must preserve live state and the last valid file. Source tests alone do not establish any of these native outcomes.

## Desktop integration candidate

The desktop Profile now exposes typed purchase, store and refit actions through the actual Store publication path. Failed publication leaves World unchanged; a written save commits the candidate even when directory synchronization reports an error. New tests cover paid purchase, same-item store/refit, cold continuation and failed publication of each action. The HUD and renderer consume all nine rounds and all retained origins. No docked button or shortcut is claimed before the menu join, and all joined Rust tests remain unrun. This is one equipment increment; full modular equipment and mining tools are still being implemented.
