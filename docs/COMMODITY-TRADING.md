# Ordinary commodity trading

This records the historical version-two commodity component. The current M2
[paid-equipment successor](PAID-EQUIPMENT.md) adds actual 92/127-tonne capacity
and explicitly changes the save boundary to version three.

Status: core source and ten additional Rust tests are authored against main
`54b6920f8db7d3935c4e3ffe1ed9d1d921d12e12`. Rustfmt and whitespace checks pass.
Compilation, Rust tests, Clippy, Sonar and actual app acceptance are **UNRUN**
for this component. Root joins MC1's immutable content and the desktop component
before one coordinated G2 review and Dell execution. This work does not replace
the installed preview.

## Content and units

The source is immutable L337 revision
`a11fe16924d87d06021816f9037952afcbac08db`, tree
`68fec574df12ddf74acb14747e0dd6839f478642`. MC1 owns the extractor, data,
original-authoring proof and provenance. The core consumes its exact
`commodities.catalogue` (MCOM0001, seventeen definitions and original display
order) and `g01-commodities.markets` (MMKT0002, two actual stations and seventeen
quotes each). Existing Food data is unchanged and its row must agree with the
new source. These definitions grant no cargo or private authority.

Native IDs follow `GameplayContracts.swift`: Food, Textiles, Radioactives,
Slaves, Liquor, Luxuries, Narcotics, Computers, Machinery, Alloys, Weapons, Furs,
Minerals, Gold, Platinum, Gem-Stones and Alien Items. Names and the display order
come from original authoring, rather than the typed enum's spelling.

`CommanderCargoValidation.swift`, `TypedCargoPayload.swift` and
`Game.swift:5836-5877` count only unit `t` against hold capacity. Gold and
Platinum use kg; Gem-Stones use g. Each command transfers one native unit.
Kilograms and grams consume **zero hold tonnes**, with no guessed SI conversion.
Native quantities and stock remain bounded by 64-bit Swift Int.max/17.
Alien Items has definition target four, but both actual station quotes have
zero stock and zero target stock. A definition never restocks a live market.

## Transactions and custody

World retains one actual account and two finite market stocks. Food keeps its
existing scalar account fields and original commands/events. Sixteen additional
native quantity/reservation entries are the sole ordinary-cargo authority for
the remaining commodities; there is no duplicate Food ledger. G02's eight
identified Machinery GoodsIds remain separate protected mission property.

Typed BuyCommodity/SellCommodity requests use actual station admission, account,
prices, cash, reservations, hold capacity and stock. A checked local candidate
contains both prospective account and market changes; publication happens only
after full current binding checks. Overflow, stale/paused admission, insufficient
cash, no stock, full tonne hold or no unreserved units leave both unchanged.
Sales retain the source UInt32 cash saturation and record the actual credit.
The core does not accept caller prices, quantities, market facts or ownership
constructors.

The original sellable-cargo calculation also subtracts marked Firearms and native
captives. This slice has no producers for those systems. It preserves actual
ordinary reservations and protects the existing identified mission freight; it
does not invent captives, marked firearms, NPC enforcement or extra mission rules.

At a successful actual launch, after every fallible launch check, the account
applies `legal |= cargo[3] + 2*cargo[6] + cargo[10]` from `Game.swift:6096`.
Market transactions neither ban those goods nor levy an immediate fine. Selling
after departure does not erase the accumulated record. Only this export rule is
implemented; combat, police encounters and unrelated legal resets remain outside
this slice. The supported 92-tonne hull bounds each export weight by 184; the OR
history is stored as an eight-bit value.

## Continue boundary and evidence

This component explicitly changes saves to version **2**, rules **2**. The
MERSAVE1 family magic remains, with an explicit version field. Version-one saves
are refused atomically; there is no migration, reset, refund or restock. The
compiled-content fingerprint now covers seven actual resource streams, including
both commodity files. New account entries and legal record follow the old owned
fit prefix; native stock and counters follow the old Food market prefix.

Load validates every commodity's native quantity against purchases minus sales,
both finite station stocks against initial stock and actual buys/sells, aggregate
cash/fuel payments, original freight receipts and tonne-only hold usage. Legal
record bits must be reachable from purchased illegal goods and the actual hold,
and a nonzero record requires a genuine launch. This is conservative semantic
validation, not reconstruction of every historical export or protection against
a party rewriting an entire save and its checksum.

Ten new tests cover all 34 exact source quotes and native-unit buy/sell
accounting; kg/g at a full tonne hold; protected reservations and transaction
rollback; stale complete binding and cash saturation; ordinary Machinery beside
eight actual mission IDs through public flight/receiving; real-launch legality
and failed launch/sale/Continue behaviour; all seventeen commodities through
stale/paused/airborne admission; saved native quantities, stock and tamper/v1
refusal; and all sixteen available commodity trades/Continue after genuine
freight income; and rejection of impossible saved export bits. Existing Food tests remain intact.

All new Rust test execution remains **UNRUN** until the complete joined source
is exercised by the coordinated execution owner. A source packet records the
exact component commit/tree and changed-file hashes; a leaf-format pass is not
an app or CI pass.
