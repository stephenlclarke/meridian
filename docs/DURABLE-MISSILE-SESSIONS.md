# Durable missile sessions

This guide records the delivered v8 preview. The authored [attempt-lifecycle successor](SAFE-GUNNERY-ATTEMPTS.md) retains these missile invariants and adds v9 attempt history; review and execution are pending.

## Status and scope

Version 8 replaces the deliberate finite-missile save refusal with a checked representation of the complete bounded session. Reviewed candidate `58f45c8` passed full Mac/Linux/native Windows checks: 193 top-level Rust tests, 48 importers, strict Clippy and desktop builds on each platform. Actual signed Metal cold checkpoints cover arm/lock, paused live flight, first impact, all three consumed/ended projectiles, physical docking and relaunch with zero ammunition. Isolated old v4-v7 Continue and invalid-checksum live refusal also passed. Root audited retained files, source and terminal records. [Implementation status](IMPLEMENTATION-STATUS.md) records precise scope and remaining limits.

The separate signed **Meridian Durable Missiles Preview** is installed from protected main `ab238c3`. Exact-main CI/Sonar, final committed-main rebuild/signature and installed Metal title/normal-close smoke passed. Use its adjacent launcher for the separate saves-v8 directory. The earlier v7 preview and real profiles remain untouched. See [implementation status](IMPLEMENTATION-STATUS.md) for exact delivered source and artifact identity.

This is persistence work. It does not qualify full T02, a live player hit, original-game parity, new missile targets, ammunition purchases or reloads.

## Wire state

The format keeps the `MERSAVE1` envelope, bounded length, content/resource fingerprint and checksum. Version 8 has a distinct rules header. The checksum detects corruption; it does not authenticate a file against coordinated rewriting of all history.

The account encodes eight explicit optional starter slots. The five foundation/laser items remain mandatory. An available pylon carries its actual saved item record. Each missing missile item must have exactly one retained launch origin, in the original highest-occupied-slot order. Its account revision is included in the ordinary wallet/cargo/fuel/equipment chronology. No birth, consumption or reload producer runs during decoding.

The launcher encodes arm, opaque qualified lock, physical cooldown and three fixed slots. Every allocated projectile retains its owner/ship/system/visit, unique projectile and emission, consumed item, legal value at launch, global and physical launch clocks, actual launch position, muzzle and fixed launch basis. Position, propulsion direction, range, age, clearance, target, last tick, contact, termination and interception are retained, including ended records. Allocation is not recycled by Continue or docking.

Each moved projectile also retains the last accepted movement segment, actual player pose and active missile body positions used by that segment's scene query. Ordinary missile impacts retain their full accepted incident. Beam interception retains its actual Pulse discharge. These are bounded private evidence records, separate from public rendering snapshots. Validation queries the same native station/player triangles, programmatic missile mesh and original beacon cubes; it does not apply an impact or publish an actor.

Practice first-hit slots retain their exact optional missile origins and pending count. Ordinary receipts encode explicit weapon and target tags. The failure, first-hit, pending, quiet-anchor and projectile endpoint references are checked together. Ended history remains history rather than becoming live bodies on load.

## Validation and publication

`World::from_save_bytes` decodes a candidate and validates the entire graph before returning it. `World::restore_bytes` validates the candidate and invalidates stale UI sequences before replacing the live World. Refusal preserves the live World. The host remains responsible for temporary-file flush, atomic replacement and keeping the prior valid file on failure.

Checks reject missing/duplicated or foreign custody, launch/projectile/emission/incident collisions, invalid lock/target/visit, future or inconsistent clocks, invalid bases and nonfinite or impossible travel. Last-segment validation checks the nearest native contact and endpoint; ordinary and first-hit receipts must point to that exact projectile. Interceptions must retain an actual beam or another projectile contact. Missile cooldown is checked against the physical clock; a physical berth resets it, while hyperspace arrival retains its countdown. Retained player-impact receipts bound resource validation and determine shield recharge numerically without applying damage during load. No live player-impact acceptance is implied.

The saved session pause flag is retained. Pausing does not age the projectile or credit quiet time. Pending actors suppress quiet credit, and the true end endpoint starts a fresh interval. Decoding does not advance either clock, emit a shot, remove an item or apply law/damage/progress again.

## Earlier versions

Versions 4, 5, 6 and 7 retain their exact rules headers and legacy wire readers. Their eight saved starter records are decoded as present; no fresh inventory is granted. Existing supported saves can subsequently be written as version 8. Version 7 never supported used missile sessions; the reader does not invent missing origins, consumed items or a migration for such bytes.

Test-only legacy writers exercise actual original layouts, including explicit version 7 after the version 8 writer becomes current. Existing semantic refusal tests retain their receipt-relative offsets. Assets and their fingerprints, dependencies and public projectile snapshots are unchanged.

## Checks and execution binding

Nine new public-journey and corruption tests cover explicit lock/arm and age-zero launch; active motion and first-impact continuation; pending pause/end quiet; all three actual consumed items through dock/relaunch; native station incident and once-only law; beam interception and target loss; custody/origin/target/motion/pending corruption; first-hit receipt and collision tampering; range expiry and live hyperspace cull; and all four old readers plus refusal of a fake used-v7 migration. The list includes combined cases within individual tests. Positive fixtures use actual World controls and existing finite markets; only negative fixtures bypass validation to create checksummed contradictions.

The existing funded paid-gear regression now continues the actual consumed session and still checks paid identity, cash, physical dock, unfit and no regrant. Existing old-stock coverage includes v8, and the old ordinary-impact test explicitly requests its v7 fixture.

The designated executors qualified the reviewed joined source through the full core/desktop suites, strict Clippy and `make ci`. The desktop now saves the complete missile state while preserving ordinary host publication rules. Outputs remain bound to their actual source/tree; later documentation changes reuse verified unchanged runtime proof. Source formatting or serialization alone does not qualify a host cold journey.

The joined desktop advertises F5 complete missile-session saving and F9 saved-state continuation while retaining explicit lock/arm/fire/safe controls. The existing host lease, temporary write, flush and atomic replacement are unchanged. An authored parent/child process regression saves a paused live missile, closes the owner, continues in a fresh process, checks exact saved state and compares the next unpaused tick with the original uninterrupted World. All opaque round and origin IDs remain unchanged. The actual parent/child regression passes on all three platforms.
