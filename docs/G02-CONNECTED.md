# G02 connected freight domain

Historical feature contract and author-stage notes. For current delivered behavior, exact validation and open limits, read [implementation status](IMPLEMENTATION-STATUS.md). Earlier UNRUN statements below apply to the original handoff, not the latest delivered source.

Status: the domain has eleven authored tests. A successor now joins it to actual World navigation/contact commands and adds three authored public-command journey/refusal tests; see [WORLD-G02.md](WORLD-G02.md). Compilation, formatting, Clippy, test execution and save publication remain UNRUN for this exact source. Existing flight/station results do not validate these changes.

## Source and scope

The reference is immutable Elite/L337 commit `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. The authored pack content digest is `37ca73ed1b3f3656c42baedaf8dbaa3ee8aa3798ccf23e376c3c25ceac9bd572`. This is a new deterministic Rust domain implementation, not a save converter or a claim of game acceptance. It does not import the human working checkout or a live save.

| Original source | Rule retained |
| --- | --- |
| `Resources/ScriptPacks/l337-consented-freight/manifest.toml` and `scripts/main.lua` under `Sources/L337Core` | Arlen at Avenor G01-S007 issues the offer; Renas utility crew at G01-S185 receives; one or all remaining units; no deposit, deadline or special paid kit |
| `OriginalConsentedFreightSupply.swift`, `OriginalConsentedFreightSupplyCommands.swift` | Owner-bound batch `avenor-utility-parts-01`, sixteen units, two disjoint eight-unit allocations; no restock on abandonment |
| `ScriptOwnedCargoState.swift`, `ScriptFreightCollections.swift`, `ScriptFreightGeneration.swift`, `ScriptFreight.swift` | Identified Machinery custody, free hold check, same-owner transfer, remaining-unit prefix, completion on the final unit |
| `ScriptContractCore.swift` | One-leg reward `(620 + 110) * 10 = 7300` credit tenths; refuse reward overflow rather than clamp |
| `ScriptClueJournal.swift`, `ScriptBaseClueCatalog.swift` | First completion per source awards `consented-freight-family-trace`; later jobs retain that first clue receipt |
| `NativeStationServices.swift` and the original native/lifecycle freight tests | Actual current contact and main-orbital admission, confirmed review, source/content continuity |
| `Resources/OriginalWorlds/catalogue.json`, `authoring.toml`, SH025 original profile | Avenor `(52.4955, 51.0662)` to Renas `(54.3368, 48.2497)` is 3.364975176728651 LY: 34 fuel tenths; supported fresh SH025 has 92t hold, 130 fuel tenths, 9 LY reach and 7200 credit tenths |

The supported fresh SH025 transition creates and retains eight genuinely owned standard items: basic shield, basic navigation, basic energy, basic drive, front pulse laser and three missiles in their original pylons. `CommanderAccount::born_sh025(FreshCommanderIds)` is crate-private and fallible; it requires eleven nonzero, distinct owner/hull/inventory/item IDs allocated once by the actual fresh-profile owner. Item records are private, and only read-only inventory views are exposed. A caller cannot supply a valid-fit flag or replace inventory through this API. This creates the actual starter property within the sole account, rather than projecting an unverified drive ID. G02 does not require a newly purchased device. Paid refits, other hulls, firing/unfitting, damage, loss recovery, general inventory operations, content reinstall and deserialization are not implemented by this slice. G02 validates each retained item identity, owner, standard provenance, component and fitted hull, and requires the original basic drive to remain fitted. It does not add a requirement to fit a special paid device or retain unused fuel for a later journey. Numeric owner/unit namespaces are new Meridian identities; they are not original Swift save UUIDs. Preserve the original source notices and the repository's asset/distribution policy when composing the game.

## Commands and transaction

`freight::g02::Command` expresses offer review, receiving review, abandonment review, confirmation and Back. Review is bound to the full account, mission revision, current contact, content, World epoch and actual admission projection. Confirm consumes the live matching review; Back revokes it. Requested actor IDs are descriptive data, never a caller-supplied grant of consent.

Acceptance allocates eight identified units without charging a deposit. Receiving one requires the next held original unit; receiving all transfers every remaining original unit. Seven single transfers yield no payment or clue. The eighth yields one completion receipt, 730 CR and the first trusted clue. A second completed allocation earns its own 730 CR but leaves the first clue receipt intact. Abandonment removes held units with no reward, refund or replenishment.

`prepare` computes a bounded candidate without mutation. Dropping it preserves source, account, pending review, custody and receipts. `commit` consumes that candidate and compares the complete original state, account and World facts before any mutation. Public receipts cannot be fed back as payment or arrival authority. There is no public state constructor, context constructor or receipt factory.

This boundary does not implement a disk writer. The eventual World publication path must serialize the complete prospective game state, retain exclusive ownership through publication/commit, and prevent a failed disk publication from exposing the candidate. Do not perform an asynchronous write, change World, then treat a rejected commit as durable rollback. Real writer-failure and reload tests remain required.

## Historical domain boundary and current World join

The source baseline had only one local station scene. Its snapshot mode and descriptive station port do not establish Avenor/Renas arrival, a current named contact or owned equipment. No helper in this module is permitted to manufacture those facts from presentation state.

Root's World composition must own this mission state and its single account, use real fresh-profile creation exactly once, project current owner/hull/equipment identities and location/admission/contact facts, and debit ordinary fuel only through an actual navigation transition. Fresh identity allocation is an actual profile transition that Root must call once in World creation; it is not a load repair or an actor grant. The private owned inventory is retained with the same account. The crate-private station/contact fact structs are an integration boundary, not verified authority in isolation. There is no separate mission actor or second authoritative wallet. The original low-level `freight::SupplyBatch` remains isolated and cannot award this account's reward.

The successor World composition now supplies the public dispatcher, actual navigation/dock producer and current roster/contact selection described in [WORLD-G02.md](WORLD-G02.md). Its public-command journey is authored but unexecuted; persistent restore and native UI completion remain unverified. Source stock must never be reset by creating a fresh mission state during load or reinstall. Save representation and restoration validation remain work for the owning World composition.

## Authored verification

Eleven unit tests use explicit synthetic internal facts and the public command types. They cover original unit identity, seven/eight settlement, two disjoint allocations and source exhaustion, first clue retention, foreign/duplicate/out-of-order refusals, Back/replay/stale review, wrong owner/location/contact/equipment, hold/fuel checks, dropped/stale publication, final-payment overflow rollback, abandonment and suspended content. They also check that receiving does not demand fuel for a future trip, exact fresh-owned inventory, zero/duplicate/aliased birth IDs, forged equipment identities and refused unaffordable spending. The source of fresh-item semantics is `CommanderEquipment.swift:88-170`, `EquipmentTransactions.swift:75-94` and the initial three missiles in `Game.swift`; paid history is never invented.

These are domain transition checks, not actual physical arrival or native game acceptance. The Dell owner must format and run the complete joined source's core tests, workspace checks and strict Clippy. Private integration boundaries now have genuine World callers; no dead-code suppression is permitted. Normal paid fuel service supplies the existing account debit path.
