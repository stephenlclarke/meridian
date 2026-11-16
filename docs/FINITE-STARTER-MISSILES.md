# Finite starter missiles

Historical feature contract and author-stage notes. For current delivered behavior, exact validation and open limits, read [implementation status](IMPLEMENTATION-STATUS.md). Earlier UNRUN statements below apply to the original handoff, not the latest delivered source.

Delivered in the signed local Finite Missiles Preview from gameplay main `d5be89b` ([PR 17](https://github.com/stephenlclarke/meridian/pull/17)). Actual controls, rendering, three-round consumption, pending quiet/pause, first practice hit, station impact and no-refill dock/relaunch were qualified on Metal. The exact-source compiler suites passed; see [implementation status](IMPLEMENTATION-STATUS.md) for counts and limits.

**Historical version 7 save boundary:** the earlier installed preview refuses saving/restoring a missile-used session. Current repository source uses [complete version 8 missile sessions](DURABLE-MISSILE-SESSIONS.md). The earlier rule is retained here as the v7 contract. This includes ended projectile and missile-hit history. Arming or locking also refuses saving until explicitly disarmed and unlocked. The existing saved profile remains untouched; refusal never restores a consumed round or drops a live projectile. Full T02 continuation remains open. Earlier previews and real profiles remain unchanged.

## Immutable source and geometry closure

The reference is L337 `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`.

| Original owner | Retained rule |
| --- | --- |
| `Game.swift:872`, `CommanderEquipment.swift:173-179` | Fresh stock is three actual pylon rounds. SH025 capacity nine does not grant nine items. |
| `StandardAmmunitionTransactions.swift:100-165`, `AlienMissileDefinition.swift` | Select the highest occupied compatible fitted pylon, remove its exact physical item once and retain it as the emission's ammunition origin. |
| `GameplayOrdnanceRuntime.swift:4-12,74-115,150-155,184-194`, `AutoTargetControl.swift:5-7`, `Game.swift:6440-6453` | Native muzzle offset 24; actual admitted owner, distinct projectile identity and selected live target; explicit armed/front/cooldown checks and transactional finite consumption. |
| `GameplayOrdnance.swift:4-27,82-191` | Standard speed 792, damage 160, zero inherited velocity, one-second firing interval, snap guidance to the current live target and target-loss termination. The 6,000 range and 24-second lifetime are explicit project constraints. |
| `GameplayOrdnance.swift:394-432,489-545`, `GameplayRaycast.swift:465-476` | Nearest swept zero-radius contact; owner excluded until native player-radius muzzle clearance; ordinary station/player effects and body-before-player tie order. |
| `NativePracticeRuntime.swift:187-220`, `NativeSafeGunneryState.swift:162-222` | Exact reserved practice hits stay harmless and retain original emission attribution. Actual pending projectile origins block quiet credit; launch and end reset the interval. |
| `Geometry.swift:37-109`, `ShipCatalogue.swift:157-173,186-198` | The actual missile mesh is the seven-vertex/eight-face programmatic fallback. It has ten fan triangles; source outward-winding correction leaves all eight faces unchanged. |

All 171 immutable `Resources/Ships/*.toml` definitions were inspected. Only SH028 declares `nativeHull = "station"` and SH031 declares `nativeHull = "viper"`. The three resolved catalogue constructors in `OriginalBisonActivation`, `OriginalPeregrineMkIIIActivation` and `OriginalPlayerHullActivation` preserve `retained.nativeHull`. Therefore the catalogue cannot override `.missile`; the fallback is actual selected source geometry. The packet retains each catalogue hash and the constructor/source pins. No bitmap, model asset, texture or material is imported. `MISSILE_VERTICES` and `MISSILE_FACES` expose only this source geometry for the renderer.

## Core command and ownership contract

| API | Authority |
| --- | --- |
| `missile_snapshot()` | Read-only remaining actual pylon items, current arm/lock/cooldown and at most three projectile records. |
| `request_missile_lock(Some(id), tick)` | Requires that opaque ID to name an actual current qualified practice beacon. `None` explicitly clears selection. |
| `request_missile_arm(bool, tick)` | Explicit arm/disarm in current owned flight; arming requires an actual compatible round. |
| `request_fire_missile(tick)` | Derives owned launcher pose, selected live target, front-plane eligibility, native muzzle and actual highest compatible item. Publishes one distinct actor/emission after all fallible checks. |

No API accepts an ammo count, arbitrary target pose, damage value, hit flag, launch velocity or caller-created projectile. A consumed pylon becomes `None` in the actual account inventory; it is not retained as owned behind a display count or mask. The exact removed item remains in the stable projectile origin with commander/hull/system/visit, unique actor/emission, launch tick, physical clock, launch position, native launch orientation and muzzle. The original body keeps that launch orientation with zero spin; snap guidance changes propulsion direction, not the geometry basis. Launch spends no laser energy or heat, sets a 60-tick launcher cooldown and disarms. Dock/launch resets do not replace rounds.

Only successful 60 Hz World steps advance ordnance. Pause and rejected input leave its time and state unchanged. Standard guidance rechecks the actual live qualified target each step. The ray starts at the retained projectile position and clips to the nearest physical surface within that step, remaining 6,000 range and lifetime bounds. It includes the source-bound native station, exact practice cubes, the native SH025 player hull after muzzle clearance, and other live missiles using the programmatic mesh. Equal-distance bodies precede the player; the main station precedes practice and projectile actors. Fixed allocation order gives deterministic updates in this bounded scene. No NPC or celestial collider authority is invented from descriptive catalogue anchors.

Ordinary missile station/player contacts use the existing impact owner with actual missile weapon/item/emission attribution. The current original main station retains its original hull-damage immunity, ordinary law window and T02 failure semantics. Player contacts select the actual front/rear surface and debit shield then shared energy; the body ray is wired into the live handler. A geometry-only ray regression is not a live player-hit acceptance result. Such acceptance still needs an observed genuine contact.

A Pulse beam also queries live missile geometry. It can intercept a missile and consumes normal Pulse resources, but does not refund ammunition, create a practice hit or create an ordinary station/player offence. Other missiles can likewise intercept a live projectile. Terminated records remain evidence and stop moving; scene retirement culls live actors without replacing their original rounds.

## Practice and atomic publication

A missile's first exact practice contact retains its complete original emission in `PracticeBeaconSnapshot.missile_hit`. Repeated hits do not replace that receipt, flash or first-hit count. The beacon stays harmless and live; no ordinary damage, law, bounty, cargo or cash result is produced. Actual active projectile count is derived inside World and passed to its own practice state. Pending ordnance yields zero quiet credit. An end at the current endpoint starts a fresh interval rather than crediting preceding flight time.

World prepares the complete missile, impact/resource, legal and practice step before any actual arrival fuel debit. Counter/geometry refusal leaves the old World intact. The final debit and publication have no subsequent fallible operation. The account's finite consumption quote also pins the exact opening account before removing the selected round once.

This historical slice used version 7, with exact v4-v7 readers and original account bytes for an untouched launcher. Loading those already-supported inventories retains the actual saved three items; the loader does not call the fresh birth or consumption producer. Armed/locked/used missile state refuses `save_bytes`, confirmation-save preparation and replacement through `restore_bytes` before mutation. A future version must retain remaining physical ownership, live/ended projectile origins and missile hit receipts together before this boundary is relaxed.

## Qualification and Root desktop join

Ten authored regressions cover finite three-item custody and refusals; no inherited velocity and exact swept cube surface/first origin; pending quiet/pause/end timing; actual Pulse interception; actual target loss after an ordinary Pulse failure; range expiry before lifetime; actual missile station obstruction/failure; funded paid cargo-gear preservation and unsupported save/restore; code-native missile/player ray geometry; and exact old-version stock compatibility. Public controls and original market income provide positive World journeys. The pure player-surface ray test creates no impact or mission receipt. These original author-stage tests were subsequently qualified in the joined slice; the exact current result is recorded in implementation status.

The joined desktop binds F1 to cycle actual beacon locks, F2 to arm/safe, F3 to launch one round and F4 to safe/unlock. The HUD presents remaining rounds, current lock and live projectile count, with an explicit saving-unavailable message after missile use. All commands remain World-owned. `PulseContact::Projectile` has an explicit interception notice in the desktop pulse match. `OrdinaryImpact.weapon/target` describe actual attribution; active projectile rendering filters `ended.is_none()`. Retired records are evidence, not live actors. The renderer triangulates the original code-native constants at the authoritative projectile position and launch orientation; ended projectiles are omitted. Two authored desktop regressions cover actual controls/round custody/save refusal and live-to-ended rendering; the joined tests and scoped Metal controls/rendering were subsequently qualified. Each preview keeps a separate profile directory.

## Durable successor

[Version 8 missile sessions](DURABLE-MISSILE-SESSIONS.md) passed scoped real cold continuation on the signed staged Metal candidate. The earlier installed v7 preview retains its boundary until a separate v8 preview is delivered. Current qualification and distribution limits are recorded in [implementation status](IMPLEMENTATION-STATUS.md).
