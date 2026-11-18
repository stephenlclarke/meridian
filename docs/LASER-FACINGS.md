# Four laser facings

## Author status

Core and isolated desktop wiring are authored with nine new core and nine new
host regression cases, all **UNRUN**. The sealed core packet remains at
`f16cbc64bde7d36044c03e6b829c7137c8801361`; the host successor uses the same
branch without changing those core bytes. This increment starts at
`65717daef31477389f1be6bb88b3b904bf31da4c`. Its qualified compiled
ancestor is `5e3e98a41a2ce3edaf824fdc25b5b6f98253573d`; only README, EMP guide
and implementation-status Markdown differ between those two commits. The ancestor's
631 Rust passes do not qualify this changed source. Root owns final composition with current App, service, ECM and codec source, then
Root/G2 review the complete joined source before Dell/G1 execution. No new
compiler, Clippy, Rust test, native or GPU result is claimed here.

The sealed core author packet passed direct rustfmt, all 50 Markdown files, ten
immutable source pins, independent axis/wire-tag/wallet arithmetic and existing
core test-source preservation, with 285 baseline files byte-identical. The host
successor has its own static checks and unaffected-file catalog. These checks do
not execute the Rust implementation.

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

## Authored desktop wiring and Root composition

The isolated successor implements the actual App/session/docked/profile bindings;
it has not been compiled, tested or run natively. It supplies these complete
fragments for Root's final shared-source composition:

| Owner | Authored behavior |
| --- | --- |
| `app.rs::step_frame` | Held Z calls `request_fire_selected_laser` for ordinary combat. Front mining authority applies only in Front view; nonfront fire clears the mining visual and uses the selected actual laser. Existing shot publication and catch-up cadence remain. |
| `app.rs::help_panel` | Inspect the actual item fitted on the persisted selected facing, preserving original module names and the front Mining Laser work hint. |
| `session.rs` | Display the selected facing, actual weapon and one shared energy/heat/cooldown readout. An empty mount explicitly reports no fitted laser and does not fall back to front. |
| Existing digit controls | `0` Front, `1` Rear, `2` Left and `3` Right still submit `SetFlightView` to the sole persisted core selection. F1-F4 retain missile controls. |
| `docked.rs::LASER_SLOTS` and `equipment_panel` | Expose front/left/right/rear slots, actual fitted kind and selected-slot rows. The combined ECM panel has 19 slots including its utility and the nine existing pylons; the EMP slot moves from index 5 to 8. |
| `docked.rs::module_rows` | Show all eligible owned lasers as movable. Fitted status means this exact slot; Store is available only at the actual fitted slot. Readout names the current mount. Offers and prices come from `World::laser_offers`. |
| `docked.rs::module_node` and `equipment_action` | Facing-specific nonfront node IDs plus actual item IDs distinguish every row and offer. Cross-slot node/kind mismatch refuses. Buy uses the selected facing; fit resolves the selected occupant and uses `InstallLaser` even when moving back to Front. |
| `profile.rs::command` | Exhaustive purchase/install messages include both facing actions. Prepared bytes still publish through the actual Store before committing the candidate or invalidating the session. |
| `help.rs::flight_selected` and station Help | Nonfront controls name the selected weapon and shared pools, or explain the empty mount. Side Mining Laser means ordinary fire; front work alone advertises mining. Equipment Help names all four mounts and retains station navigation and modal bounds. |

Front default purchase actions remain the existing public front actions; all laser
fit actions use the facing-aware core capability. No host row/cache creates an
item or grants fitting authority. Old assertions remain except the explicit
combined starter slot count increases from 15 to 19; the existing EMP keyboard test pilot
advances eight slots instead of five to reach the same real EMP slot. Its earned
public-market helper is shared only within host tests without changing its body.

Root's separately verified Sonar extraction
`2d5f4a57ae05db68ec7f233f9b695f3ecc4b2f75`, tree
`16391c8e179353703b32ddb8d65e69f361f51dcd`, has parent `652b9c9` and changes
only core flight stepping, traffic movement and the desktop pylon helper. That
source is retained by the Root composition, which is serially
composing this successor; a whole-file replacement of `docked.rs` would lose the
newer pylon extraction. No qualifier or release state is inferred from those
source identities.

MC1 independently owns ECM. Root must serially join the shared modular action/tag,
resource replay, missile and save fingerprint hunks, assigning noncolliding tags
and including both rules markers. The Root composition includes the separate MC1 implementation and both rules markers. Its combined-source tests remain unrun; no ECM behavior is qualified by the earlier laser author packet. Review the complete joined source and its exact identity.

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

Nine additional desktop cases are authored in `app/laser_tests.rs` and
`docked/laser_tests.rs`:

1. Each actual digit view binding fires the matching fitted body axis, spends
   shared resources and retains selection/item/resources through a fresh Profile reader.
2. An empty selected mount never fires the front item; HUD and Help explain the refusal.
3. The existing genuine finite-market pilot earns 33650 tenths, then presented
   Left and Rear purchases each pay 14500, leaving 4650; Right refuses and cold custody matches.
4. Front mining authority cannot steal side Pulse fire; a paid side Mining Laser
   fires as an ordinary selected weapon and has the correct Help text.
5. Every station Help retains navigation; selected flight Help blocks view,
   held fire and publication, retains saved state and respects text bounds.
6. Four mount panels expose unique slot/node/item/offer identities and reject
   wrong-slot or foreign node selection without changing saved state.
7. Actual selected occupant replacement stores that item, preserves the born item,
   allows movement back to Front and cold-loads unchanged custody/wallet.
8. Actual fit and paid purchase writer failures preserve live World, wallet,
   items and the prior file. This case uses explicitly labelled XYZZY setup.
9. A stale reviewed mount/occupant cannot publish after another genuine fitting change.

The 400-case baseline core has 409 source test attributes and the 226-case desktop
baseline has 235, plus two unchanged compatibility cases and three retained observer
cases. These are source counts, not passing results. After the complete current
App/ECM join and independent review, Dell/G1 run the pinned
`make ci`, explicit observer cases and locked release qualification in owned lanes,
retaining exact head/tree, commands, raw output, pass/fail counts and durations.
Any failure needs a causal changed delta; no unchanged retry or weakened gate.

Native acceptance then requires actual paid nonfront fitting, selected-view hit and
miss, wall occlusion, shared heat/cadence, save/quit/new-process Continue and refusal
through the delivered UI. Record source/binary/backend/profile identities and actual
before/after values. No native execution, full original parity, publication or
release acceptance follows from this author packet.

## Targeted mounted-ray receipt correction

Whole-source review of Root `b5150cae85868c5768ee4b6757d8bf7352654e7e`, tree
`71e9163d39ee19039c5ac998fc809fb2d7698baa`, found P2: player-to-NPC and
missile-interception history could select another genuinely fitted equal-type
laser after coordinated item/facing relabelling, while retaining the old ray.
This correction starts at that exact combined laser/ECM source. It retains ECM,
all three 2d5 extractions and all existing collision/resource/save gates.

`MountedLaserEmission` records the actual command-time rigid body pose, selected
facing and physical clock. The sole World producer captures it independently of
the ray. Player-to-NPC `TrafficBeam` has this evidence; NPC-generated traffic beams
must have none. Interception `Beam` always retains it. The cold reader checks a
finite valid body basis, exact origin and exact signed facing-derived direction,
actual historical fitted item/slot/profile, and exact shared damage-event
physical clock/facing/item/weapon. Same-tick receipts additionally match the live
World body and clock. Interception native geometry now derives its ray directly
from retained body/facing rather than reconstructing a body from a stored ray.
Existing native nearest contact, occlusion, projectile/scene/emission identity,
weapon resources and whole-save budget remain authoritative and unchanged.

The evidence adds 105 bytes (clock, facing and twelve binary64 body components).
Traffic's optional field adds its presence byte; interception has mandatory
evidence. Existing resource events remain nine bytes. The private current-format
fingerprint changes with a distinct mounted-receipt marker, so old content
refuses before decoding. There is no migration, fallback or inferred facing.

Two meaningful regressions are authored **UNRUN**. Both buy distinct Left and
Right Pulse items through actual paid transactions using the existing genuinely
earned, completed G02/finite-market pilot, with no XYZZY funding or fabricated
NPC/ammunition producer. One lures and hits a real native attacker with the paid
Right laser. The other returns to Avenor, consents to real practice, flies beyond
the beacons, launches a real owned missile, turns through public yaw controls and
intercepts its still-airborne native body with the paid Right laser.

Each positive receipt advances one real tick and cold-loads before corruption,
so a current-tick body check alone cannot satisfy the case. Negative controls
coordinate damage-event facing with the other paid item, then also alter the new
receipt-facing field, retaining the real body/ray/endpoint/emission evidence.
They require checked save, cold load and live restore refusal with unchanged
original bytes. Another control changes the retained physical clock. No test
creates a positive body, actor, hit, resource spend or incoming NPC missile.

The existing traffic-only unit fixture supplies an explicit rigid body and its
forward direction for the newly required input. It remains a component fixture,
not a public journey. Existing EMP paid/defender helper bodies and assertions are
unchanged; only test visibility is widened for reuse. The P1 missing
`ecm` clone in the EMP control fork is corrected separately by Root and retained
in the joined successor.
This P2 source is not a whole-source compiler-clear claim. No compiler, Cargo,
Clippy, Rust/helper, native, GPU or review bot was run by this author.

## Joined slot and archived practice correction

Frozen Mac head `3553ea5b5f99ca261ed472513c8b79afe28d1dcf`, tree
`bda557c5e243b1e0a43eafd7297cbc7c9a64a250`, compiled all targets and passed
99 Python cases, 51 Markdown files and formatting. Its core run reported
415 passes and 15 failures in 55.79 seconds. The retained full failure receipt
separates eleven StaleOccupant errors from four positive cold InvalidEncoding
errors. No unchanged diagnostic rerun is used. Raw CI is 80923 bytes, SHA-256
`fd5d092e733ccd4a4da9c0ef31306526d09c97b4d33c7c93030331c7a6ec4655`.
Mac compatibility/desktop/Clippy/observer/release/native remained unrun.

The eleven nonfront fit failures share a concrete source inconsistency:
`Ledger::install_slot` permits laser profiles at the four typed laser slots,
but `scoops::validate_slots` still requires every fitted item to equal its kind's
default slot, which is Front for all lasers. Consequently real nonfront purchase
or movement fails the full fit check as StaleOccupant. `ModuleKind::accepts_slot`
now supplies that same explicit typed contract to both boundaries. Native SH025
identity, wrong-kind refusals, occupant equality, units, utility counts and mount
capacity remain intact. Laser profiles fit laser slots; every other kind still
requires its exact typed slot. This does not make mining work follow a side view.

The four cold failures all retain a prior practice attempt containing first-hit
records. `Practice::encode_full` uses the current writer, including each current
first hit's optional item/facing/emission tuple, but `decode_full` hardcodes
version 9 and therefore skips those bytes. That misaligns subsequent historical
fields and produces InvalidEncoding. The archived reader now uses its already
admitted whole-save version, matching the live practice decoder and writer.
No optional fields are ignored, inferred or defaulted, and no old content or
format is admitted. The strict marker and nine-byte resources are unchanged.

Every old test body and assertion is byte-preserved. Existing actual public fit,
paid item, coordinated corruption, wrong-slot, full-history missile and practice
cases must all pass after review. One new real rear-laser practice first hit is
physically docked, closed as abandoned and archived by a genuine retry consent;
cold Continue must preserve its original attempt plus the new attempt, selected
Rear view and owned item. This covers the new facing tuple across the historical
boundary beyond the existing front/missile history cases. The 431 core, two
compatibility, 244 desktop and three observer source attributes are not passes.
All corrected compiler/Rust/Clippy/native results remain **UNRUN** in this lane.
Root owns one complete changed-delta review and existing owners own execution.

### Bounded native setup artifact

`crates/meridian-core/examples/prepare_native_laser_ecm.rs` is source preparation,
not an executed helper or product feature. Existing public stock funding lives
in `equipment/tests.rs::earned_from`; sealed laser tests reuse it, and ECM
`ready` reuses its journey and Avenor practice consent. The EMP paid helper adds
its separate EMP purchase/defender journey. This artifact does not duplicate
that broad route. It chooses the explicitly permitted original public XYZZY
setup and labels all funding as testing provenance, never native earning.

After the corrected source passes compilation and review, G1 may compile the
example with the same corrected core/lock and run it against an explicitly
isolated current-save input and a distinct, absent staging-file path. It never
calls new_commander, creates a profile, discovers a default save directory,
changes an input file, imports an old EMP save or overwrites output. It uses the
real bounded World decoder, actual Avenor berth, public Enter and fuel-toggle
reviews/transactions. Original XYZZY fills the three utility mounts with cargo,
EMP and Fuel Scoops; the helper publicly stores only the granted scoops to free
one actual mount for later paid ECM. It leaves accounted fuel enabled, checks
real purchase and
practice reviews, cold-validates the final bytes and emits through create_new.
An I/O failure may leave a partial staging file; retain the failure and choose a
new output only after a changed/corrected run. The helper never installs that
file into a profile; existing G1 isolation/publication procedures retain that
responsibility with the app closed and real profiles untouched.

The helper leaves all paid fitting, firing, ECM and practice acceptance to native
input. Original XYZZY grants remain visibly labelled, including its supported
outfit and finite missile grants; these are not paid or earned items. Output
requires no pre-fitted ECM or earlier activations. Its public paid-review probes
include Front Beam, Rear/Left/Right Pulse, ECM and Mining. They do not guarantee
that all offers can be bought simultaneously or preserve the old review after a
purchase; the native screen must review every actual new transaction.

G1's intended native journey is: record XYZZY provenance and opening cash; buy
Front Beam (47000 tenths), three nonfront Pulse (14500 each), ECM (21000) and,
when needed, front Mining (33000). Four paid mount items then cost 90500 and ECM
adds 21000; Mining adds 33000. The born Pulse remains a distinct stored item;
using four new Pulse purchases would hit the existing four-per-kind cap. Store
and refit the same paid items and verify exact slot/identity/cash before launch.
Confirm actual Avenor practice with F10 then Enter. Exercise all four views and
selected Z fire, empty-mount refusal, front-only Mining work admission/refusal
without invented ore geometry, manual N ECM and paused/docked/no-hardware/active
field/energy refusals as reachable through real controls. Preserve each actual
refusal rather than claiming unobserved timing or damage. Help must retain modal
reviews and block commands; actual Save/Quit/new-process Continue must preserve
paid provenance, selection, pools, finite missiles and ECM journal, without
restoring active field permission. Retain actual binary/source/lock identity,
commands, raw log, input/output hashes and captures. None is observed here.
