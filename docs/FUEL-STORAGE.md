# Stock fuel storage

The source candidate adds four individually owned repeatable Fuel Storage Units. Ordinary SH025 has three shared utility mounts, so it can own four units but cannot fit four at once by ordinary purchase or refit. The separately labelled XYZZY TEST MAX can exercise the original four-tank maximum exception. Native acceptance remains pending.

## Ordinary acquisition and operation

| Property | Original stock value |
| --- | --- |
| Price / technology | 2,000 CR / 1 |
| Global ownership cap | 4 distinct physical items |
| Capacity per fitted unit | 70 fuel units / 7 LY before navigation reach limit |
| Per fitted unit | 1 equipment unit and 1 shared utility mount |
| SH025 normal capacity | 130 stock; up to 340 with 3 tanks and no competing utilities |

Pay for each physical tank once at the Equipment screen, Fuel storage slot. A purchase fits the next free position and leaves the current fuel unchanged. Refuel through the existing Market and pay 0.2 CR per missing fuel unit. Store an identified tank only when its removal leaves enough capacity for the existing fuel; fit a held tank without another purchase or free fuel. Fuel Scoops, EMP and cargo expansion compete for the same three SH025 utility mounts. The display reports actual tank capacity and current fuel; the navigation computer still limits jump reach.

All four physical identities, payment or grant receipts and exact fitting events survive Continue. A stock-only account or route retains the previous fuel field bytes. Extended fields encode fuel and the fitted count in the existing two-byte position. The account mirror and historical fuel supply, purchases, refills, tank removals and real jumps must reconcile with ownership. The route planner uses the actual current and full-tank limits; fuel shown in the cockpit is a fraction of the real tank, not the stock denominator. Current playable stations do not provide a leg longer than the stock 13 LY tank. Other original playable stations and ordinary four-mount hull ownership remain separate work.

## Explicit four-tank test fit

Open Ctrl+Shift+X and select **TEST MAX: fit all four fuel tanks**. Enter reviews and Enter confirms. This is an explicit test-only exception, not an ordinary fitting grant. It preserves existing paid items and uses stored compatible property before granting missing tanks. The new record installs all four and then refills the actual 410-unit tank; repeat activation neither duplicates items nor resets cash a second time. The original limited supported outfit and shield selector remain available without taking this extra action. Limited activation, refit or restore preserves already-fitted tanks and other fitted utilities, adding optional utilities only while ordinary mounts remain.

The original permits its budget exception only without active passenger or contract custody. Meridian does not yet implement passenger cabins/people or those mission owners. It has actual G02 freight custody and T04 navigation trade attempts: both refuse this action while active, and a test overfit must be reduced to ordinary physical limits before starting either. For new G02 jobs the exact issue and abandonment revisions are persisted, independently of transfer receipts; a legacy abandoned job that lacks an exact closure revision cannot prove the guard and therefore refuses this exceptional fit. New T04 attempts retain exact consent and closure account revisions with their historical fuel anchors; old attempts retain their earlier tick-based evidence and same-tick order remains unexpressible for legacy actions. Later explicit supply is not a reason to rewrite a closed anchor. From the first maximum exception, actual cargo fitting changes are also retained, allowing the shared utility and equipment budget at each known custody issue to be checked even after later storage and closure. No unavailable passenger state or mission receipt is fabricated.

## Immutable source

Revision `a11fe16924d87d06021816f9037952afcbac08db` and tree `68fec574df12ddf74acb14747e0dd6839f478642`.

| Source (under Sources/L337Core unless stated) | SHA-256 |
| --- | --- |
| FuelStorage.swift | `b78beb5454d505217b71770d1c9daf7afb8be13fe81ab4b9d8c7725514c0c2fc` |
| OriginalFleetR3Fitting.swift | `5a50e94cb510d7ce51da194cc3570159a28f1b7bd126e140a948192412d913ee` |
| OwnedEquipment.swift | `c8a9e6eb6d859d917f8621cf9146b518521ea7708a5ad6ac68334efe9f9607cf` |
| EquipmentMutations.swift | `a1d0bece73e7d6370f4cab8fc952d2e25a3d20dfceb89ce8fab189c4b65941d5` |
| PassengerCustody.swift | `9bb1f6239ce2b77c2bd6c7ce11f75a18d639c86ed5e74ad112be659cf6f88db1` |
| Game.swift | `e5b4e30c835d460aa7ca6f77364db0aaeaf877bb9d73bacc162e9111b26e2e17` |
| Tests/L337CoreTests/FuelStorageTests.swift | `fae8d81192ff2cd881e5e6b41fcf6b4f56738e66f0900e87bcc13033801b2d7f` |

Authored regressions cover real earned cash and four paid identities; three-mount refusal; separate refuel, occupied removal, actual traversal, stored refit and cold replay; four explicit test grants or reuse of paid property; repeated max and real custody refusal; stock encoding; actual desktop menu and cold publication. The [exact protected-source inverse](../integration/sh001-mule/FUEL-STORAGE-INVERSE.json) preserves the previous independent Mule source pins.
