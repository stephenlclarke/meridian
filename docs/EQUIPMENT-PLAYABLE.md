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

## Current reconciliation

`Ready` below means source for the listed SH025 paths exists; the current preview can predate it. It does not claim new native testing, other hull compatibility or full parity. `Partial` names a concrete missing path. `Open` means no playable equipment implementation now. All rows require review of paid or appropriate non-paid acquisition, fit/storage, real effect and save/Continue before closure. XYZZY is a separate explicit acquisition path, not a substitute for ordinary payment or a mission receipt.

| Family and original grades | Acquisition and fit | Gameplay / Continue | XYZZY / remaining |
| --- | --- | --- | --- |
| Drive: Basic, Intermediate, Advanced | Born / paid / paid; this candidate | Actual timed travel and historical fit | Advanced; candidate |
| Shield: Basic, Basic II (Pulse), Intermediate (Mining), Intermediate II (Beam), Advanced (Military), Advanced II (Capital), Elite | Basic born; four successive shop grades; Capital is Elite-rank reward or paid PLUGH-only listing, Elite acquisition-only by default | Candidate: real capacity ratios 1/1.5/1.8/2/2.3/6.9/9.2, recharge 1x; wider banks, fit and damage replay. Actual rank and PLUGH still open | Elite maximum and separate explicit seven-grade selector; [source and acceptance](SHIELD-GRADES.md) |
| Energy: Basic, Intermediate (Extra), Advanced (Naval), Elite | Born; three ordered shop grades | Ready: recharge 1/2/3/4x, capacity 1x; higher-grade shop TLs constrain admission | Elite grant; more playable stations needed to reach paid Naval TL17 |
| Navigation: Basic, Intermediate (Advanced saved key), Advanced (Military saved key) | Born / two paid grades | Ready: 7/14/21 LY module reach with hull floor, actual fit and route history | Advanced; full original world/station scope remains |
| Targeting: Basic, Advanced, Docking, Identification, Pirate, Military | Six paid grades and stored identities | Partial: docking and work/target controller exist; verify each tier's distinct source effect and native controls | Military; original automatic options remain open |
| Mining and Salvage Targeting | Two independent paid modules require fitted core | Partial: rock hold, ore/wreck guidance; no positive complete salvage or ore intake | Both granted; custody and work closure open |
| Lasers: Pulse, Beam, Mining, Military, Naval | Pulse born; four repeatable ordinary shop/held definitions; Naval reward or paid PLUGH-only listing | Partial: real front paths; other facings and dedicated native Mining utility open | Naval front; not all supported native facings |
| Ammunition: Standard Missile, Alien Seeker Missile | Standard paid and finite nine pylons on SH025; Alien Seeker requires recovered research then repeat procurement | Standard ready; alien round, salvage variants and compatible hull pylon count open | Finite standard refill only; no virtual infinite inventory |
| Large Cargo Bay | Paid unique hull-bound expansion | Ready: actual SH025 92→127 t; full-hold removal refuses | Actual item granted; other hull expansion and unlimited cargo open |
| Fuel Scoops | Paid unique fitted utility; source F-only test fit | Candidate: real solar fuel and cabin thermal effect; physical ore/cargo intake owner still open | Maximum and separate scoop-only action; [solar source and acceptance](SOLAR-FUEL-SCOOPS.md) |
| ECM | Paid unique utility | Candidate: owned fit, exact manual field debit/duration/cold and HUD; incoming missile interaction remains open | Guarded automatic setting only; original maximum constrained by shared mounts; EMP is distinct. [Source and preview limits](STOCK-ECM.md) |
| Escape Pod | Paid unique utility | Open: eject, retained actual hull/body/crew and rescue/consumption | Maximum grant open |
| Energy Bomb | Paid unique utility | Open: finite owned ordnance, in-game effects and retained source | Maximum grant open |
| Galactic Hyperdrive | Paid unique utility (TL15) | Open: actual one-use inter-galaxy travel, destination and journal | Maximum grant open |
| Salvage Cutter | Paid unique compatible work utility | Open: real work slot, physical cutting and recovered cargo; SH025 source supports this work tool | Maximum only on compatible hull; open |
| Fuel Storage Unit (four physical repeatable slots) | Candidate: four owned, three normal shared SH025 utility mounts, 2,000 CR each | Actual capacity +70 each, separately paid refuel, occupied removal refusal, historical account and route proof | Explicit four-tank test maximum with custody guard; [source and acceptance](FUEL-STORAGE.md) |
| EMP Pirate Blaster; Power, Capacity, Recharge each grades 0, 1, 2, 3 | Paid utility, three independent sequential paid axes | Ready for current local finite pulse/capacity/recharge; broader original law/actors and native recovery open | Maximum fits utility and all three grade 3 axes; source values are Root-adopted balance estimates |
| Packed Stasis, Economy, Standard, Luxury, Single Stateroom, Double Stateroom | Six paid cabin layouts; each five tonnes and one unit | Open: 20/8/4/2/1/2 places, actual passengers, life support, fit/store and mission custody | No generic maximum grants should fabricate passengers |
| Component improvements and salvage quality I, II, III | Physical salvage and technology composition, not store substitute | Open: exact fitted-instance upgrades, original caps, preservation of condition/provenance | No automatic fabricated salvage or upgrade receipts |
| Alien Energy Capacity, Energy Recharge, Shield Capacity, Shield Recharge, Hyperspace Drive | Authentic recovered physical attachment; four scalar kinds upgradable, drive has no grade upgrades | Open: owner/hull binding, accumulated contributions and separate immediate drive branch | Must not grant by ordinary maximum outfit |
| Salvage missile variants Standard, Guided, Heavy, ECM-resistant; recovered hardpoints, pylons and other modules | Actual recovered technology, progression and fitting | Open | No fake shop stock or mission grant |
| Lerr Isolation Programmer, Torenran Inspection Kit, Tow Interface Reference, Assembly Reference Instrument, Atlas Reference Watcher | Five acquisition-only physical mission loans, not retail or permanent personal equipment | Open: actual issuer, item custody, capacity and return | Never a generic secret grant |

Basic fuel remains ordinary paid station service. Additional research recipes, physical salvage features, custom component tuning, passenger service, ordinary earned items and quest issue policies require their own source-to-runtime closure. The [existing XYZZY action inventory](XYZZY-ACTIONS.json) tracks non-equipment menu actions separately.

## SH171 Medusa compatibility

The original human clarification distinguishes the four explicit configurations. The mission instance is limited and beatable; the paid PLUGH showroom ship is stock and gets no free maximum fit; a legally fully upgraded player Medusa must be the most powerful ship under matched-condition and matched-attachment comparison. Do not copy the player maximum into the mission actor or replace the actual quest salvage/receiving requirement with a purchase. The source's exact limited equipment nodes and comparative power result are not yet established.

The frozen R3 proposal lists 22 t cargo plus 10 t expansion, five internal-organic missile positions, four laser facings, four utility mounts, 13 stock equipment units, one operating crew, and native-compatible cargo scoop, mining, salvage cutter, survey, rescue, salvage beam and auxiliary tank work capabilities. These are source design constraints for a future fitted-hull owner, not Meridian native acceptance. The 32-unit paid expansion and candidate performance targets (including energy 2,000 and shield 800) are explicitly **proposals**. The physical finite-layout experiment does not establish free volume or a biological mechanism. An authentic fitted alien hyperspace attachment confers its separate immediate, zero-mass-lock source policy; ordinary Advanced is still timed and cannot stand in for it. No Medusa hull or attachment has been admitted through this SH025 increment.

## Immutable source provenance

Revision `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. Exact Git blobs were read from an isolated historical object store. No human working checkout or player profile was copied. The following hashes pin this reconciliation; existing Meridian family-specific manifests retain related source and notices.

| Original path (under `Sources/L337Core/` unless noted) | SHA-256 |
| --- | --- |
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
