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

## Source-derived geometry pilot correction

Actual Mac `ba8dc70bb6d7d0ddb8070869f4594d8bdb8b2174`, tree
`1ed634e72c30937d00880492c1fd3591f34bee86`, compiles and reports 428/431
core passes. The previous eleven StaleOccupant/four InvalidEncoding cases all
execute: twelve pass and three fail at later geometric/flight assertions. The
new real rear-hit archived retry case passes. The raw failure report contains no
actual contact, endpoint number or yaw angle for these three failures; none is
invented here. Static source analysis proves three setup inconsistencies. The
correction starts at sealed `2fb707b08cb167d0e7fc252adad4087a31e61025`, whose
core/desktop source matches this frozen ba8 qualification exactly.

Original SH028 native faces 482/483 form a two-metre front trim strip with X
68.12 through 70.12, Y -48.384 through 48.384 and Z 136.32. The old wall pilot
crossed X 70 at speed 1, then braked. Real movement uses 18 metres per speed unit,
acceleration/deceleration 9.1 and 1/60-second ticks: independent discrete braking
arithmetic gives about 0.8445 metres of additional travel, before waypoint-step
overshoot. This can leave the strip even though X 70 itself hits its front plane.
The corrected public pilot targets the original strip midpoint X 69.12 at speed
0.25 and checks the actual ray's source-quad crossing before firing. Its original
StationBlocked, Z direction, endpoint Z 136.32 at tolerance 1e-6, exact impact
point/item/body pose and cold assertions remain. No broader endpoint tolerance or
replacement collision surface is introduced.

Original faces 384/387 meet on their Y=0 edge from (105.632,0,-112) to
(96.8,0,-76.16). The prior rear-occlusion pilot approached X 90 at speed 4; braking
can drift about 15 metres toward an actual mesh opening. A bounding box or a
nominal waypoint cannot prove obstruction. The changed pilot derives its X
waypoint from the actual beacon and actual rear Z position, aiming through the
source edge midpoint (101.216,0,-94.08), and approaches at speed 0.25. An
independent source-plane equation, `35.84*(x-105.632)+8.832*(z+112)=0`, checks the
actual rear ray crosses within that edge before reaching the real beacon cube.
StationBlocked, zero beacon hits and full cold custody assertions remain. The
station aperture/native triangles are neither filled nor enlarged.

The old interception setup called a general yaw helper that subtracts raw atan2
headings. Across -PI/+PI it can request 270 degrees rather than its intended
90 while the real missile advances at 792 metres/second. That unsafe raw delta
is source-proven; the actual failed run's angle and terminal contact were not
logged, so they are not claimed. Only this finite-projectile setup now wraps the
yaw delta, retains a stopped ship, caps the intended quarter-turn at 146 existing
rate ticks and checks actual muzzle-to-beacon distance exceeds missile travel
during that turn. The following 120-step tracking bound, actual owned airborne
projectile requirement, real interception, finite spent round, no beacon award,
exact cash and historical cold/P2 coordinated corruption controls all remain.
Missile speed/range/lifetime, targets, hit geometry and production command
admission are unchanged; no projectile or positive receipt is fabricated.

The original SH028 TOML is pinned in [source references](LASER-FACINGS-SOURCES.json)
and byte-identically reproduces the admitted native physics asset. Independent
Python source/triangle/plane/braking arithmetic is preparation, not Rust or
native execution. The packet retains exact original raw qualification evidence,
all changed test source, untouched production/test proofs and execution criteria.
No test is dropped, skipped or added; all original named assertions retain their
meaning. Current 431 core, two compatibility, 244 desktop and three observer
source attributes remain counts, not passing results for this correction.
The current-main importer and its 32 cases remain separately owned and unrun in
that Mac failure; no 712-pass claim follows. The same sealed public XYZZY native
setup example remains uncompiled/unrun, for G1 only after changed compilation
passes and complete Root/G2 source review. Navigation/trade progress is preserved
separately and does not enter this delta. No compiler, Cargo, Clippy, Rust/helper,
native/GPU or bot is run by this author.

## Compact selected-weapon HUD correction

Exact Mac `2de5f2d7155fa4fef54386264a5e0227c38e5dfa`, tree
`e2b069905133009bb648978c196623c3b792f803`, reports all 431 core and two
compatibility cases passed, then 242 desktop passes and two failures. All earlier
15 core failures, the three later geometry/finite-flight conditions and the new
rear-hit archive/cold case pass on that source. Its actual total is 675 passes
and two failures among 677 executed top-level Rust cases. The remaining 32
importer and three observer cases, strict Clippy, releases, headless setup and
native acceptance are unrun. The original raw desktop failures contain no actual
HUD line dump; no truncation, content or resource value is invented from them.

The causal source path is explicit. `Session::lines` identifies default flight
as compact, calls `append_compact_flight` and returns. That builder emits missile,
practice/countdown and Help rows, while the selected-mount label exists only in
`append_flight_status`, which the compact path never reaches. The failed host
cases use default flight and no modal panels. They already pass actual view,
body-axis shot, owned-item and resource assertions before checking the omitted
HUD label. Profile's wrapper only adds its existing labelled status rows; no
later frame publication or renderer truncation is needed to explain the missing
line at the Session source boundary.

`selected_laser_status` now supplies the existing exact selected-facing/weapon or
empty-mount label to both builders. Compact flight appends exactly one short
`Z FIRE {FACING} {WEAPON}` or `Z {FACING} UNAVAILABLE: NO FITTED LASER` status row
when not spooling and with actual resource state. Expanded status retains its
existing energy/heat/cooldown row unchanged. The compact overlay does not call
the expanded status/control builder or reintroduce the movement/control block;
resources retain the actual cockpit owner and H/? retains the full guide.
Missile, practice, countdown, notice/review priority and Help behavior remain.
No core state, selected-view command, firing/mining admission, ECM control,
geometry, resource, save, asset, lock or profile publication code changes.

Both original failing predicates remain with actual HUD lines added only to
meaningful failure diagnostics. The same two existing cases additionally require
the exact selected label and H/? to fit above the real cockpit at 640x360, remain
within columns, exclude the full control block, produce bounded CPU HUD/cockpit
vertices and leave save bytes identical. The first case retains all four actual
digit/view/fire paths and actual Save/Quit/fresh leased Profile continuation;
the second retains actual empty-mount refusal/no fallback and selected Right Help
context. All other test bodies and every 431-case core source byte remain.
No case is added, dropped, skipped or weakened. These new checks are authored
**UNRUN**, not native rendering evidence. Corrected source needs one complete
Root/G2 delta review and fresh existing G1/Dell qualification before native work.
The sealed same-source setup helper and navigation/trade packets remain untouched.

## Stored Beam Fit review investigation

The internal `4d02e5123659a818f3bc8d5f8c735ba7ad1bb2f7` native run physically docked at Avenor, selected its paid Front Beam and displayed the owned/no-charge Store review. Enter committed Store and autosaved. Immediate Y produced a black client twice while the title remained DOCKED RUNNING. Escape recovered Equipment, with Front empty and the Beam stored. No blind confirmation or retry followed. The sealed save retains all module identities, five paid provenances, cash and finite missiles; only the paid Beam slot and fit revision changed. Practice, positive-hit and missile-termination acceptance stay blocked. Cause is **undetermined**.

`Docked::review_equipment` validates different Store/Fit commands, then builds the same Equipment review title and owned/no-charge details. `presentation::modal` discards the underlying screen primitives and emits the same modal labels, lines and controls. The pending command is not read by that renderer. `Geometry::compose` produces glyph and shape vertices before `Pass::prepare` uploads them; native drawing and surface presentation follow. Source inspection does not establish which boundary failed. No speculative GPU, layout, save or fitting change is included.

The new `docked_render/refit_review_tests.rs` case uses a disposable commander and labelled public XYZZY funding with accounted fuel. It stores granted scoops and actually pays for Front Beam, three side/rear Pulse lasers and ECM, leaving 9,888,500 cash tenths. It launches, brakes, yaws and returns through real physical docking admission without pose or dock injection. Actual equipment key mappings drive U, Enter, then immediate Y without an Equipment repaint. At 640x360, 1280x720, 1800x1125 and 3600x2250 it requires the modal title, item, no-charge details, enabled Confirm/Cancel, identical Store/Fit primitives and bounded finite CPU geometry with lit immutable glyph-atlas samples. These are coverage dimensions, not claimed native failure dimensions.

The test cancels Fit, requires the stored save unchanged, opens a fresh review and confirms through its actual button hit region. Paid IDs/provenance, cash and resources must survive autosave and cold Profile Continue after the original lease drops. In the sealed regression-only step, all original tests/assertions, assets, lockfile, App/Profile, core and production rendering remained byte-identical. That step added one desktop source case, **UNRUN**. The harness acknowledges CPU frames through the existing controller seam; it does not run the App title-prefix wrapper, upload, GPU or surface. It is not a proven failure reproduction or a causal fix.

The sealed native failure receipt pins the retained save and decoded observation. There is no durable screenshot, failed pending Frame, viewport/scale receipt, composed geometry or per-frame upload/draw/presentation result in the available evidence. Save state cannot reconstruct an earlier cancelled review or prove graphics success. Narrow boundary evidence is required to locate the defect; an unchanged native retry is not a correction. The external handoff records exact evidence/source hashes, preserved old source, acceptance criteria and designated-owner preparation without claiming execution. Existing navigation/trade work remains sealed.

## Bounded opt-in review boundary capture

The capture is causal evidence collection, not a renderer cure, public preview update or full acceptance. Enable it only in the designated owning process with `MERIDIAN_REVIEW_CAPTURE=1`, retaining normal stderr in the native receipt. Unset or other values leave capture disabled. It creates no files and reads no profile. The actual pending module command supplies Store/Fit token, item and fitting facing; purchase and unrelated reviews are excluded. App preparation failures are recorded when a real pending review exists. Otherwise the first actual draw attempt for each review transition is captured, including the post-App title prefix and optional Help/XYZZY overlay draw counts.

Each attempt records these boundaries:

| Record | What was actually observed |
| --- | --- |
| `host_surface`, `logical_window` | Physical window size, queried scale, configured surface descriptor, tick and logical window size used by preparation |
| `cpu_frame`, `cpu_labels_first8`, `cpu_hits_first8`, `cpu_clip_counts` | Actual presented-candidate stamp/revision/screen/viewport; total labels/characters/lines/hits/enabled hits/models/planets; bounded actual label/clip/control samples and finite-positive versus nonpositive clip counts |
| `compose_result`, `cpu_geometry` | Actual composer result; shapes/text/models vertex counts, corresponding byte lengths and length-delimited FNV-1a digest of those exact upload bytes |
| `queue_write_shapes_returned`, `queue_write_text_returned`, `queue_write_models_returned` | Each original write call returned, with its vertex/byte counts; completed GPU upload is unobservable |
| `surface_acquire` | Exact Success/Suboptimal/Timeout/Occluded/Outdated/Lost/Validation outcome from the normal acquisition call |
| `draw_calls_encoded` | Docked models/shapes/text vertex ranges when those calls were encoded; HUD vertex count and whether the HUD/Help overlay was encoded |
| `queue_submit`, `surface_present`, `surface_reconfigure`, `draw_return` | Original API calls returned and normal application result/error; no GPU completion, compositor display or framebuffer readability claim |
| `asynchronous_error` | Existing device-loss/uncaptured-error callback, at most twice; latest-review correlation only, originating GPU frame explicitly unobservable |

The digest is diagnostic FNV-1a over each byte slice prefixed by its unsigned 64-bit little-endian length, in shapes/text/models order. It is not a security hash or cross-version compatibility promise. Text vertices divided by six give emitted glyph quads. The data are the same bytes supplied to the existing queue calls. Metadata and identity are kept separate from original paid provenance, which remains in the sealed native save receipts. No profile inspection or synthesized review is introduced.

The entire process admits at most eight transitions, twenty records per attempt and two asynchronous error records. Each line is at most 4096 UTF-8 bytes, with an explicit truncation marker, and begins `MERIDIAN_REVIEW_CAPTURE v1`. Repeated frames for the same token/item/operation/facing produce no further attempt. A transition whose first attempt skips or errors is retained as that outcome; the capture does not retry it or silently replace it with a later success. Resetting the controller or returning to the same review after an intervening nonreview frame may count as another transition within the same global cap. Native process restarts start a new capture budget. No timing, frame-rate, memory sampling, queue completion wait, GPU readback, shader or pipeline change is included. Logging can affect scheduling while enabled, so the evidence does not establish timing parity.

Source tests cover disabled/nonreview admission, same-review suppression, both global bounds, escaped/truncated errors, explicit unobservable claims, a separately calculated digest vector and sensitivity to buffer order/boundaries/changed bytes. The paid/physical-berth regression also records actual Store/Fit identities/labels/clips/buttons and binds the actual CPU composer byte counts/digest. All tests remain **UNRUN** in this author lane. Every original assertion remains. One complete cumulative source packet includes production capture, regression, raw earlier failure receipts, criteria, unaffected-source proof and designated execution preparation.

After the existing independent source review and changed-source G1/Dell qualification, the native owner can perform one materially changed paired Store/confirm/Fit workload on the actual retained paid state, retaining the unchanged original failure evidence. The currently retained Beam is stored: do not blindly confirm an invisible Fit or mint a replacement item to create a baseline. Any prerequisite visible refit and new Store must be genuine supported controls with exact identities/cash preserved; if unavailable, record that limitation and stop. Capture opt-in does not authorize a public preview or claim the blocker fixed. Pair the captured boundaries with what the real native window displays. Missing records, failed acquisition, truncated samples, process termination or unobserved GPU/display completion stay explicit; no success is fabricated from a returned void API call. Preserve the first result without unchanged retries.

## Immediate Store-to-Fit revision handoff

The actual `658c9506afccb37122f7ce787641d796ab225ce3` Mac run compiled, then passed 248/249 desktop cases. All four capture guards passed; the public paid/physical-berth case failed at its original missing-review assertion. Its actual frame was ordinary Equipment with Beam stored and a stale-screen footer. No native capture ran. This failure is preserved separately from the earlier black native Fit review, whose cause remains undetermined.

`Screens::handle` validates the painted stamp/revision, then dispatches Confirm and bumps `ui_revision` for the returned request. `Docked::request` commits Store through Profile and clears the pending review. The painted frame remains the last actually acknowledged Store review. Immediate Y reaches the same stamp but old UI revision and is refused before Fit dispatch. In production, `App::key` calls `menu_input`, which calls `menu.input` then requests redraw; it does not synchronously render or acknowledge Equipment. `complete_presentation` alone acknowledges a later presented frame. The regression's immediate chronology therefore exposes a real caller gap rather than a missing test acknowledgement. Removing the revision bump, accepting arbitrary stale inputs or adding a test-only Equipment repaint would hide that gap.

The correction retains the entire controller stale-input guard byte-for-byte. After a successful Store command, Docked may hold one ephemeral handoff for that same stored item/node, selected slot and existing post-store Anchor. Only Y may mark it requested before the next normal frame build. This does not dispatch a World command, confirm fitting or acknowledge a frame. The next ordinary `build_frame` consumes it once, rechecks active Equipment, pending absence, slot and Anchor, then uses the existing Fit-review builder and validation. It emits the Fit modal in that next frame; there is no intermediate Equipment repaint. The new Fit retains its own token and current anchor and can be confirmed only after normal presentation acknowledgement.

Any other input discards the handoff. Open/close/focus loss clears it. An unused next build expires it. Anchor drift before Y refuses the deferred path; drift after Y drops it during build with the normal fresh-review feedback. Existing row/fit validation still applies. The state is bounded to one handoff and never serialized. The core, save format, capture pipeline, renderer, App scheduling and controller guards do not change.

The failing paid/physical-berth case retains its U/Enter/immediate-Y chronology, original missing-review assertion, all four viewport/modal comparisons, paid identities, cancellation and cold Continue. Added checks require the next built Fit modal before any intervening Equipment frame, unchanged stored disk/World bytes, and refusal of Enter and an old Store button hit while that Fit is built but unpresented. One new public-control case covers immediate second Enter, queued-Y focus loss, real paid inventory/cash anchor drift and close; none may open or commit a deferred Fit. No test weakens the review requirement or fabricates presentation success. Corrected tests/compiler/Clippy/native remain **UNRUN** in this author lane. This fixes the source-supported deferred-review gap only; it is not evidence of a native black-screen cure.

## Cold Continue byte contract

Actual Mac source `7661fac996a5430c789f0378be502b41d80632bb` compiled and passed 249/250 desktop cases. The paid/physical-berth case reached its final `assert_eq!(ok(cold.world.save_bytes()), bytes)` and failed. Its earlier Store/immediate Fit, four viewport, paid refit, disk and cold inventory/view/cash/resource assertions passed within the failed case. This is not whole-case acceptance or native rendering evidence. No native capture ran.

The actual vectors are both 5605 bytes. The original file has SHA256 `21b4eceb24331c8c904919d7070232491505609e04bbc4eea1f885dc84278e16`; the continued World has SHA256 `56e7085540bcdf3fcd949fc60bfc2f144d1d79274b68431bf01c0ab8b9286a27`. Source-derived layout places the UI review sequence at offset 2126, changing 2 to 3, and session epoch at 2134, changing 5 to 6. Only those payload fields and bytes 5597 through 5604, the final checksum, differ. These offsets describe this observed vector, not constants used by the corrected test.

`Profile::continue_saved` loads and validates the old file through `World::from_save_bytes`. That loader invokes `FreightSession::invalidate_after_continue((0, 0))`: the review sequence and epoch advance once, pending permissions/contact are cleared, and owned items/roster/dock stay retained. Both counters are intentionally serialized. Existing core repeated-restore tests require revocation of stale reviews. Existing Profile cold tests compare the complete public-loader result and separately retain the old disk file. Requiring raw pre-Continue and post-Continue bytes to match conflicts with this established contract. Removing invalidation would break a safeguard.

Root explicitly authorized replacing this new incorrect predicate after reviewing the actual vectors and source. The sealed raw log, vectors, predicate and hashes remain evidence. The replacement does not shadow `bytes`, mask differences, normalize the result or reduce equality to selected snapshots. The test-only `continue_oracle.rs` walks the actual V17 prefix: literal content identifier, flight fields, navigation options, account/starter-item options, commodity arrays, equipment receipt and empty freight fixture. Only the content identifier is read as literal source data; no private codec is invoked. Field offsets follow the walked schema rather than observed numeric offsets. Unsupported format, changed fixture or malformed tags fail explicitly.

The independent oracle clones the original expected byte vector, increments exactly the located UI review sequence and session epoch once and computes FNV-1a checksum independently. The complete actual continued bytes must equal that expected vector, including checksum. Additional assertions require exact increments, identical payload segments before/between/after those fields and a valid continued checksum. A separate public-loader result crosschecks the oracle; it is not the sole expected result. The entire original disk file must still equal the original `bytes` after public Profile Continue. The expected vector is test data only and is never imported as a save, applied to World or used as authority.

Offline arithmetic over the retained first-failure vectors independently traverses this same source-defined layout and exactly reproduces the actual continued vector. This proves the explanation for the recorded mismatch without executing the product. Corrected Rust/compiler/Clippy/native results remain **UNRUN**. No production/codec/save-format or anti-replay behavior changes, and every other old assertion remains. The earlier native black client remains a separate unresolved cause; this test-contract correction is not a renderer cure or public preview update.

## Constant-size glyph chunk iterator

The actual `80eb3c8` run passed all 285 fresh desktop/importer/observer cases, including the paid refit and independently checked cold-Continue bytes. Strict Clippy then rejected the new glyph test's `chunks_exact(6)` iterator. The correction uses `as_chunks::<6>().0` over the same slice: the same complete six-vertex groups are checked and the remainder stays excluded. All assertions and production bytes remain unchanged. Changed Clippy and release builds are unrun; this is not native black-screen clearance.

## Cross-frame native review evidence gap

Root's one changed signed `6301a8d73b1ce7fdea63e63d7c3b03e2716056b2` native workload showed a visible prerequisite Fit (token 1), confirmed it, showed Store (token 2), then committed Store and immediately pressed Y. The paired Fit (token 3) appeared black. No blind Enter or retry followed. Escape recovered Front empty with the same paid Beam stored and cash retained; Root saved and quit normally. The earlier 4d02 failure remains preserved. Current native display is therefore an actual blocker despite qualified CPU tests.

The retained 15,100-byte stderr log contains exactly three captured transitions with seventeen stage records each. Their geometry, labels, clips, enabled buttons, logical/physical dimensions, draw ranges and successful acquisition/API-return values match. Geometry is shapes/text/models `[12, 774, 0]`, bytes `[288, 21672, 0]`, diagnostic digest `6758444242580820565`. The actual stamps are 3043, 5100 and 6356, with UI revisions 5, 7 and 9. Host ticks and stamps differ as expected. The eight-transition cap was not exhausted. No asynchronous GPU error was recorded; absence is not completion proof. The title/geometry are present in the first token-3 attempt, so this evidence does not support a missing-title diagnosis.

The exact source path is App frame preparation, post-XYZZY-prefix Frame, Renderer draw, docked prepare/compose, three queue writes, surface acquire, black-cleared model pass, overlay pass, queue submit, pre-present notification and void surface present, followed by App `complete_presentation` and `Docked::presented`. Every CPU composition clears and rebuilds vectors; every normal docked draw stages the current vectors and uses their current lengths. Persistent GPU buffer/pipeline/glyph bind-group ownership stays in Pass. The pinned local wgpu 29.0.4 queue API states that `write_buffer` immediately copies input into staging, and next `submit` executes those transfers before submitted commands. Reusing the CPU Vec after that call is supported; no evidence justifies a lifetime patch or an extra submit/wait. Modal model and planet lists are empty, celestial preparation clears its active flags and the scissor is reset before overlay. No operation-specific shader, resource replacement or delayed conditional clear was identified by source inspection.

The current recorder admits only the first attempt for an identity. Every later `begin` for that same identity returns None. It neither records App's later acknowledgement nor the GPU target pixels/completion. The black screenshot was observed later and has no bound frame ID. Equal first-attempt inputs/API returns therefore cannot distinguish a later CPU/state/upload/draw change, an incomplete GPU pass, or a later surface/compositor/display issue. These are unknown boundaries, not diagnoses. No renderer, layout, queue-order, capacity or GPU workaround is included in this analysis.

A necessary changed capture must retain the original first-attempt records and add bounded cross-frame correlation. The proposed contract is an explicit opt-in version with a monotonically increasing attempt ID, Frame stamp/UI revision and pending identity, linked to the actual App acknowledgement outcome. Keep a fixed-size latest-attempt ring while that review remains active; flush it only at an explicit diagnostic observation marker, review transition/exit or error. Preserve first and last outcomes separately, count overwritten records, and never relabel a skipped/failed first attempt as success. Constant storage and process-wide byte/transition budgets must be tested. It must observe current labels/geometry/physical configuration/encoded ranges and normal API errors for those retained attempts, with no general frame-rate, memory or timing probe.

To distinguish actual rendered target content from returned API calls, the proposed capture also needs a small fixed number of explicitly requested pixel-copy slots bound to those attempt IDs. Check actual surface format/usage support first. Only when the existing target supports copy-source usage may the diagnostic encode a copy from that exact final colour target after overlay and before present, through an aligned bounded readback buffer. Bind submitted work, actual map callback result, format/row-stride and immutable pixel artifact to that same attempt. Do not invent support, change shaders/layout/surface format or call a void present successful display. Unsupported usage, oversize target, map failure, missing completion or unfinished capture remains explicitly unobservable; do not substitute a rendered clone or synthetic review. This source-audit requirement is implemented in the v2 section below, whose changed execution is still unrun; it does not authorize a native launch.

The GPU artifact would establish copied target pixels for one named submission, not compositor or screen readability. An owner observation record must name the diagnostic marker/attempt and actual window observation separately, preserving any remaining screenshot-to-frame gap. No freeze, hidden retry or scheduling workaround may be described as a cure. Enabled capture may affect scheduling and does not prove timing parity. One materially changed capture would still require its own source/guard/readback review and designated changed-source qualification; the current unchanged native workload must not be rerun. Root's qualified `as_chunks::<6>().0` glyph iterator is retained exactly. All original assertions, paid state, core anti-replay, handoff and capture guards remain unchanged.

## Opt-in cross-frame and pixel capture v2

This is changed causal evidence collection for the actual `6301a8d` black review, not a renderer cure. The v1 first-attempt recorder and all old guards remain. With `MERIDIAN_REVIEW_CAPTURE=1`, v2 assigns a monotonic review-attempt ID to each actual admitted review draw and keeps the most recent four attempts in fixed storage. Each has the real Frame stamp/UI revision, labels/clips/hits, geometry digest/counts, surface data, encoded ranges and normal API outcomes. `App::complete_presentation` records the expected Frame identity and the actual `Docked::presented` result read from the painted frame; accepted=false is retained rather than relabelled success. This is an application acknowledgement, not proof of display.

Repeated draws write no log lines. Review change/exit, draw error and Renderer drop flush the fixed ring with its overwrite count. Capture-only F8, otherwise unused by this path, flushes the latest acknowledged history and requests the next matching review draw. A pending request cannot cross a changed review, and a second request is refused while one is pending/in flight. Three observation requests per process are allowed. The requested attempt's complete metadata/API records and subsequent App acknowledgement are emitted immediately with its ID, preventing later ring eviction from losing the artifact's source identity. The marker explicitly refers to the next draw, not an earlier screenshot or already-displayed frame.

The original eight-transition, twenty-record and 4096-byte v1 bounds remain. V2 has an additional process-wide 1536-record limit at 4096 bytes per line; exhaustion is explicitly marked and later boundaries become unobservable. Each latest entry is bounded to twenty records, including acknowledgement when space remains. Oldest entries are overwritten in a four-entry ring with an explicit count; this is constant storage, not a time/FPS/memory sampler. A requested draw that skips/fails is retained as unobservable and is not retried. F8 is a diagnostic binding only when capture is enabled and the current real review is admitted; disabled capture follows the prior normal control and surface-configuration path.

Actual-target copies also require an existing explicit `MERIDIAN_REVIEW_CAPTURE_DIR`. At initialization the implementation queries real surface capabilities only for enabled capture. COPY_SRC is added to the existing configuration only when that directory exists, capability allows it and format is Bgra8Unorm/Srgb or Rgba8Unorm/Srgb. The actual acquired texture's usage and format are checked again. Other formats/usages, missing directory, zero/overflow dimensions or padded targets over 16 MiB produce explicit unobservable records. No alternate target, synthetic review, surface-format change, shader or pipeline is used.

For an admitted F8 request, the normal encoder copies the actual final colour texture after overlay and before submit/present into one MAP_READ/COPY_DST buffer. Rows are padded to wgpu's copy alignment with checked arithmetic. Only one such buffer is in flight. Mapping is requested asynchronously after the same normal submission; a nonblocking Poll is used only while the buffer is pending. The callback sends its real result to a bounded single-result channel; it performs no file work or wait. Missing result is not readiness, and failed/disconnected callback cannot become success. On actual map success, the owner thread reads the mapped buffer, removes only row padding, unmaps it and writes a new `review-request-N-frame-ID.pixels` file with create-new semantics and a durability flush. Existing/partial files are never overwritten or retried. The artifact record binds request/attempt/review, actual dimensions, padded and packed row sizes, format/channel order, byte count and diagnostic byte digest. At most three artifacts of at most 16 MiB each can be created. Incomplete shutdown explicitly retains the unavailable result without blocking for the GPU.

Artifacts are packed top-to-bottom native RGBA/BGRA8 rows, not PNG or screen captures; retain their metadata. A successful map establishes copied target readability for that exact named command sequence. It does not prove the compositor displayed it, establish readable screen text or identify a prior CUA screenshot's frame. Native window observations remain separate records tied to the F8 marker and reported state. Enabled copy/log/file work can affect scheduling; no timing parity or cure is inferred. No freezing, automatic native retry or change to core/save/equipment actions is included.

New source guards cover disabled v2 silence; fixed ring/overwrite/exit behavior with one v1 first output; three next-frame requests and review mismatch; finite output/exhaustion; actual paid public-Frame acknowledgement identity; driver usage/format admission; padding/unpack/overflow limits including the actual 2560x1440 target; callback-channel missing/failure handling; and artifact identity/create-new preservation. Tests use CPU/channel/file seams only and do not fabricate a GPU completion. All new Rust/compiler/Clippy/native results are **UNRUN** in this author lane. A complete exact-source packet preserves the original three actual first-attempt records, normal-exit receipt, source/API contracts, previous black failure, all assertions and Root's `as_chunks::<6>().0` correction.

After one independent changed-source review and designated compiler qualification, the owning native lane may execute only the necessary one changed workload. Configure a fresh existing artifact directory and capture environment in the signed process receipt. On the real retained stored paid Beam, request F8 while the prerequisite Fit is visible (or black); confirm only a visibly valid review. If visible and confirmed, request F8 on visible Store before confirming it, then perform Enter/immediate Y and request F8 on the paired Fit, without blind confirmation if black. This fits the three-request budget. Retain actual marker/window observations, source/binary/environment identity, latest-frame/App acknowledgement records, actual map results/artifact bytes and normal terminal/save receipts. If an earlier step is black, unsupported, incomplete or otherwise blocks progress, preserve that result and stop instead of manufacturing the pair or restarting to evade a bound. Original `6301a8d` remains FAIL; no authority to run comes from authoring this implementation.

## Post-render presentation and window-observation boundary

The one signed `ad5b9f9da259c85389cd662af2473e543e26ab11` native run showed prerequisite Fit and Store, then Enter/immediate Y produced the paired black Fit through supported CUA. F8 requested the actual next targets at review-attempt frames 1939, 6076 and 9753. Each real map callback returned Ok, and each actual App decision accepted the matching painted Frame stamp/UI revision. Root observed the paired window still black after the third copy. No invisible confirmation or retry followed. Escape recovered Equipment with the same paid Beam stored, and Root saved/quit normally. These observations remain distinct from image/frame identity claims.

Independent offline inspection confirms all three artifacts are byte-identical, each 14,745,600 bytes at 2560x1440 BGRA8, SHA256 `6e9e739d3b1d2064cd63cdee178664d7f3e0feadfe0ea3ab5d274ceaeaf7615d`. Counts are 3,670,113 opaque black pixels, 7,852 opaque white, 6,139 opaque cyan and 2,296 opaque yellow: 16,287 nonblack pixels and alpha 255 throughout. This rules out a fully black or zero-alpha value in those copied targets only; it does not measure a native layer/window alpha or composited screen. The 201,195-byte stderr log contains all three copy/map/artifact/accepted-ack identities, with no recorded asynchronous error, false acknowledgement or output exhaustion. Absence of reported error is not proof that presentation completed.

The current application owns one Window/Surface. `resumed` returns if Renderer already exists, and `window_event` filters by that WindowId. Store/Fit do not create, hide, replace or adjust window transparency. The application changes fullscreen only through its existing F11 control and reconfigures resize only through existing lifecycle handling. The per-frame title call reaches native setTitle; the pinned macOS `pre_present_notify` implementation is a no-op. Drawing writes the acquired target, schedules normal rendering/copy and calls present before App acknowledgement. No operation-specific code or source-supported renderer defect was identified at these boundaries.

The pinned wgpu Metal backend obtains a CAMetalDrawable from the layer and retains its texture. Its `present` implementation creates an additional internal Metal command buffer, schedules the drawable there and commits it, or uses the existing transaction branch. The application's mapped copy belongs to the earlier render/copy command sequence. Its readiness neither observes that distinct presentation command's status nor reports a drawable-presented event. Public `SurfaceTexture::present` returns no native drawable/window/layer identity or display-completion result. Its native HAL escape APIs are unsafe, while this repository forbids application unsafe code. This analysis does not bypass that restriction or fabricate a presented callback from a void call.

The exact remaining gap is copied acquired target to actual drawable presentation, then native NSWindow/CAMetalLayer/WindowServer image to the supported CUA observation. Current attempt IDs/stamps are application identities, not native drawable/layer/window IDs. The pinned winit macOS WindowId is the NSWindow object pointer cast to usize, not the WindowServer CGWindowID/windowNumber; its numeric value must not be passed directly to an independent window-capture API. Existing CUA images are retained in Root's conversation, without an independently bound OS-composited artifact. The data do not yet distinguish a surface/drawable presentation issue, native window/layer binding or visibility issue, and an observation-path mismatch. None is asserted as the cause.

A meaningful changed diagnostic must correlate the existing F8 attempt/target identity with the actual native window and presentation boundary, rather than capture the same render target again. The minimum evidence is the owning process plus stable native window identifier and bounds/scale/visible/minimized/focus/occlusion state, observed native layer/drawable identity and actual presentation-command/drawable-presented result if supported. Missing native results remain unobservable; queue-copy completion cannot substitute. Any platform observer or safe adapter requires its own reviewed source and finite budgets. Do not acquire an extra drawable, alter present mode, add a wait, change surface/layout/alpha, freeze the app or inject a backend workaround to manufacture a result.

The complementary observation is a bounded independent OS-composited capture of that same identified window, and where supported a display-region capture containing it, bracketed by the existing observation markers. Preserve capture API/permission/result/window identity/bounds and artifact hashes separately from the GPU artifact and CUA image. A CUA-only black image is not automatically physical-display proof; an independent OS image is not automatically proof of what a person saw either. If the independent image shows the review while CUA shows black, investigate the observation path. If it is black while the named GPU target contains the review, the missing evidence remains presentation/window/compositor state. Do not assert exact atomic screenshot-to-drawable identity without an actual supported association.

This is a concrete next-observation requirement, not implemented native instrumentation or permission for another launch. A separately reviewed materially changed diagnostic in the existing owning lane is necessary before another workload; rerunning ad5 with the same F8 target copies would be unchanged evidence. Current source/test guards remain byte-identical and all 728 attributes are retained. Root's current 295-fresh/433-carried per-platform qualification remains bound to its exact source. No renderer fix, native acceptance or public preview update is claimed.

## Independent macOS window observer preparation

The concrete observer consists of `tools/capture_meridian_window.py`, `tools/meridian_window_capture.swift` and `tools/test_capture_meridian_window.py`. It is execution-pending material for the exact missing independent window-image boundary. It is not a renderer fix or a replacement for safe native drawable-presented telemetry, which remains **UNOBSERVABLE**. The qualified ad5 game/runtime and all 728 Rust attributes remain unchanged.

The Python policy accepts only bundle `io.xyzzy.meridian.review-v2.ad5b9f9`, source `ad5b9f9da259c85389cd662af2473e543e26ab11` and the qualified signed executable SHA256 from the owning launch receipt. A scope binds the exact live PID, normal window mode, existing local evidence directory, native provider executable hash and CUA-reviewed global point bounds. A window must be the unique active/on-screen layer-zero window for that PID/bundle, fully on one display without covering a display, and match the reviewed bounds within 0.5 point. It must have a real nonzero UInt32 CGWindowID from `SCWindow.windowID`; pointer IDs are rejected. Ambiguity, missing owner, bounds/visibility/fullscreen mismatch or changed identity fails without fallback.

The native provider uses only supported ScreenCaptureKit metadata and `SCContentFilter(desktopIndependentWindow:)` with `SCScreenshotManager.captureImage`. It requires macOS 14.2 or newer and keeps AppKit/ScreenCaptureKit objects on the main actor. Before any metadata enumeration it checks the recorded exception reference and existing capture permission via preflight, then validates the live PID/bundle. It never requests permission, invokes a picker, changes system settings, sends keyboard/mouse/CGEvent/AppleScript input, activates the app, or captures a display/other application. It serializes only target-owned window metadata. The selected ID/owner/bounds are checked again before and after the one image request. A failed/unsupported call stops; no alternate renderer or capture API is attempted.

Each observation requires an existing unique F8 request/frame assignment, matching actual App acknowledgement and named successful GPU-map result in the bounded native stderr snapshot. That binds context, not a screenshot's atomic drawable identity. Python and native uptime brackets are retained with their clock names. The observer captures only the selected window, omits cursor, shadows and child windows, and records the returned image dimensions and metadata. It does not silently rescale a mismatched result. PNG/raw images are capped at 16 MiB, owned-window metadata at sixteen entries, responses at 24 MiB and each native invocation at eight seconds. There is one enumeration call plus one capture call, with native pre/post revalidation, and no retry.

Three request slots are allowed in one immutable scope/evidence directory. Claims, receipts and PNGs use exclusive creation. Duplicate slots, changed scope, existing destinations, prior failure/incompletion or non-increasing frame context stop before another capture. The F8 stderr must be the native-stderr.log beside the same owning launch receipt, preventing stale logs from another run directory. The Python publisher checks window identity again, PNG structure/CRC, dimensions, byte count and hash. Only validated images are published. Failed/partial evidence is not overwritten or retried. The transient in-flight lock is the only helper-owned file removed; claims and result evidence persist. There are no profile/save accesses or network calls.

Sixteen offline Python fixtures cover exact selection, ambiguous/missing/foreign/hidden/fullscreen windows, bounds drift, winit-style pointer IDs, pending exception, launch identity and F8 marker mismatch, three-slot/exclusive publication, native identity drift, negative native status, timeout/no fallback, PNG/hash/dimension corruption and forbidden native APIs. They use a fake provider and synthetic PNGs, not an OS screenshot or a simulated GPU completion. The focused fixture result is PASS; the Swift provider is **UNCOMPILED/UNRUN**, and no CoreGraphics/AppKit/ScreenCaptureKit enumeration or capture has executed. This is not a full Python-suite or new native-pass claim.

G1 may perform compile-only Swift 6/SDK checks in its existing lane without running the helper. Before actual execution, Root must present the exact reviewed source/provider hash and narrow scope and obtain the explicit human exception required by the CUA tool rule for alternate UI technology. None has been granted. A `human-approved:` correlation field is not a programmatic grant; it must reference the genuine external approval. The default scope template is intentionally non-runnable. Existing OS capture permission is separately unobserved; denial must stay an explicit failure with no permission request or settings change.

Only after that exception and a new owning diagnostic binding may Root authorize the one necessary changed identity workload. Bind PID and bounds from the one approved Meridian instance and supported CUA metadata, never by guessing an object pointer or querying another app. Keep actual GPU artifacts, independent window PNGs, CUA observations and any remaining compositor/presentation uncertainty separate. If the helper fails, stop without fallback, recapture, system changes or a new unapproved run. The external native drawable-presented result remains unavailable rather than fabricated.

The observer now records provider and capture attempts before invocation. If final receipt publication or cleanup fails after a capture attempt, the CLI reports an unknown capture outcome and exits nonzero, retaining claims and image evidence. Three new offline regressions retain the original thirteen fixture bodies and cover these late failures, preflight rejection and enumeration-only failure. No retry, fallback or native execution is introduced.
