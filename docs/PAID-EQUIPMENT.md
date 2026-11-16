# Paid SH025 cargo expansion

Historical feature contract and author-stage notes. For current delivered behavior, exact validation and open limits, read [implementation status](IMPLEMENTATION-STATUS.md). Earlier UNRUN statements below apply to the original handoff, not the latest delivered source.

Status: one coherent M2 core slice is authored on exact main
`edaa29f8daefe849e55efcf69e7ed33d804aab77`. Formatting and source checks pass;
compilation, Rust test execution, strict Clippy, whole-slice review and native
acceptance are **UNRUN** for this component. Root owns the joined desktop;
MC1 owns immutable content/extractor/provenance. No Mac compiler/native job,
installation or probe ran here. The separate retained memory failure is
unchanged and remains unresolved.

## Exact source rule

Immutable original L337 source is
`a11fe16924d87d06021816f9037952afcbac08db`, tree
`68fec574df12ddf74acb14747e0dd6839f478642`. This slice implements exactly one
item: original Large Cargo Bay, family `utility-cargoExpansion`, on the real
SH025 `utility.cargoExpansion` slot.

- `Game.swift:2541,2583`: price 28000 credit tenths (2800 CR);
  technology field three means required technology level **four**.
- `OriginalFleetR3Data.swift:1036` and `ShipPerformance.swift:69`:
  SH025 has 92 base hold tonnes plus 35 only when the expansion is fitted.
  There is no imported geometry or fleet acceptance prerequisite added here.
- `EquipmentRuntimeCatalog.swift:12,137` and
  `EquipmentTransactions.swift:176,285`: this utility uses a genuine declared
  cargo-expansion slot, automatically fits on purchase, remains bound to its
  actual hull and cannot be purchased twice merely by storing the first item.
- `OwnedEquipmentInventory.swift:298,344` and
  `EquipmentMutations.swift:150`: fit/store retain physical item identity and
  acquisition provenance. A proposed fit is committed only if the complete
  commander remains valid, including checked cargo capacity.
- `NativeStationServices.swift:174` and `Plugh.swift:62`: ordinary equipment
  purchases require actual main-station services and sufficient technology;
  the PLUGH discount does not apply at either supported station. Avenor
  G01-S007 has TL12; Renas G01-S185 has TL13. Both qualify at the authored price.

MC1's source proof also covers the actual fitting budget, beyond a descriptor:
the current eight starter items are four standard intrinsic shield/energy/
navigation/drive foundations, one front Pulse Laser and three missiles.
The foundations and those weapon/ammunition nodes consume zero equipment units
and zero utility mounts under `EquipmentIssueTransactions.swift:257` and
`ScriptEquipmentProjection.swift:4`. The cargo bay's ordinary default module
cost is one equipment unit and its fitted utility location uses one utility
mount (`OriginalFleetR3Fitting.swift:74`). Thus this slice uses 1/24 equipment
units and 1/3 utility mounts when fitted, 0/24 and 0/3 when stored. The paid-state validator requires that unchanged supported eight-item fitted
basis; this slice offers no starter-item unfit operation. No unowned device,
extra starter kit, cabin or occupied slot is inferred from the ceilings.

The loader consumes MC1's exact 107-byte `sh025-cargo-bay.equipment` MEQP0001
record, SHA256
`2e99546d65fec686e65275599c0ac529a17551655678fd65e4b794e74104b83d`.
It checks every supported numeric field, the five original identity strings,
actual two station technologies, truncation and trailing bytes. Descriptive
content never creates an item, ownership, slot occupation or debit authority.

## World transaction and public interface

`World::request_equipment(command, expected_tick)` uses the same actual berth,
owner/hull, current tick, pause and station admission as ordinary services.
Commands are `PurchaseCargoBay`, `FitCargoBay(ItemId)` and
`UnfitCargoBay(ItemId)`. ItemId is opaque, owner-scoped and allocated only by
the committed purchase. There is no public item factory, fitted-state setter,
price override, inventory grant, refund, sell or migration endpoint.

Purchase prepares a private candidate, checks availability and funds, debits
the sole actual account, creates its one paid receipt/item, fits its real slot,
and validates the full proposed account before one publication. Fit/store
prepare the same candidate and advance account/equipment revisions without
changing money. Stored paid property remains the same hull-bound ID; a fit
requires that actual item. A refused action changes neither wallet, ownership,
fit, stock, fuel nor identified mission freight.

The immutable equipment snapshot exposes exact source definition/offer, paid
item identity/hull/site/price, fit state, current used/capacity tonnes and actual
equipment-unit/utility-mount usage. Editing it grants no command authority.
ServiceSnapshot also exposes actual hold capacity so desktop views need no
hardcoded stock capacity.

Market purchase, native-tonne accounting, G02 acceptance and save validation now
use the account's actual 92/127-tonne capacity. Unfit refuses above 92 tonnes,
including genuine mission freight and reserved ordinary cargo. The source
kg/g rule remains zero tonne occupancy at either capacity. Equipment actions
change the same account binding, so old freight reviews become stale.

## Save boundary and rejection

The current envelope is explicitly version **3**, rules **3**, with the existing
MERSAVE1 family magic and an eighth compiled resource stream for equipment.
Version-one and version-two saves refuse before decoding into a candidate.
There is no automatic migration, fallback, reset, restock or new-profile repair.

The appended account payload retains equipment revision; the one paid record's
item, owner, bound hull, transaction, purchase account revision, real site and
original debit; and current fitted state. Load checks identity, actual price/
eligible site, single purchase, revision chronology and fit-toggle parity.
Equipment and utility usage are derived from the actual supported fitted item
against the pinned capacities. Unpaid/descriptive source fields cannot
manufacture fitted property.

World accounting includes equipment purchase debit once and equipment command
revisions alongside freight/service/jump revisions. All original market,
wallet, fuel, native-cargo and G02 custody checks remain. Current cargo capacity
comes from current fit. Legal-history bounds retain the maximum capacity
supported by this still-owned paid item even after storage; storing hardware
does not erase an already accumulated export record. This is semantic
validation and corruption detection, not authentication of arbitrary rewritten
complete histories.

## Independent expected outcomes and authored tests

The public integration tests earn all purchase funds from original finite
markets and actual flight/jump/physical docking, never a balance setter.
Independent expected cash after the prescribed sales is 11434, 18296, 29042,
33650, 49541, 57509 and **85349** tenths. Two paid 102-unit refills cost 204
tenths each. Purchase at Renas leaves **57349**, fits the actual same-owned hull
and increases capacity from 92 to 127 with 1/24 units and 1/3 mounts.

Six new Rust tests cover:

- Source content identity, original numeric budgets/technology and malformed
  content rejection.
- Internal paid identity, bound hull, retained stored ownership, reversible
  fit and no extra purchase debit.
- Internal foreign-item/refit, 93-tonne reduction refusal and checked revision
  exhaustion without mutation; these arithmetic fixtures grant no public
  ownership or navigation authority.
- Public earned purchase, complete 127-tonne hold, another tonne refusal,
  native Gold at full capacity, blocked unfit, exactly 92-tonne unfit success,
  retained paid ID, no-refund/no-charge refit and fitted/stored Continue.
  Resealed receipt owner/ID/hull/payment/site/revision/fit edits and versions
  one/two refuse with the live world's full saved bytes unchanged.
- Public genuine eight-unit G02 custody plus 85 ordinary tonnes blocks
  reduction at 93; selling one ordinary tonne allows storage at 92 while all
  eight original mission units remain. Fit/store invalidates a prior review.
- Public insufficient funds, stale tick, pause, airborne/unsupported berth and
  prototype refusal retain full bytes and cannot create equipment.

All original test assertions remain. These six tests are **authored, UNRUN**.
Root must join the exact core, MC1 content and desktop source for independent
whole-slice review, one coordinated Dell execution and exact-source quality
checks. Native app and durable process restart acceptance remain separate
evidence; a formatter or Python content/arithmetic check is not a gameplay pass.
