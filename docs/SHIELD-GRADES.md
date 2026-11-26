# Stock shield modules

Development source candidate. The seven source-defined SH025 grades have an independent, finite pair of shield banks. Blocked acquisition paths remain separate from explicit disposable-preview test access. No current release or native acceptance is claimed.

## Original catalogue and acquisition

All capacities scale the current SH025 stock 300-unit bank. Source stock component templates retain front/rear recharge at 1x; the current Meridian fixed-tick recharge remains three hundredths per tick for every grade. The older Mining/Beam booster aliases retain different historic strengths in L337 migration; those are not the stock module strengths below.

| Display grade | Original purchase identity | Capacity per bank | Normal source acquisition | Equipment units |
| --- | --- | --- | --- | --- |
| Basic | Basic Shield | 300 | Standard birth | 0 |
| Basic II | Pulse Shield Booster | 450 | 1,087.5 CR, TL3; Basic owned | 2 |
| Intermediate | Mining Shield Booster | 540 | 2,475 CR, TL6; Basic II owned | 2 |
| Intermediate II | Beam Shield Booster | 600 | 3,525 CR, TL7; Intermediate owned | 2 |
| Advanced | Military Shield Booster | 690 | 12,300 CR, TL14; Intermediate II owned | 2 |
| Advanced II | Capital Ship Shield Booster | 2,070 | ELITE-rank reward or PLUGH-only paid stock | 2 |
| Elite | Elite Shield Module | 2,760 | Acquisition-only by default | 1 |

The original PLUGH commodity/equipment operation supplies a special 13,500 CR base Capital offer, discounted once to 10,125 CR. That transaction and the native ELITE rank reward both remain implementation dependencies, not generic ordinary shop offers or free reward simulations. They need actual PLUGH discovery/berth/funds/identity or authoritative Elite progression with once-only receipt respectively. Ordinary Elite purchase is not supplied by the default original progression graph. The original maximum outfit explicitly grants/refits Elite; no paid history or rank proof is manufactured for a test grant.

The original catalogue, fitting, ownership and source evidence use immutable L337 revision `a11fe16924d87d06021816f9037952afcbac08db`. See [full equipment reconciliation](EQUIPMENT-PLAYABLE.md) for exact source pins, including `EquipmentPurchaseProgression.swift`, `EquipmentModuleTemplates.swift`, `ShieldBooster.swift`, `InventoryInteraction.swift`, `EquipmentMutations.swift`, `ScriptEquipmentProjection.swift` and `Plugh.swift`. This candidate uses no imported player save.

## Resource and save criteria

- Store the previous module on replacement; operating with an empty shield slot is not an admitted player fit. Keep the same owned IDs and provenance on later refit.
- Scale both actual bank fractions on grade change without resetting damage, energy, heat or cooldown. The integer hundredths representation rounds to the closest hundredth; a depleted bank stays depleted. At actual physical berth the existing full-bank reset applies to the current grade only.
- Incoming ordinary and EMP effects consume the actual appropriate bank, and residual ordinary damage reaches the existing independent energy pool. Every grade retains the same recharge rate and finite capacity. The HUD divides actual amount by fitted capacity, never by the old Basic capacity.
- Continue replays retained actual fit, berth and damage chronology. A forged grade or bank value cannot authorize extra shielding. Basic-only saves retain the original four-byte bank representation; non-Basic uses a distinct reserved tag and full-width amounts, which an old reader cannot accept as a valid Basic bank.

## Disposable preview test access

The dedicated current `Test shield` action is a Meridian testing extension, not a recovered original secret key. Enter XYZZY explicitly at a real station berth. Its ordinary maximum action still fits Elite and all other currently supported maximum equipment. Select Test shield with Up/Down and choose any of the seven grades with Left/Right. Enter reviews; Enter confirms. Only that shield is fitted, with explicit test provenance for an unowned non-Basic item. It neither resets cash nor refills fuel, creates paid receipt or claims earned rank. A compatible held paid or test item is reused. Basic is the actual born foundation. Reopening a confirmed maximum outfit restores the actual held Elite item.

The dedicated test action requires active XYZZY and an actual current unpaused berth. All ordinary fitting-budget and durable publication rules remain. Selecting merely displays the choice, and a reviewing row cannot change grade. The remaining original maximum, PLUGH, earned-access and broader hull dependencies are tracked in the equipment ledger; test access does not complete them.
