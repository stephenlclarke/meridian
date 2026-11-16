# Paid Mining Laser combat

## Source and actual behavior

Original immutable revision `a11fe16924d87d06021816f9037952afcbac08db`, `Sources/L337Core/LaserSpecification.swift`, defines Mining damage50 and interval0.3s. `GameplayWeapons.swift` admits actual fitted laser fire with energy>1 and heat<240, shared cost1, heat10 and the physical mount ray. The [sixteen-blob manifest](ENERGY-PAID-PULSE-SOURCES.json) retains the exact source files. This increment adds that combat role to the existing real paid3300CR/TL6 Mining Laser. It grants no property, money, ammunition, extraction authority or damage capability.

`World::request_fire_front_laser(tick)` reads the actual validated front fit and admits Pulse or Mining. Existing Pulse-only command/snapshot keep their strict meaning. `front_laser_snapshot()` reports actual ID, weapon class, source damage,6000-unit range and18-tick cadence. Existing `PulseShot` is the shared descriptive beam result; callers do not submit it as authority.

Both classes use the same physical ray/contact, nearest projectile, occlusion, pool, incident window, legal penalty, emission and practice observers. Mining consumes100 energy hundredths and50 heat fifths, then18 airborne cooldown ticks. This same pool blocks an immediate selected mining action. Combat heat<240 stays distinct from selected work heat<=230. No existing resource accounting/validation equation changes.

Ordinary receipts add `FrontMiningLaser` tag2, retaining Pulse0/missile1. Validation resolves actual fitted item AND class at shot tick/visit. Later born Pulse refit preserves the Mining receipt. Coherent stored-item or Pulse-class substitution refuses without changing live state. Interception likewise resolves historical fitted combat custody; projectile geometry, motion and retained body archive stay unchanged.

The private ordinary reducer uses source Mining50, Pulse15 and missile160. The supported station still admits no hull damage under its susceptibility guard. No public NPC laser victim producer exists here; the shield/energy case is explicitly an internal reducer fixture, not new NPC gameplay.

## Format and Root composition

Isolated current format is explicit draft v15 with a Mining-combat content marker. Existing v4-v11 boundaries and resource/archive layouts remain unchanged. Unqualified v12-v14 author drafts are not relabelled/migrated. Root chooses one actual final layout after MC1/profile/screens/HUD/all-hull serial composition. Existing preview files remain untouched; corrupt/unsupported saves refuse atomically.

Root generic fire input should use `request_fire_front_laser` and `front_laser_snapshot`. Extraction still uses the same actual `paid_fitted_mining_item`, sealed authority and `prepare_mining_resources` contract. Do not overwrite joined common files from standalone after-images or replace MC1's ray/cargo/depletion path.

## Checks and acceptance

Nine new Rust cases are **ALL UNRUN**: paid public firing/cadence/shared work cooldown; stored empty-slot refusal/cold custody; actual station receipt with class/item forgery and later born refit; actual born Pulse generic/Pulse equivalence; stale prepared Mining contact after Continue; distinct heat guards; Mining50 selected shield/energy reducer; docked/stale/paused public refusals; paid Mining physical interception with stored-born forgery and later refit. Public positives use genuine finite-market earnings and actual purchase, never grants. Unit resource/reducer fixtures are labelled.

Author checks are direct rustfmt, seven-file Markdown lint, whitespace and static immutable-source/assertion/numeric checks. No compiler, Cargo/tests, Clippy, GPU, native or app process ran; parent results do not qualify this source. G2 whole-source review and existing execution owners must run retained/new tests, strict Clippy and release on exact joined head/tree/lock, retaining failures and source pins.

Native requires earned funds, paid Mining buy/refit, physical beam/contact/interception, actual energy/heat/cooldown and work mutual exclusion, correct Mining receipt, history after real refit, and quit/coldContinue without grant, refill or profile reset. Static checks do not establish native acceptance.

Fast Beam/Military/Naval accounting/acquisition, all facings, shields, navigation/drives, targeting, EMP versus ECM, collection/salvage/rescue/passenger hardware, all-hull capacities and actual screen/profile integration remain open before new missions. Naval price0 is not a free offer;135000 secret-port purchase needs its actual discovered service.

The separately authorized [XYZZY / Commander CLARKE increment](XYZZY.md) adds explicit secret grants only to supported current equipment. Paid acquisition and earned evidence remain distinct; full secret maximum outfit stays a dependency.
