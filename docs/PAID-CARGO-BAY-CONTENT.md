# Original SH025 paid cargo bay content

This component supplies one original equipment item to the paid equipment slice: Large Cargo Bay, original ID `cargo-bay`. Root owns the desktop join; G3 owns all Rust content decoding, account, commands and save changes. MC1 supplies only immutable content, extraction, provenance and independent source-derived fixtures from main `edaa29f8daefe849e55efcf69e7ed33d804aab77`. This component changes no installed app, profile, core or desktop behavior by itself.

## Original source and admission

All reads use Git blobs at L337 revision `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. The importer authenticates 23 exact inputs, cross-checks the original authored definition against the runtime price/TL, cross-checks SH025's original R3 fitting against its active TOML/profile/runtime projection, and requires the actual purchase/ownership/fit/service witnesses. [The manifest](../assets/legacy/equipment-content-manifest.json) records all source hashes, sizes, paths and witness lines. It reads no human working files or live saves.

| Source rule | Exact retained result |
| --- | --- |
| OriginalWorlds authoring `cargo-bay`; Game Equipment.base/requiredTechnologyLevel | 2,800 CR = 28,000 integer credit tenths; source stored technology 3 means required one-based TL4 |
| Original world catalogue | Avenor G01-S007 TL12 and Renas G01-S185 TL13 both qualify; ordinary price is unchanged at both |
| SH025 active game/R3 performance; ShipPerformance.standardCargoCapacity | Base 92t; fitted bay adds 35t; actual capacity 127t; held/stored bay adds nothing |
| EquipmentRuntimeCatalog | Family `utility-cargoExpansion`, slot `utility.cargoExpansion`; no slot if the hull has zero cargo expansion |
| Original SH025 R3 and OriginalFleetR3Fitting | 24 equipment-budget units, three utility mounts; counts actual fitted utility/fuel-storage locations; no role kit included |
| ScriptEquipmentProjection and EquipmentIssueTransactions.isIntrinsicFoundation | Actual standard shield/energy/navigation/drive at their true foundation slots cost zero units; laser/ammunition cost zero; ordinary bay costs one unit |
| EquipmentTransactions | Actual owner and active hull, authoritative price, sufficient actual cash and unique paid item; purchase auto-fits and binds the item permanently to that hull; duplicate refuses even if the paid bay is stored |
| OwnedEquipmentInventory and EquipmentMutations | Store/refit moves the same actual item, retaining paid provenance and bound hull; stale identity/revision or foreign owner/hull refuses; no refund or repeated debit |
| Game.validAfterEquipment and EquipmentMutations candidate validation | Shrinking to 92t refuses when actual occupied hold exceeds 92t; ordinary and identified mission cargo both remain aboard |
| NativeStationServices and Plugh | Purchase requires actual main-station equipment service and TL; source fit/store requires actual docking; no discount outside PLUGH |

The actual current eight starter items are the four standard foundations, front Pulse Laser and three fitted missiles. Under these source rules they consume zero equipment-budget units and zero utility mounts. Adding the paid fitted bay gives nine actual items, one of 24 units and one of three utility mounts, at its dedicated vacant slot. Storage returns usage to zero while retaining the paid bay in ownership. These are source-derived reference calculations; the Rust command must independently inspect the real retained starter fit and paid item rather than accept a caller's budget or count. No four extra foundations, spare devices, crew, cabins or role kit are issued.

This capacity effect needs no new hull mesh, scale, attachment geometry or G4 fleet change. It uses the existing supported SH025 physical authority. The pinned R3 `decisionStatus` still says implementation/playtesting pending and is retained verbatim. Content extraction does not qualify docking, acquisition of another hull or the new player journey.

## Exact consumer contract

`assets/legacy/sh025-cargo-bay.equipment` is 107 bytes, SHA-256 `2e99546d65fec686e65275599c0ac529a17551655678fd65e4b794e74104b83d`. All integers are unsigned little-endian. The binary contains exactly one definition with no separate definition count.

| Field in byte order | Width | Value |
| --- | --- | --- |
| Magic | 8 ASCII bytes | `MEQP0001` |
| priceTenthsCR | u32 | 28000 |
| requiredTechnologyLevel | u8 | 4 |
| baseHoldTonnes | u16 | 92 |
| extraHoldTonnes | u16 | 35 |
| utilityMounts | u8 | 3 |
| equipmentBudgetUnits | u16 | 24 |
| itemUtilityMountCost | u8 | 1 |
| itemEquipmentUnitCost | u8 | 1 |

The fixed header is 22 bytes, Python struct `<8sIBHHBHBB`. Next come five UTF-8 strings, each preceded by its `u8` byte length, in order: `id=cargo-bay`, `name=Large Cargo Bay`, `family=utility-cargoExpansion`, `slot=utility.cargoExpansion`, `hull=SH025`. Then `u8 siteCount=2`, followed by `<HB>` rows (`u16 system`, `u8 technology`) `(7,12)` and `(185,13)`, both in galaxy0. Reject wrong magic, field/schema/identity/count changes, malformed strings, truncated or trailing data. A selected site/TL or decoded definition is descriptive and grants no physical service, item or payment authority.

## Provenance and reproducibility

The equipment price and TL are in the original project authoring TOML whose complete semantic dictionary was compared during MC1's prior [original economy audit](../assets/legacy/original-economy-origin.json). That audit preserves the original generator/input graph and adoption evidence; the manifest binds its receipt hash. SH025's original project R3 source hashes to `a305641b7be704a2af0129e146d4ef1d72d165a1cbc7b935458c4c35aea14cc0`; its actual runtime TOML hashes to `aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883`. Their independently authored capacity fields agree. Historical generic labels/identities remain; this slice copies no Elite/Oolite third-party mesh, coordinates, material, bitmap or font. Retain `assets/legacy/Elite-L337-MIT.txt`. No commercial release approval is inferred.

Use Python 3.11 or later and an immutable reference object database:

```sh
python3 -B tools/import_equipment.py /path/to/reference/repository --check
make assets-check
```

The importer uses fixed allowlisted blob arguments, validates the repository directory, disables Git external diff/text conversion helpers and authenticates every input before conversion. `--check` compares both owned outputs and never overwrites differences. Omit it only to regenerate those outputs from the same pinned sources. No native compiler or running reference game is required.

## Focused evidence and remaining joined gates

Eleven new Python tests pass, including an independent byte decoder, exact price/TL reconciliation, source/hash/helper refusal, hull/profile/runtime cross-checks, actual station TLs, every required behavior witness, complete schema conversion and CLI tamper refusal. Standard-library trace covers 217/221 executable importer lines (98.2%, excluding compiler line zero). [The independent reference fixture](../tools/fixtures/sh025-cargo-bay.reference.json) records exact cash/TL/budget/cargo/identity boundary cases and explicitly distinguishes source-derived expectations from executed Rust or Swift behavior. Current source checking reproduces the exact binary and manifest from actual a11 Git objects.

The whole Root/G3 slice still needs independent diff review, exact-head owned Dell checks and actual purchase/unfit/refit/save/restart validation. That evidence must retain one paid hull-bound identity, the sole account debit, 92/127 capacity, real per-station stock, current legality and all identified G02 custody. Insufficient funds, stale/replayed commands, unsupported service, foreign/unpaid items and overfull unfit must leave all state unchanged. G3 owns version3 accounting/save and explicit refusal of unsupported version2; no migration, fallback, profile reset or repair is supplied here. The installed preview and its profile remain separate.
