# Durable missile sessions

## Status and scope

This source slice starts at merged finite-missile main `d5be89b9b1a515e9185d450687a558b20d9e1e7d`. Version 8 replaces the core's deliberate missile save refusal with a checked representation of the complete bounded session. The earlier installed preview remains untouched. Rust tests are authored and unrun in this source lane. Independent review, Root's desktop save/status join, G1 compilation and genuine host cold Save/Quit/Continue remain required.

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

## Authored checks and execution binding

Nine new public-journey and corruption tests cover explicit lock/arm and age-zero launch; active motion and first-impact continuation; pending pause/end quiet; all three actual consumed items through dock/relaunch; native station incident and once-only law; beam interception and target loss; custody/origin/target/motion/pending corruption; first-hit receipt and collision tampering; range expiry and live hyperspace cull; and all four old readers plus refusal of a fake used-v7 migration. The list includes combined cases within individual tests. Positive fixtures use actual World controls and existing finite markets; only negative fixtures bypass validation to create checksummed contradictions.

The existing funded paid-gear regression now continues the actual consumed session and still checks paid identity, cash, physical dock, unfit and no regrant. Existing old-stock coverage includes v8, and the old ordinary-impact test explicitly requests its v7 fixture.

Run the reviewed joined source through the missile tests, full core suite, strict Clippy and `make ci` using the designated executors. Root must remove stale desktop save warnings and refusal assertions, preserve ordinary host publication rules, and exercise a new process at live, paused, ended and exhausted checkpoints. Bind all outputs to the actual joined commit/tree. Source formatting or serialization alone does not qualify the host cold journey.
