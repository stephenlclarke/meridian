# Four laser facings

## Author status

Core source and nine new regression cases are authored, **UNRUN**. This increment
starts at `65717daef31477389f1be6bb88b3b904bf31da4c`. Its qualified compiled
ancestor is `5e3e98a41a2ce3edaf824fdc25b5b6f98253573d`; only README, EMP guide
and implementation-status Markdown differ between those two commits. The ancestor's
631 Rust passes do not qualify this changed source. Root owns the desktop join,
then Root/G2 review the complete joined source before Dell/G1 execution. No new
compiler, Clippy, Rust test, native or GPU result is claimed here.

Author static checks pass: direct rustfmt, all 50 Markdown files, ten immutable
source pins, independent axis/wire-tag/wallet arithmetic and existing test-source
preservation. The delta leaves 285 baseline files byte-identical. These checks
do not execute the Rust implementation.

## Reference and admitted scope

[Immutable source pins](LASER-FACINGS-SOURCES.json) bind mechanics and notices to
original revision `a11fe16924d87d06021816f9037952afcbac08db`, tree
`68fec574df12ddf74acb14747e0dd6839f478642`. `LaserSpecification.swift` defines
front, left, right and rear mounts and independent fitted lasers.
`ShipPerformance.swift` assigns hull mask bits front=1, rear=2, left=4, right=8;
qualified SH025 has mask 15. These are original hull bits, distinct from private
save enum tags. No other hull, model alias or new asset is admitted.

`GameplayWeapons.swift::fireLaser()` selects the mount from the current screen;
its explicit mount command supports autonomous fire without changing the view.
Local directions are front +Z, rear -Z, left -X and right +X. All rays start at
player local zero, range 6000 and radius zero; there are no invented mount offsets.
The original shared energy, heat, cooldown and electrical-disable guards apply.
Pulse/Mining cadence is 18 ticks and Beam/Military/Naval cadence six ticks at the
existing 60 Hz core rate. One discharge costs 100 energy hundredths and 50 heat
fifths. Native nearest contact and occlusion remain authoritative.

## Core behavior

`LaserFacing`, `FittedLaserSnapshot`, `World::laser_snapshot(facing)` and
`World::laser_offers(facing)` expose actual owned mounts. Existing
`FlightView` is the persisted selection: `selected_laser_facing()`,
`selected_laser_snapshot()` and `request_fire_selected_laser(tick)` use it.
`request_fire_laser(facing, tick)` is an explicit mount command and leaves view
unchanged. Existing front commands retain their front-only meaning.

`ModuleAction::PurchaseLaser { kind, facing }` pays the existing catalogue price,
checks existing technology, capacity and prerequisites, creates one paid item,
and fits the selected mount. Existing default purchase/install APIs still target
the kind's default slot. `InstallLaser { item, facing, replacing }` checks actual
ownership and target occupant, moves the same item and stores the replaced item.
It never duplicates or repurchases an item. `Store { item }` preserves custody.
Nonlaser kinds cannot enter a laser mount. Naval remains unavailable for ordinary
purchase; an existing genuinely owned example can move between admitted mounts.
The starter Pulse's hull attachment follows the same born item across all mounts.

World derives the ray by exact signed permutations of the actual rigid body basis.
The actual native station, NPC, mining, missile and lesson queries select the
nearest contact; no caller supplies a ray, hit, target or damage amount. Resources,
physical clock and combat emission sequence retain one owner across facings.
Front mining work retains its existing separate tool authority; selecting a side
Mining Laser does not fabricate a side work-tool protocol.

## Current save contract

The existing private V17 envelope has a changed content fingerprint and refuses
older content. There is no migration, fallback or silently reconstructed fit.
Module actions reserve tag 17 for paid facing purchase and 18 for facing install;
existing tags retain their meanings. Nonfront damage events retain the nine-byte
footprint and reserve `64 + facing_tag * 8 + weapon_tag` (valid tags within 72-93).
Front event tags are unchanged. Actual historical fitted-item/slot replay resolves
the profile and shared spend at the physical clock.

Station receipts retain the actual body pose, from which the historically fitted
facing derives the ray on cold validation. Practice first hits additionally retain
actual item, facing and emission, check the exact selected weapon cadence and
native ray, then join to the shared sealed spend. Identical active/history mirrors
are allowed; conflicting targets or another station/NPC/interception result for
the same practice emission refuse. NPC and missile receipts use the historical
owned laser rather than an assumed front item. Corrupt restoration preserves the
prior World. Actual persistence execution remains pending.

## Desktop composition owned by Root

This branch deliberately does not edit desktop source. The new public enums need
exhaustive desktop match arms before all-target compilation. Root's complete join
must implement the following bindings against its current App source:

| Owner and anchor at base | Required binding |
| --- | --- |
| `app.rs::step_frame`, ordinary fire near line 64 | Use `request_fire_selected_laser(state.tick)`. Retain front mining work only when `selected_laser_facing() == LaserFacing::Front`; otherwise use the selected ordinary laser. Publish its actual `PulseShot` through existing render/audio state. |
| `app.rs`, front-fit gate near line 206 | Inspect selected laser snapshot for ordinary combat. Preserve any explicitly front-only mining authority. |
| `session.rs`, HUD near line 678 | Read `selected_laser_snapshot()` and selected facing; label fire and fitted kind for that facing. Keep one shared resource readout. |
| Existing F1-F4 view commands | Keep `request_flight_view(view, tick)` as the sole persisted selection; core already maps all four views. |
| `docked.rs::module_rows`, match near line 609 | Add Left/Right/Rear slot labels. Laser eligibility is `kind.laser_profile().is_some()`, while fitted status is `item.slot == Some(selected_slot)`. A laser fitted elsewhere must remain movable, not appear fitted here. |
| `docked.rs`, offers and review near lines 643-674 | Use `laser_offers(facing)` and `PurchaseLaser { kind, facing }`; use `InstallLaser { item, facing, replacing: actual_selected_occupant }` for fitting. Retain other families' existing actions. |
| `docked.rs::equipment_action`, near lines 553-569 | Translate the selected laser slot to `LaserFacing` for both buy and install. Resolve actual item and current selected-slot occupant afresh through existing review/prepare flow. Store uses actual item ID. |
| `docked.rs`, equipment tree near line 697 and slot parser near line 755 | Add `left-laser`, `right-laser`, `rear-laser` beside `front-laser`. Include facing in row identity so a movable item appearing under multiple mount choices has unique UI nodes. |
| `profile.rs`, action result match near lines 146-156 | Include `PurchaseLaser` with paid purchase messaging and `InstallLaser` with fit messaging. Retain prepared bytes, durable store publication and commit ordering. |

No host cache may mint a fitted item or bypass core review. Retained front-specific
host tests stay explicit front cases; new host cases must cover all selected views,
paid and stored item movement, refusal messaging, and actual atomic save publication.

MC1 independently owns ECM. Root must serially join the shared modular action/tag,
resource replay, missile and save fingerprint hunks, assigning noncolliding tags
and including both rules markers. This branch neither merges MC1 source nor claims
ECM behavior or tests. Recheck the complete joined source and its exact identity.

## Meaningful authored cases and acceptance

Eight cases in `flight/laser_facings/tests.rs` and one appended modular case cover:

1. Independently specified rotated body axes, unchanged origin and valid basis.
2. Three paid mounts with wallet 85349 -> 70849 -> 56349 -> 41849 tenths,
   unique items, store/move/refit and cold selected-item identity.
3. Foreign owner item, nonlaser item/kind and stale occupant refusals with identical save.
4. Unfitted rear refusal; actual side clear miss pays the shared pool and blocks front cadence.
5. All three nonfront rays hit the original station wall at Z=136.32, preserve body
   orientation and cold-replay actual item/endpoint.
6. Rear starter hits a real consented practice beacon and cold-loads after changing view.
7. Station occlusion refuses credit for the real practice target behind the wall.
8. Right laser damages a real native NPC attacker's shield after ordinary launch,
   flight and public steering; cash, food and living hull remain, with cold custody.
9. Changing saved paid mount after a real right station discharge invalidates checked
   save and atomically refuses forged cold restoration.

The 400-case baseline core therefore has 409 source test attributes after this
increment. This is a source count, not a pass count. Existing assertions remain.
After the complete desktop/ECM join and independent review, Dell/G1 run the pinned
`make ci`, explicit observer cases and locked release qualification in owned lanes,
retaining exact head/tree, commands, raw output, pass/fail counts and durations.
Any failure needs a causal changed delta; no unchanged retry or weakened gate.

Native acceptance then requires actual paid nonfront fitting, selected-view hit and
miss, wall occlusion, shared heat/cadence, save/quit/new-process Continue and refusal
through the delivered UI. Record source/binary/backend/profile identities and actual
before/after values. No native execution, full original parity, publication or
release acceptance follows from this author packet.
