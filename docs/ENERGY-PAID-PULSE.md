# Stock energy modules and paid Pulse replacement

## Source stage and actual coverage

This increment adds paid Pulse Laser replacement and the stock energy-family effects to the modular inventory. Eleven new Rust cases are authored and **UNRUN**. No compiler, Cargo/tests, Clippy, GPU, app or native process ran in the G3 author lane. Parent execution results do not qualify this source. The actual Root/MC1 mining, screen, profile and hull joins remain separately required.

The [sixteen frozen source files](ENERGY-PAID-PULSE-SOURCES.json) bind original revision `a11fe16924d87d06021816f9037952afcbac08db`. The implementation retains source prices, one-based technology levels, owned-node limits and energy prerequisites. It introduces no grants, arbitrary recharge input, free reward laser, second wallet or shop stock counter.

## Real definitions and effects

| Node | Price in CR | Required TL | Recharge multiplier | Equipment units |
| --- | ---: | ---: | ---: | ---: |
| Basic Energy System | Born foundation | Not for sale | 1 | 0 |
| Extra Energy Unit | 7,200 | 9 | 2 | 2 |
| Naval Energy Unit | 19,600 | 17 | 3 | 3 |
| Elite Energy Unit | 13,500 | 14 | 4 | 4 |

All stock energy grades retain capacity1x. The original energy graph requires the preceding owned grade, including a stored predecessor. Each node is single-owned. Buying a higher grade stores the previous active energy item without changing its identity, provenance or price history. Storing an energy unit restores the original native1x recharge fallback; it does not refund money or create another Basic unit. Refitting uses the exact current occupant and item.

For the existing SH025, native recharge4.2/s is7 hundredths per60Hz tick. Stock grades therefore use7/14/21/28. Actor advance, current resource snapshots, saved-resource validation and ordinary-hit resource checks consume the same actual fitted rate. The 380-unit pool, shot cost1, heat10 and18-tick Pulse/mining cadence stay unchanged. Fitting does not modify the pool or refill a deficit. The independent ship-resource snapshot now includes actual capacity and recharge per tick. Root must bind the real selected-hull base rate when composing G4's all-hull runtime; this isolated core still uses the existing SH025 base.

Current Avenor/Renas equipment sites have TL12/TL13. They can sell Extra Energy Unit, but Naval and Elite retain meaningful technology refusals. Their stock definitions, slot effects and prerequisite policy are implemented; no high-tech acquisition in those two berths is claimed. The pure higher-grade ownership test is explicitly a definition/policy fixture, not a funded World purchase or native pass. A real eligible berth/recovery path remains required before higher-grade gameplay acceptance.

Paid Pulse costs1,450 CR at TL3. The original maximum is four owned Pulse items, including the born one and stored spares. A purchase creates its own paid ID, stores the previous front occupant and uses the actual existing wallet. The actual front fire command consumes its normal energy/heat/cooldown and emits from that paid item's identity. Its range, geometry, damage15, ordinary-incident/legal rules and interception math remain unchanged.

Historical station hits and missile interception beams now resolve the actual Pulse item fitted at the shot's tick and visit. A stored born Pulse cannot be substituted for a paid firing item. Later refitting of the born item does not invalidate the retained paid shot. No decoder applies a shot, damage, recharge or fitting action.

## Inventory, fitting and budget

The module snapshot now exposes the **already owned** born Basic Energy System from the account. This adds a readout of existing property, not a second birth grant. The account's original energy and Pulse items remain present, with fitted/stored mirrors validated against the module ledger. Only recorded changes to those supported slots permit a stored birth mirror; other foundations retain their existing gates.

`ModuleAction::Purchase { kind }` uses the authoritative stock definition. The existing mining purchase command still routes to the same canonical node and limit. `module_offers()` provides the five actual supported stock offers: Pulse, Mining and the three energy grades. Slot, equipment-unit cost, prerequisite and availability are descriptive outputs. UI fields never authorize an action. Installing an energy item changes only the energy slot; installing a laser changes only the front laser slot.

`ModularEquipmentSnapshot` reports fitted equipment units together with the actual cargo bay, and its current capacity. Lasers use their source laser position and zero utility/equipment units. Energy uses its dedicated slot and2/3/4 equipment units. Stored ownership, finite ammo and the source256 ordinary-item bound are reconciled together; the one reserved stock drive remains exempt. Existing3-active/128-record service caps and65,536-byte save cap remain.

Every accepted purchase/store/install advances the real account and modular fit revision once. Prepared candidates validate the complete prospective account, inventory, budget, accounting and bytes. Atomic writer failure drops wallet and displaced-item changes together. Material reviews become stale after a real fit change, even when cash stays unchanged.

## Current format and actual join

This isolated source writes explicit draft v14 for generic stock-purchase tags and the expanded module definition boundary. It does not reinterpret previous unqualified v12/v13 drafts. Existing explicit v4-v11 boundaries remain unchanged; the greenfield brief requires no additional compatibility work. Unsupported saves refuse without replacing the live World or touching existing preview/profile files.

Root must choose the actual final composed version/content boundary containing its real MC1 mining state and module/ammunition state. Equal local version numbers do not establish byte compatibility. Root owns screen/profile/HUD and all-hull serial composition. The previous concrete mining join remains in `docs/integration/PAID-MINING-ROOT-JOIN.rs`; the new source retains its actual paid fitted getter, shared resource preparation and opaque authority contract.

## Meaningful regressions and remaining coverage

Eleven new cases cover real7,200 CR energy purchase, two budget units, unchanged capacity/pool, native4.2 vs8.4 recharge through actual public shots/steps, cold deficits, independent energy/front slots, genuine extra finite-market funding to86,605 tenths, paid Pulse fire and station impacts, same born/paid item reuse, writer failure, cash/technology/standard-foundation refusal, all stock energy definitions/prerequisites, no refund, coherent save tampering, real four-owned Pulse limit and actual paid Pulse interception. The interception negative substitutes the owned-but-stored born ID and requires refusal; later real refit still preserves the paid receipt. All are **UNRUN** here.

The mining tests now expect the additional readout of the pre-existing born energy item and locate the unchanged paid Mining item at its new position. They keep all prior price, cash, identity, resource, writer, refusal, slot and custody assertions, with added standard-energy identity checks. Current writer-version assertions move13→14. No old physics, range, damage, heat, cooldown, service cap or source price is weakened.

G2 must review the whole final joined source. Existing owners then execute real repository/tests/strict-Clippy/release checks on the exact joined commit and tree. Native proof must use earned cash, buy/refit real items, show actual recharge and paid firing identity, preserve historical receipts, and cold-continue without grants/refills or profile loss. Source formatting and unit criteria do not establish native acceptance.

The full modular requirement remains ongoing: other laser profiles/facings, shields, navigation/drives, targeting, EMP versus ECM, collection/salvage/rescue/passenger hardware, services and actual all-hull effects are not closed by this increment. Military Laser retains TL14; Naval Laser needs its real discovered secret-port/reward acquisition rather than an ordinary zero-price offer. Broader core work stays ahead of new mission expansion.
