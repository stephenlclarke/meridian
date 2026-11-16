# Modular equipment: first mining-tool increment

## Actual coverage and validation

This source adds genuine paid front Mining Laser ownership and same-item removal/installation, including storage and refitting of the displaced born Pulse Laser. It is the second equipment increment after paid finite missiles, and is **not the complete modular equipment system**. Thirteen standalone Rust cases and two Root-join integration cases are authored and **UNRUN**. No compiler, Cargo, tests, Clippy, GPU or native process ran in the G3 author lane. Source/static formatting and provenance checks are reported separately from execution.

The next preview still needs its actual docked equipment screen, complete HUD and fleet joins. All other lasers/facings, shield/navigation/energy grades, drives, targeting modules, EMP, ECM, scoop/salvage/rescue/passenger equipment and their real effects remain subsequent increments. A menu definition is not an implemented effect. This source sells no placeholder module, free Naval Laser, fabricated mining utility or Pulse alias.

## Verified definitions and compatibility

The [frozen source manifest](MODULAR-EQUIPMENT-SOURCES.json) binds fourteen complete files from immutable original revision `a11fe16924d87d06021816f9037952afcbac08db`. Game.swift defines Mining Laser at 33,000 tenths (3,300 CR), zero-based technology 5, therefore one-based TL6. LaserSpecification defines damage50 and interval0.3s. MiningWorkTool and GameplayWeapons use the same energy1/heat10/cooldown pool. Selected mining work additionally requires heat at most230 and an actual available rock/ray/cargo before any effect.

SH025 declares all four combat laser facings, but its workTools list contains only salvageCutter. The supported mining destination in this increment is the **front laser slot**. Do not add a separate mining utility to that hull. Original MiningWorkToolTests verifies that a front Mining Laser works without the dedicated utility. Rear/left/right laser effects are still pending and cannot be installed or sold through this front-only action.

The canonical family is `laser`, with nodes named Pulse Laser and Mining Laser. The original Mining Laser node allows at most five owned examples, including stored items. Each paid purchase creates a fresh deterministic owner-local item and transaction, debits the actual existing wallet and stores the previous front occupant without deleting or refunding it. The born Pulse retains its original ID and standard provenance. The account's birth-fitting mirror and World module ledger must agree about whether that Pulse is fitted or stored. Installation requires the exact current occupant ID, so a stale slot cannot be silently replaced.

Laser gear costs zero utility/equipment-budget units in the original; it consumes its actual laser position. Stored gear still counts as owned ordinary inventory. The source 256 ordinary-item bound, reserved stock-drive exception, actual cargo bay and finite ammo are checked together. Existing 3-active/128-record service limits and 65,536-byte complete-save bound remain. No new stock quantity, wallet, cargo owner, hull or asset permission is introduced.

## Actual core API

- `World::modular_equipment_snapshot()` returns the real born/paid laser items, locations, provenance and current mining offer. `review_module` and `prepare_module` implement paid purchase, Store and Install with stale/profile/epoch/account/berth checks. `PreparedModule::bytes`, `item`, `commit` and `publish_with(writer)` retain complete candidate publication before live mutation. Writer failure drops the wallet debit and displaced-item change together.
- `World::fitted_mining_tool()` is a descriptive readout of the actual paid compatible front fit. It includes owner, hull, current account revision, live freight/profile epoch, item, source damage/range/energy/heat/cadence. It is not accepted as public work authority. `paid_fitted_mining_item` is the crate-only pure getter used by the real mining authority producer.
- `World::prepare_mining_resources` is crate-only. It rechecks actual paid front fit, item, tick, pause and flight, then prepares the existing shared resource pool with the stricter mining heat guard. It confers no ore, target, cargo or cash. Public `mining_resources_ready` is only an advisory query. Root's actual MC1 scene performs the physical ray/source/cargo checks and commits that resource candidate with depletion/yield.
- `World::flight_resources_snapshot()` reads the ship pool independently of a particular fitted laser. `combat_snapshot()` uses that pool. Storing or replacing Pulse disables Pulse fire but cannot hide the ship's energy or mark the ship dead. Refitting the same born Pulse restores its real public fire command and identity. Fitting never resets energy, heat, shields or cooldown.

Existing equipment fit/material review fingerprints include the module ledger and account revision. No service slot or mission reward is granted by a purchase or fitting action. No physical mining shot is simulated in this standalone equipment branch.

## Root's concrete mining join

[Root composition source](integration/PAID-MINING-ROOT-JOIN.rs) supplies the actual authority producer and two meaningful joined test cases, ready for the existing MC1 mining module. It is intentionally outside the compiled crate until Root serially joins both owners' actual source. These cases are **UNRUN and are not counted as compiled entries** in this branch.

The producer mints MC1's opaque MiningToolAuthority only from the real paid fitted getter. It binds owner/hull/account revision/item plus live epoch and the per-World Arc/Weak runtime scope. Request admission must retain MC1's getter re-query. Restore, independent Continue of identical bytes, account changes and unfit items cannot reuse an older capability. No scope or authority is decoded from a player file or supplied by UI.

Replace the actual request_mining direct Pulse preparation with `self.prepare_mining_resources(tool.item_id, tick)`, retaining its fallible preparation before any account/resource/depletion assignments. Keep MC1's owned physical rock/ray/occlusion, speed, native cargo plus T04 reservation and depletion checks. Its greenfield extraction rule is one finite native unit per admitted work action; the original laser damage value does not claim a port of original asteroid hit-point/debris rules.

The joined cases earn cash through ordinary market journeys, buy and fit the genuine tool, approach the actual belt through public flight, refuse old/foreign/continued/restored/paused/unfitted authority without yield, extract one real unit, compare resource and depletion deltas, and cold-continue exact ownership/cargo. They never grant a tool or manufacture a fake positive authority. Native player-controlled work remains separately required.

## Explicit current format

This isolated equipment source writes draft v13: module revision in the account, module acquisition/fitting events before ammo, and the existing paid-missile/T04/T01 state. The ff62 v12 author draft has not been delivered and is explicitly unsupported by this draft; it is not relabelled or silently reset. Existing explicit v4-v11 readers remain. No further compatibility expansion is required by the greenfield product brief.

MC1 authored an independent mining v12 draft with a different layout. **Equal version numbers do not establish compatibility.** Root must choose one explicit final composed version and content boundary containing actual modules, ammo, mining state and all live runtime-scope initialization. This document does not claim that Root's final format is this isolated draft's format. Existing published previews and player files remain untouched.

## Required execution and remaining work

G2 must review the complete joined delta. Existing owners then run real repository/compiler/test/strict-Clippy/release checks on the exact final joined commit. Parent results do not qualify this source. The thirteen standalone cases cover price/TL/provenance, same born ID, failed writer, stale/foreign/pause/Continue/flight, removal/replacement, genuine resource preparation, five-owned source policy, coherent save tampering, actual cargo-bay compatibility, three occupied lesson slots, real Pulse fire after refit and complete-save history refusal.

Root must complete real input and profile publication plus MC1 authority/resource integration, add the two real joined cases, bind all-hull native capabilities, and show independent resource values in the HUD. Native acceptance must buy a tool at a real main berth using earned money, store/refit actual items, select/approach the real belt, mine finite cargo with resource debit, sell it without a second wallet, dock, quit and cold Continue. Do not describe these authored criteria as completed gameplay. The full remaining modular catalogue and fitted effects stay on the critical path before further mission expansion.
