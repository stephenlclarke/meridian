# Ordinary commodity trading

## Atomic quantity trading

This following-preview core increment is based on frozen `e470017b1233e05d8e32781bdfd96d2936b90222`. It is composed development source; fifteen new core tests and six new desktop tests are **UNRUN**. The existing E470 preview remains frozen. Root has joined quantity controls and durable host publication; independent review, owning compiler checks and actual bulk-trading screen acceptance remain pending.

The market starts at one native unit. Left/Right adjust by one and Shift+Left/Right by ten; the visible -10/-1/+1/+10 buttons use the same semantic actions. Quantity cannot fall below one or wrap. Changing commodity resets quantity to one. The review captures the requested quantity, native unit, actual quote and exact total before Enter confirms. Sales show gross value and actual credited cash separately. Only the currently presented frame authorizes controls; changing quantity invalidates the old frame. An open transaction review blocks quantity changes. Save failure and complete-batch refusal leave the live World and previous save unchanged.

A player supplies `station_services::CommodityTrade { side, commodity, quantity }` with `TradeSide::Buy` or `Sell`. `World::prepare_commodity_trade(intent, tick)` uses the actual supported current berth, sole CommanderAccount, native commodity row, finite stock and current lesson/freight reservations. It returns `PreparedCommodityTrade` with the exact receipt and complete semantically validated save bytes. `publish_with` commits the live account, market and lesson only after the host writer succeeds. Dropping preparation or writer failure leaves the complete live save unchanged. `request_commodity_trade` is the synchronous core equivalent; a durable host must prepare and publish first.

The receipt reports the native quantity, station, unit price and exact gross total in credit tenths, plus actual paid or credited cash. Zero, unsupported native quantity, price multiplication/counter/stock overflow, insufficient complete funds/stock/hold, protected cargo, wrong berth, paused or stale commands refuse the whole batch. No partial purchase, per-unit save loop, new warehouse or mission reward exists. Quantity arithmetic is constant time; complete-save validation remains the same bounded validation performed for ordinary services. Food uses its existing scalar row; other commodities keep their sole native entries. Only tonne goods use hold capacity in this source. Gold/Platinum kg and Gem-Stones g retain their existing zero hold-tonne rule without guessed SI conversion.

Ordinary sales retain the existing `u32::MAX` cash saturation: gross bid value and actual credited cash are separate. Arithmetic beyond the exact `u64` gross-total range refuses rather than wrapping. Buy totals exceeding available `u32` funds refuse. The host must show the actual credit when saturation reduces the gross proceeds; this is separate from T04's existing strict credit-overflow admission.

T04's eligible identified purchase and destination sale require quantity one and use the same original one-tonne receipt path. A larger batch at either boundary returns `NavigationTradeError::SingleUnitRequired` before removing reservations or making changes. Other ordinary bulk activity retains the current reserved capacity/held tonne and unchanged T04 guards. G02's eight identified units and once-only payment remain independent of ordinary Machinery quantities. Existing single-unit service commands/events are unchanged.

No persistent fields, content bytes, layouts or save version change. Existing account/market revision and buy/sell counters count native units, so one batch advances them by its quantity. That produces the same durable accounting as repeated single-unit actions without repeating preparation or publication. Internal prepared market candidates now carry a typed receipt through their existing complete account/market/berth binding check; the default single-service receipt stays unchanged.

The eight new component tests cover all available rows at both markets against repeated singles, quantity-one event/state equivalence, exact funds/stock/hold limits, kg/g at a full hold, protected native reservations, zero/native/price/counter/stock bounds, ordinary saturation and large constant-time arithmetic, stale complete candidate binding and actual service-fact guards. These arithmetic fixtures are not grants or gameplay acceptance.

Seven public-command tests cover prospective/drop/failed-writer/cold publication and complete save-byte equivalence; actual profitable bulk travel funding and a genuinely paid Mining Laser with real export legality; stale/paused/airborne admission for all seventeen rows; complete-byte refusals; T04 identity/single-trade/reservation boundaries; an actually funded full hold with reserved capacity and native kg/g rules; and ordinary bulk Machinery alongside all eight G02 receiving actions with only the original final reward. The profitable route contains fourteen bulk buy/sell actions for the same 766 native units as the existing single-unit funding helper; equality is authored as an exact-byte assertion, not reported as a measured pass or timing improvement.

The assigned existing executor must run these tests on the exact composed source and cover affected service, freight, equipment and T04 tests, strict Clippy, required CI and coverage. Independent review and actual native quantity selection/price display, refusal, save-failure and cold Continue acceptance remain open. No compiler, test, Clippy, Lua VM, game or new Rust helper ran in this authoring lane.

## Historical single-unit component

Historical feature contract and author-stage notes. For current delivered behavior, exact validation and open limits, read [implementation status](IMPLEMENTATION-STATUS.md). Earlier UNRUN statements below apply to the original handoff, not the latest delivered source.

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
The historical single-unit interface did not accept caller quantities. Caller prices, market facts and ownership constructors remain unavailable.

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
