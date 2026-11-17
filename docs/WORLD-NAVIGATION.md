# World navigation and physical destination admission

Historical feature contract and author-stage notes. For current delivered behavior, exact validation and open limits, read [implementation status](IMPLEMENTATION-STATUS.md). Earlier UNRUN statements below apply to the original handoff, not the latest delivered source.

Status: source and seven Rust tests authored; Rust execution and native UI acceptance remain pending on the existing Dell lane. This slice is based on station main `7aee6b6` and G3's connected freight/owned-starter commit `42a4e94`. It does not complete the first playable.

## Reference and content

All legacy reads use immutable `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. No human working files or player saves are read. `tools/import_navigation.py` extracts only the fixed catalogue Git blob, disables external diff/text conversion, validates its exact SHA-256 and emits the original 256 G01 IDs, names and Cartesian LY coordinates. The importer treats the source repository as a working directory, never an option or an arbitrary blob selector. The output hash, source hash and retained MIT notice are in `assets/legacy/navigation-manifest.json`.

| Source at the pinned revision | Preserved rule |
| --- | --- |
| `Sources/L337Core/Resources/OriginalWorlds/catalogue.json` and `Sources/L337CatalogueData/OriginalWorldCatalogue.swift` | Avenor G01-S007 `(52.4955,51.0662)` and Renas G01-S185 `(54.3368,48.2497)`; Cartesian hypot distance, with no classic anisotropic conversion |
| `Sources/L337Core/OriginalWorldNavigation.swift` | Minimum hops, stable system-ID order, current reach on the first leg and full-tank reach later; later refuelling is a planning assumption |
| `Sources/L337Core/FuelStorage.swift`, SH025 and original new-commander defaults | Stock 130 fuel tenths, 9 LY navigation reach, multiplier1 and nearest-integer distance times10 fuel debit; Avenor/Renas costs34 |
| `Sources/L337Core/HyperspaceDriveDefinition.swift` and `HyperspaceDriveRuntime.swift` | Actually owned stock Basic drive, ten-second activation and mass-lock scale1; SH025's older `[game].spool=15` is not the fitted drive activation |
| `Sources/L337Core/Game.swift`, `beginHyperspace`, tick update, `performNativeArrival` and `dock` | Flight-only admission, station center mass-lock distance600, fuel debit on arrival, retained attained speed, new gameplay visit, station16000 ahead, and docking cancels pending activation without debit |
| `Sources/L337Core/MainStationGeometry.swift` and `NativeStationAdmission.swift` | Actual original SH028 main orbital geometry, world/system station namespace, actor entity1 and positive visit/dock sequence; actor identity is distinct from display SpaceBody index0 |

The fixed 60 Hz host uses600 accepted ticks for the Basic drive's ten seconds. This is deterministic Meridian timing, not variable-delta Swift bit parity. The isolated scene contains the actual station and player hull only. Missing traffic, celestial mass locks, hazards, damage and general drive/storage upgrades remain explicit limits.

## World ownership and commands

`World::new_commander(seed)` creates the actual fresh Avenor profile with one owned SH025 starter and one commander account. The host supplies a unique deterministic seed per profile. Repeating a seed recreates the same replay; no global counter, filesystem or wall clock is introduced in core. Credits and fuel belong to the same account used by freight. Navigation does not own a second wallet or tank.

Fresh creation reserves eleven distinct nonzero IDs: owner seed, hull seed+1, inventory seed+2 and eight starter items seed+3 through seed+10. Zero and exhausted seeds refuse. The account's fallible birth creates and retains the exact Basic shield/navigation/energy/drive, front pulse laser and three missiles under StandardBirth provenance. Navigation binds the actual owned Basic drive item and rechecks the actual fitted owner/hull/navigation items; it does not interpret a caller's label as fitted equipment. These are Meridian profile IDs, not copied Swift save UUIDs. Root's host must keep profile seeds globally distinct when persistence is added.

`request_jump(SystemId, current_tick)` requires genuine profile/drive ownership, Flight mode, a known different G01 target, actual station distance, current fuel and source reach. Rejected stale, duplicate, unknown, mass-locked, out-of-range and unfunded requests leave World unchanged. Fuel is not charged at request time. Accepted ticks advance the private activation; rejected inputs do not. On completion World publishes the checked debit, destination and new visit together, retaining attained speed and entering Flight16000 ahead of the actual station. No dock context is granted by that hyperspace transition.

`current_dock_context()` returns an opaque private-field binding only for the actual initial start or a subsequent admitted physical station entry. It contains owner, actual ship, system, main-orbital actor entity1, visit, dock sequence, committed tick and `InitialStart` versus `PhysicalEntry`. Launch and travel invalidate the current binding. `dock_context_is_current` checks a retained binding against the actual owning World. Snapshots, catalogue rows, routes and public location IDs cannot construct this context or authorize freight progress.

Root's mission composition must use this actual context, actual account inventory and a real source-grounded contact roster to project G3's private World facts. This slice does not create Arlen/crew consent, publish mission receipts, implement a disk writer, refuel for free or provide save restoration. Existing pure local station constructors have no commander/location authority. Source stock and inventory must never be recreated as a load repair.

## Validation and limits

Seven authored Rust tests cover exact coordinates/fuel/route/content rejection; fresh owner/ship/location binding and snapshot non-authority; stale/duplicate/invalid-input countdown rollback and once-only arrival debit; public controls from hyperspace arrival through physical destination docking; docking cancellation before charge; actual station mass lock/range/unknown/tank exhaustion; and stable first-leg route assumptions. They are UNRUN until the Dell owner executes the composed artifact. The physical docking tests use real World controls/ticks, with no state injection or public receipt factory.

All13 Python importer tests pass, including four new navigation checks. Exact pinned catalogue re-import reproduces the committed output bytes. Rust formatting parses/formats the new source; whitespace checks pass. These checks do not establish Rust compilation, Clippy, GPU or playable completion. Root retains integration, independent review, required checks and merge ownership.

## SH025 navigation computers

The dedicated navigation slot uses the existing owned-equipment transaction and fit ledger. The Basic Navigation Computer is the born item. The Intermediate Navigation Module costs 3,000 CR at TL8 and requires Basic ownership. The Advanced Navigation Module costs 9,000 CR at TL11 and requires Intermediate ownership. A purchase fits its grade and stores the prior item. Storing or refitting changes the range through the same ledger.

The original a11 `EquipmentModuleTemplates.swift` and `FuelStorage.swift` give the Basic, Intermediate and Advanced modules limits of 7, 14 and 21 LY. The SH025's native 9 LY floor still applies when the Basic slot is fitted or empty. The actual limit is the lesser of the fitted-or-native range and the distance funded by current fuel. A stock full tank supports 13 LY, so either paid grade reaches 13 LY at most on this hull. The standard fuel cost and the Basic drive's 600-tick spool apply to all three grades.

The World regression uses Avenor to Enlirin (G01 system 162), 9.806708 LY. The stock fit refuses it. The Intermediate fit admits it at sufficient fuel and spends 98 of 130 fuel units on arrival. The pending jump continues from disk and keeps the original charge timing. Save format 17 binds pending range to the replayed fit. It retains an ordered receipt for each completed jump, including the actual system pair, departure visit, arrival tick, grade and debit. The reader checks every retained leg against catalogue distance and the fitting at that departure. It reconciles the exact debits against the cumulative navigation and account fuel totals before publishing Continue. This source reads only the current format 17. Its journal begins at the original Avenor state and stores every completed leg. Earlier format input refuses with a clear error before World replacement. The captured PR #44 format-16 and qualified format-15 files remain historical evidence and refusal fixtures.

Desktop travel selection and commerce remain limited to Avenor and Renas. The fitting screen and chart show the fitted range. Arrival at Enlirin grants no berth. Meridian has no verified Enlirin service binding, market, refuel or funded return route from the remaining 32 fuel units. Desktop acceptance for a paid range increase requires an evidenced outbound destination and a supported continuation.
