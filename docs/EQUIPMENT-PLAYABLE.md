# Equipment implementation and testing ledger

Scope: every stock equipment family and grade in the immutable original L337 catalogue, plus separately sourced earned, loaned, salvage and alien devices. The first Meridian increment adds ordinary hyperspace drives on SH025. Source is authored; compiler, hosted checks, independent review and native preview acceptance are pending.

## First increment: ordinary hyperspace drives

The original `HyperspaceDriveDefinition` provides three grades. Prices are credit tenths in source; table shows CR. The original displayed technology level is the stored value plus one. The basic device is part of ship birth, not a shop purchase.

| Grade | Price / TL | Fitting prerequisite | Actual spool | Station mass lock | Equipment units |
| --- | --- | --- | --- | --- | --- |
| Basic | Standard birth | None | 600 ticks / 10 s | 600 | 0 |
| Intermediate | 5,000 CR / 10 | Owned and fitted Basic | 300 ticks / 5 s | 480 | 1 |
| Advanced | 15,000 CR / 13 | Owned and fitted Intermediate | 60 ticks / 1 s | 360 | 1 |

One drive must remain fitted. Buying an upgrade pays once through the same account and stores the previous item. The player can fit another held drive in its place without another purchase. Direct storage that would leave the drive slot empty is refused. The station Equipment screen shows the fitted drive, stored item identities, price, technology, predecessor, spool and station mass-lock distance.

Jump admission, countdown and arrival use actual owned fit; the existing reach, fuel, station, pause and berth rules remain. Only the existing station-distance mass lock is changed; the original broader body/celestial exclusions and custom component modifications are not introduced here. A pending saved jump binds the fitted item; completed routes retain the grade and validate it against the actual historical fitting timeline before accepting the elapsed spool and fuel debit. No unlocked or shorter journey is inferred from cash or a displayed label.

The v17 record layout stays the same. The two low bits of the existing completed-leg grade byte remain the navigation-computer grade; the upper bits encode the drive grade, with Basic encoded as zero. Old Basic-only records are byte-identical. Unknown drive tags refuse. Older XYZZY saves retain their historical fit until an explicit new outfit, and the existing paid-base fixture is used unmodified. The earlier reader cannot accept new non-Basic records or new module tags; only forward Continue of existing v17 saves is intended here.

The [exact drive inverse](../integration/sh001-mule/HYPERSPACE-DRIVE-INVERSE.json) reverses only the 12 deliberately changed historical protected files to the immutable pre-drive source. The existing Mule checker still verifies all 115 original protected files and every prior inverse; extra edits to an approved file or the new manifest are rejected.

The existing explicit XYZZY maximum action now grants/refits the actual Advanced drive. It preserves a compatible held paid Advanced item, does not create paid history for a grant, never invents a paid Intermediate device and does not repeat a grant when already fitted. No other secret capability becomes available in this increment.

Authored new tests: source offers, cash/technology refusal, earned Intermediate purchase and failed writer rollback, paid replacement/refit, real five-second and one-second travel, cold pending and completed Continue, fitted predecessor and duplicate refusal, exact grade mass-lock boundary and mismatched pending item, forged faster historical drive, secret grants and reuse of a previously paid Advanced drive, and the desktop projection. The Advanced paid test uses the genuine checked-in legacy XYZZY cash fixture, distinct from the normally earned Intermediate test. Native input and process restart still require an owned preview lane.

## Naval Energy Unit at Aelavia

The original `EquipmentPurchaseProgression.swift` defines the Naval grade as a shop upgrade after Extra. At revision `a11fe16924d87d06021816f9037952afcbac08db`, `Game.swift` lists 196,000 tenths of a credit, internal technology 16 (display TL17) and triple base recharge. The same revision of `OriginalWorlds/catalogue.json` identifies Aelavia as G01-S235, TL17, with a hostile fraction of 0.4. Its catalogue SHA-256 is `120543eedc02a705cce0e37d026125bf926ec22651cff9337ca5d943f2ffd99d`.

**Player acceptance.** At Lareria, fit Advanced Navigation, two Fuel Storage Units and Fuel Scoops. Buy the Extra Energy Unit and fill the tanks. Fly the 20.55 LY leg to Aelavia, dock and buy the Naval Energy Unit for 19,600 CR under Equipment → Energy system. In flight, press Y. The SH025 should show 380 energy capacity and 12.6 energy/s recharge. After a save and restart the same paid unit should remain fitted. Use the local sun to collect enough fuel to return to Lareria. Aelavia offers this equipment purchase only. Its station has no commodity or fuel service.

The focused test at `b70f83c2618cf290fb47fb6a21ed99b3866914ca` failed on Dell WSL Linux (6.66 s) and native Windows (12.86 s). Each ran one test, with zero passes and exit 101. Both reached the paid-item cold restore and reported `InvalidMode` at `flight/solar/tests.rs:88:14` during yaw. Neither log contains a backtrace or the ship's temperature and energy at the failing turn. The revised return test starts braking at 2.65 solar radii at speed 8, samples the ship state at each turn, and checks the fuel debit after returning. The production heat and collection rules are unchanged.

## Current reconciliation

`Ready` below means source for the listed SH025 paths exists; the current preview can predate it. It does not claim new native testing, other hull compatibility or full parity. `Partial` names a concrete missing path. `Open` means no playable equipment implementation now. All rows require review of paid or appropriate non-paid acquisition, fit/storage, real effect and save/Continue before closure. XYZZY is a separate explicit acquisition path, not a substitute for ordinary payment or a mission receipt.

| Family and original grades | Acquisition and fit | Gameplay / Continue | XYZZY / remaining |
| --- | --- | --- | --- |
| Drive: Basic, Intermediate, Advanced | Born / paid / paid; this candidate | Actual timed travel and historical fit | Advanced; candidate |
| Shield: Basic, Basic II (Pulse), Intermediate (Mining), Intermediate II (Beam), Advanced (Military), Advanced II (Capital), Elite | Basic born; four successive shop grades; Capital is Elite-rank reward or paid PLUGH-only listing, Elite acquisition-only by default | Candidate: real capacity ratios 1/1.5/1.8/2/2.3/6.9/9.2, recharge 1x; wider banks, fit and damage replay. Actual rank and PLUGH still open | Elite maximum and separate explicit seven-grade selector; [source and acceptance](SHIELD-GRADES.md) |
| Energy: Basic, Intermediate (Extra), Advanced (Naval), Elite | Born; three ordered shop grades | Recharge 1/2/3/4x with the same capacity. In flight, Y shows the fitted unit, current energy, capacity and recharge per second. | Elite grant. Aelavia (G01-S235, TL17) offers the paid Naval grade. |
| Navigation: Basic, Intermediate (Advanced saved key), Advanced (Military saved key) | Born / two paid grades | 7/14/21 LY module reach with hull floor. Shift+R plots a route during flight using the fitted unit and available fuel. | Advanced; full original world/station scope remains |
| Targeting: Basic, Advanced, Docking, Identification, Pirate, Military | Six paid grades and stored identities | Identification or higher shows type and custody for current ordinary rocks and ore. Tab selects a contact and Shift+I expands its details. | Military; original automatic options remain open |
| Mining and Salvage Targeting | Two independent paid modules require fitted core | Shift+M selects the next current rock or ore with the Mining Targeting Module fitted. Shift+V selects wrecks with recoverable cargo with the Salvage Targeting Module fitted. Tab selects across all scanner contacts. See [automatic work](AUTOMATIC-WORK.md) for rock hold, ore intake and salvage. | Both available in the explicit maximum outfit |
| Lasers: Pulse, Beam, Mining, Military, Naval | Pulse born; four repeatable ordinary shop/held definitions; Naval reward or paid PLUGH-only listing | Partial: real front paths; other facings and dedicated native Mining utility open | Naval front; not all supported native facings |
| Ammunition: Standard Missile, Alien Seeker Missile | Standard paid and finite nine pylons on SH025; Alien Seeker requires recovered research then repeat procurement | Standard ready; alien round, salvage variants and compatible hull pylon count open | Finite standard refill only; no virtual infinite inventory |
| Large Cargo Bay | Paid unique hull-bound expansion | Ready: actual SH025 92→127 t; full-hold removal refuses | Actual item granted; other hull expansion and unlimited cargo open |
| Fuel Scoops | Paid unique fitted utility; source F-only test fit | Candidate: real solar fuel and cabin thermal effect; physical ore/cargo intake owner still open | Maximum and separate scoop-only action; [solar source and acceptance](SOLAR-FUEL-SCOOPS.md) |
| ECM | Paid unique utility | Candidate: owned fit, exact manual field debit and actual local standard-projectile termination; cold retains time-only cooldown and outcomes | Guarded automatic setting only; incoming missile interaction remains open; shared mounts constrain maximum; EMP is distinct. [Source and preview limits](STOCK-ECM.md) |
| Escape Pod | Original unique utility, 3,600 CR at TL6 | Open. Player ejection requires a retained abandoned hull, body, crew and rescue outcome. The frozen player action settles losses and docks immediately. | Maximum grant open |
| Energy Bomb | Paid unique utility | Open. Original use consumes the fitted item and requires more than 180 hull energy. Every eligible body needs an attributed result and each applicable destruction needs a settled incident before the action commits. | Maximum grant open |
| Galactic Hyperdrive | Paid unique utility (TL15) | Open. Costs 28,500 CR at TL15. The highest admitted shop is TL14. Arrival, station data and the saved journey must support the next galaxy before the fitted drive can be consumed. | Maximum grant open |
| Salvage Cutter | Paid unique compatible work utility | Source candidate: independent SH025 fit; finite ordinary external terminal-material custody | Maximum grants one owned unit; stores it when all three shared mounts are full |
| Fuel Storage Unit (four physical repeatable slots) | Candidate: four owned, three normal shared SH025 utility mounts, 2,000 CR each | Actual capacity +70 each, separately paid refuel, occupied removal refusal, historical account and route proof | Explicit four-tank test maximum with custody guard; [source and acceptance](FUEL-STORAGE.md) |
| EMP Pirate Blaster; Power, Capacity, Recharge each grades 0, 1, 2, 3 | Paid utility, three independent sequential paid axes | Ready for current local finite pulse/capacity/recharge; broader original law/actors and native recovery open | Maximum fits utility and all three grade 3 axes; source values are Root-adopted balance estimates |
| Packed Stasis, Economy, Standard, Luxury, Single Stateroom, Double Stateroom | Six repeatable paid cabin layouts; each five tonnes and one unit | Candidate: 20/8/4/2/1/2 places, healthy working support, separate private G04 occupancy, fit/store, reception and cancellation. See [passenger cabins](PASSENGER-CABINS.md). | No generic maximum grants; cabin damage, loss receipts and broader passenger services open |
| Component improvements and salvage quality I, II, III | Physical salvage and technology composition, not store substitute | Open: exact fitted-instance upgrades, original caps, preservation of condition/provenance | No automatic fabricated salvage or upgrade receipts |
| Alien Energy Capacity, Energy Recharge, Shield Capacity, Shield Recharge, Hyperspace Drive | Authentic recovered physical attachment; four scalar kinds upgradable, drive has no grade upgrades | Open: owner/hull binding, accumulated contributions and separate immediate drive branch | Must not grant by ordinary maximum outfit |
| Salvage missile variants Standard, Guided, Heavy, ECM-resistant; recovered hardpoints, pylons and other modules | Actual recovered technology, progression and fitting | Open | No fake shop stock or mission grant |
| Lerr Isolation Programmer, Torenran Inspection Kit, Tow Interface Reference, Assembly Reference Instrument, Atlas Reference Watcher | Five acquisition-only physical mission loans, not retail or permanent personal equipment | Open: actual issuer, item custody, capacity and return | Never a generic secret grant |

The ordinary Identification core costs 6,000 CR at TL9 and follows Docking in the paid equipment sequence. A stored core grants no readout. The source for the mining readout is `TargetingComputer.swift` (SHA-256 `096f8cee2331a03c60826d4f54c905c2967c9a31d55747cdfcda0815c623e7dd`) and `WorkTargetRuntime.swift` (`a4072efc794e3d0c0aec9eaa41d215a2ccb9a19b5634614e08a8f55aca4140e4`) at the immutable revision below. Ordinary belt rock is classed as `rock` and a released ore fragment as `cargo`. Both have `abandoned` custody while available. Selection is transient. After Continue, select the contact again to read it with the restored module.

Basic fuel remains ordinary paid station service. Additional research recipes, physical salvage features, custom component tuning, passenger service, ordinary earned items and quest issue policies require their own source-to-runtime closure. The [existing XYZZY action inventory](XYZZY-ACTIONS.json) tracks non-equipment menu actions separately.

## Escape Pod source gap

At the frozen revision below, `Game.escapePod()` refuses occupied passenger berths. It stages consumption of the fitted pod, settles mission and cargo losses, records abandoned people, clears the electrical charge, resets legal status and fuel, and calls `dock()`. It commits the candidate only after docking and commander validation. `NativeEquipmentConsumption.swift` binds consumption to the fitted item. `ScriptFreight.swift` records the escape cause against held units. The original clue-journal test checks that earned clues survive escape.

Meridian still needs an authoritative player rescue transition that identifies the abandoned hull, its people and cargo, the rescue actor and the arrival. The frozen player path clears the scene at docking. The SH040 Newt stock-inspection asset supplies its shape only. Neither supplies this transition or evidence of physical station entry. Ejection stays unavailable pending that source or an approved gameplay specification.

## Plot a route in flight

Buy the Intermediate Navigation Module for 3,000 CR at TL8 or above. With enough fuel and the Intermediate unit fitted, the computer can plot the direct 9.93 LY leg from Avenor to Lareria. Basic plots a route via Renas.

Press **Shift+R** in local flight to see feasible routes between the supported stations. Use **Up/Down** to select a path. **Escape** returns to flight with the previous plan. **Enter** saves the selected route. Press **J** to start the next jump once clear of mass lock. The commander save retains the accepted plan and jump progress. Dock at the next stop to advance the route. The original flight chart and route choices are in `PlayerHelp.swift` and `NavigationComputer.swift` at the frozen revision below.

## Energy in flight

Buy the Extra Energy Unit for 7,200 CR at TL9 or higher. It replaces the fitted Basic Energy System and stores the original item. Press **Y** in ordinary SH025 flight to read the fitted item, current hull energy, capacity and recharge rate. Press it again after energy use to see the new reading. Basic restores 4.2 energy/s and Extra restores 8.4 energy/s, up to 380. Storing Extra and refitting Basic restores the lower rate. Continue reads the saved fit and current charge.

## Mining contact selection

Buy the Basic Targeting Module and the Mining Targeting Module (2,000 CR at TL6) and fit both. In flight, press **Shift+M** to cycle current belt rocks and released ore in scanner order. **Tab** cycles all scanner contacts. **F** changes manual or assisted control. Manual mining uses the existing tool and scoop controls. The frozen selection rules are in `WorkTargetSelection.swift` (SHA-256 `66a21e1837b13bd58b63cd46300b2b47fcf3b55f823a9363b03e224e4c361b70`) and `WorkTargetRuntime.swift` at the revision below. Shift+M is the Meridian shortcut.

## Salvage contact selection

Fit a targeting core and the Salvage Targeting Module (4,000 CR at TL8). Press **Shift+V** in flight to cycle wrecks in scanner distance order. Each wreck must have cargo left on its external salvage manifest. An empty search keeps the current contact selected. Cargo transfer uses the existing cutter, scoop and hold checks. Use **Tab** for other contacts, including disabled ships with people aboard. The wreck selection follows the salvage hold in the same frozen `WorkTargetSelection.swift`. Shift+V is the Meridian shortcut.

## SH171 Medusa compatibility

The original human clarification distinguishes the four explicit configurations. The mission instance is limited and beatable; the paid PLUGH showroom ship is stock and gets no free maximum fit; a legally fully upgraded player Medusa must be the most powerful ship under matched-condition and matched-attachment comparison. Do not copy the player maximum into the mission actor or replace the actual quest salvage/receiving requirement with a purchase. The source's exact limited equipment nodes and comparative power result are not yet established.

The frozen R3 proposal lists 22 t cargo plus 10 t expansion, five internal-organic missile positions, four laser facings, four utility mounts, 13 stock equipment units, one operating crew, and native-compatible cargo scoop, mining, salvage cutter, survey, rescue, salvage beam and auxiliary tank work capabilities. These are source design constraints for a future fitted-hull owner, not Meridian native acceptance. The 32-unit paid expansion and candidate performance targets (including energy 2,000 and shield 800) are explicitly **proposals**. The physical finite-layout experiment does not establish free volume or a biological mechanism. An authentic fitted alien hyperspace attachment confers its separate immediate, zero-mass-lock source policy; ordinary Advanced is still timed and cannot stand in for it. No Medusa hull or attachment has been admitted through this SH025 increment.

## Immutable source provenance

Revision `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. Exact Git blobs were read from an isolated historical object store. No human working checkout or player profile was copied. The following hashes pin this reconciliation; existing Meridian family-specific manifests retain related source and notices.

| Original path (under `Sources/L337Core/` unless noted) | SHA-256 |
| --- | --- |
| NativeShipValues.swift | `98705f53ae126240c328b72c8c34f3402bbd72f02da99c5418cbade119c09e54` |
| NavalMissions.swift | `207eb8e4ba0e4428e89ddebdcbd1c438aa44808ee9b36095a649a484d8a19bb0` |
| HyperspaceDriveDefinition.swift | `931e8c50576e69dbaa41819b25e5a61c9ccdecefae9a581cb432ba76a89a91d3` |
| HyperspaceActivationPolicy.swift | `54f39bb030469535cef84db3737ff2e347ee9d741234d8135654852829041665` |
| HyperspaceDriveRuntime.swift | `28ab31e945096be00dd37d1280f9365af63762d70e5fe98b0cbccb957a17f4fa` |
| EquipmentRuntimeCatalog.swift | `ccf1ee2a940175f0d1a26a65aaa7b62ca7cf61d0eb33906f8b8feb6664210e27` |
| EquipmentPurchaseProgression.swift | `770618fcbb38330f568dbb862fc471762552cc17b55edfd407a1eb7462b0460c` |
| EquipmentTransactions.swift | `21a44066185ea17c8e3b262988c193cdef98bf88f80294d7a216767d93ad4f36` |
| InventoryInteraction.swift | `7d6e3e89b256577c4516365863071159f96fafc70588bbd9c307ca9e4d72a6cc` |
| EquipmentMutations.swift | `a1d0bece73e7d6370f4cab8fc952d2e25a3d20dfceb89ce8fab189c4b65941d5` |
| ScriptEquipmentProjection.swift | `a1c3ffad9881c7d02f5a6cd31e6a51015b5c7735f43781f54850197e9313dac1` |
| Game.swift | `e5b4e30c835d460aa7ca6f77364db0aaeaf877bb9d73bacc162e9111b26e2e17` |
| Plugh.swift | `2c43ae0738fa93375eb51a4859e11bc33e7bf9e7496ff432f140e813baa6b8b` |
| EquipmentModuleTemplates.swift | `387f5623bc0a390864b29ca5f2662c52bd091c54b024f8761bf8b098610a566f` |
| ShieldBooster.swift | `a7a91bc9d94a4f8e805192752a0ee69e2e9f5fb4117b48941dc68ad4bc2ec528` |
| AlienModuleDefinition.swift | `e0f98ac9a2e792c8d7083f18398b5a9a14a083506754eebac6e3033aad4264e9` |
| AlienHyperspaceDrive.swift | `a05fd68dfeb7f09d7f20d80cfce5815466c73e7100a735bee122b0591623faff` |
| AlienMissileDefinition.swift | `f780b221baec6568fca625f2f5cadb8239ed31e4acda12c41319edd97759946f` |
| PassengerCabinGrades.swift | `12fac9619f733666db4cba1c9f83732ade9f1b29ff19318b17d9b04813a6334c` |
| FuelStorage.swift | `b78beb5454d505217b71770d1c9daf7afb8be13fe81ab4b9d8c7725514c0c2fc` |
| EMPModuleGrades.swift | `11e19ac590798228b0aaaa2a841861ad3b4303ae95db9addbc019f4d483f51fc` |
| `docs/fleet-design-v2/ships/SH171.toml` | `f1e1365586a3723bed5a87f1041d086ab7a7dd3c4ce9d665c6fedce1b39eb48f` |
| `docs/fleet-design-v2/references/salvage/MEDUSA-FLAGSHIP.json` | `0e2e6639721f77392cdc5566a0df3b9eb41b038d61564d7753b7f64d760b8771` |
