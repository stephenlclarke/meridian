# Original visible-scope route alternatives

## Pure planner prerequisite (retained predecessor)

The existing M2 roadmap requires navigation-computer effects before new mission
families. `OriginalNavigation` currently exposes only a full-G01 `fewest_jumps`
BFS. Original `OriginalWorldNavigation.swift` also restricts routes to explicitly
visible systems and provides a Safer alternative scored by hops plus arrival
hostile exposure. The qualified baseline lacked the original exposure field.
That concrete read-only prerequisite was the scope of the retained pure increment.

The pure predecessor starts at qualified `2d5f4a57ae05db68ec7f233f9b695f3ecc4b2f75`, tree
`16391c8e179353703b32ddb8d65e69f361f51dcd`. Eight Rust cases are authored **UNRUN**.
The three Python importer cases pass (0.002s), exact pinned re-import and source
checks pass, as do direct rustfmt and all 50 Markdown files. Compiler, Clippy,
native and gameplay acceptance remain unrun for this delta.
The pure predecessor is independent of the laser and ECM composition: no World/flight, damage,
modular equipment, save, App, desktop, workspace or lock changes are made. The
sealed laser branch and packets retain their own identities.

## Original mechanics and data

[Source pins](ROUTE-ALTERNATIVES-SOURCES.json) identify original revision
`a11fe16924d87d06021816f9037952afcbac08db`, tree
`68fec574df12ddf74acb14747e0dd6839f478642`. The original world-route provider
accepts one galaxy and at most 256 distinct visible systems; endpoints must be
visible. Every hop obeys current reach at the origin and full-tank reach at later
stops. The stable search chooses the lowest original system ID for equal cost,
updates only strictly cheaper paths and validates every eligible score before an
early destination exit. Fewest-jumps weight is 1; Safer weight is
`1 + destination.hostileFraction`. Identical alternatives merge their labels.
Distance, first-leg distance and summed arrival exposure are descriptive results.

`tools/import_route_exposure.py` reuses the existing fixed-blob reader and pins the
same immutable 2048-record original catalogue SHA. It extracts only the 256 G01
`hostileFraction` values into `assets/legacy/g01.route-exposure`, with ordered IDs,
finite 0-1 scalars and an exact output manifest. The 2570-byte companion does not
change the qualified coordinate bytes. The existing original MIT notice remains.
No new ship, body, encounter, market or foreign-game asset is imported.

## Pure planner boundary

`navigation::OriginalNavigation::route_alternatives(from, to, visible,
first_reach, full_tank_reach)` returns typed `RouteAlternative` values or a typed
`RouteError`. `RouteKind` labels distinguish Fewest jumps and Safer. Only the
existing G01 catalogue is supported; unknown systems and cross-galaxy scopes
refuse. Duplicate caller IDs have set semantics and never expand the bounded
internal scope. Computed distance/score matrices contain at most 256 squared
entries. The kernel rejects nonfinite/nonpositive weights, malformed matrices
and cost overflow, including invalid edges beyond an early destination.

A disconnected valid scope returns no alternatives. A same-origin route contains
one system, both labels, zero distance/exposure and no later-refuelling assumption.
All original coordinate types, existing `fewest_jumps` behavior and existing test
bodies remain unchanged. There is no new saved field or content migration.

This API grants no visibility, known-system status, installed navigation grade,
fuel, refuelling, market availability, route installation, jump, station or arrival
authority. The returned explanation explicitly says later fuel, payment, stock
and launch are not guaranteed. Safer means the original aggregate score; it does
not promise a safe encounter or substitute government IDs for authored exposure.

## Pure planner authored cases

Eight Rust cases cover direct-route label merging and real arrival exposure;
a genuine G01 alternative taking an extra hop for lower exposure; first-leg
fuel versus later-tank reach; unchanged selected legacy fewest results; visible
scope, duplicate IDs, hidden/unknown/cross-galaxy/excessive refusal; invalid reach
and same origin; stable ties and a weighted counterexample; validation before
early exit and overflow; and corrupted exposure header/IDs/scalars/truncation.

An independent heap-based calculation over the pinned source freezes Avenor to
G01-S003 at reach 9 LY: Fewest is `[7,164,107,56,138,209,3]`, six jumps and 2.4
exposure; Safer is `[7,164,146,253,35,102,85,3]`, seven jumps and 0.9 exposure.
Distances are 46.19213094965041 and 51.44493420766694 LY. This is source arithmetic,
not execution of the Rust implementation. Three Python importer cases exercise
identity refusal, exact committed output and malformed structural inputs; their
synthetic parser fixtures patch the source hash only within the test.

## Pure planner binding and acceptance

Root may compose this bounded navigation module independently of shared
laser/ECM files. Preserve the exact current `navigation.rs` declarations and the
existing provider body. No whole-file overwrite of later owner work is needed.
The actual later navigation screen must supply genuinely visible IDs and reach
from the real fitted grade, current fuel and full tank. Render both returned
labels, distance, exposure and the refuelling explanation. Refresh on visibility,
fit, fuel or origin changes. Planning is user-invoked, not a per-frame calculation.
Use current World commands for every later leg and physical arrival; a route row
cannot approve a jump or mint a refuel. Trade/profit alternatives, route custody,
UI installation and additional galaxies remain separate unmet work.

After one complete Root/G2 source review, Dell/G1 run the appropriate pinned
core/all-target tests and strict Clippy through existing owned qualification.
Retain exact head/tree, commands, raw logs and timings. The 400-case baseline core
has 408 source attributes here, with two compatibility, 226 desktop and three
observer cases unchanged; these are not passing counts for this source. Actual
UI and native acceptance require the later host binding and delivered binary.
This prerequisite does not complete navigation parity, a mission or a preview.

## Installed-route host successor (unqualified)

The successor starts at exact pure planner `f26614695adc505210e63b1b6ac67e3e2170fa74`,
tree `0e3e48d09ac25afa6e4c5da556b4e480c85ee81b`. Its eight planner tests and
provider remain unchanged. Five additional core cases and four desktop cases are
authored **UNRUN**. This candidate has 413 core, two compatibility, 230 desktop
and three observer test source attributes; those counts are not passing results.
Compiler, Cargo, Clippy, Rust tests, release, native/GPU and actual journey
acceptance remain unrun in this source-only lane.

Original `Plugh.swift` establishes that ordinary systems are chart-known without
a visit grant. Only synthetic PLUGH (system 256) uses the per-galaxy discovery
mask, granted by opening actual main-station rumours at system 7. Current G01
contains only ordinary records 0 through 255; it has no admitted PLUGH record,
arrival/service provider or station-rumour command. `known_navigation_systems`
therefore exposes ordinary G01 charts without marking a visit or granting hidden
knowledge. Hidden port, other galaxies and unsupported arrivals refuse. Hidden
port discovery parity remains blocked by those concrete provider prerequisites;
there is no inert discovery flag or claim that all worlds were discovered.

Visibility and executable arrival authority remain separate. Route installation
uses only existing main stations Avenor, Renas and Lareria. It reads the real
owned fitted navigation item, current World reach, current first-leg fuel and
full-tank later reach. A private World-bound review captures the canonical pure
planner result and actual berth, item, account revision, tick and session epoch.
Confirmation revalidates them. A foreign or restored World cannot reuse its
review. The actual profile writer publishes candidate bytes before live commit;
a write failure preserves the plan, fuel, cash and prior file.

Galaxy/System O cycles Direct, Fewest and Safer; R reviews the current selection
and Enter installs it. C clears and saves the installed route. Direct mode keeps
its existing current-fuel/direct-range refusal. The review shows actual path,
hops, distance, arrival exposure, first-leg cost and the original later-refuelling
warning. In flight J requests the stored next leg through the existing World
jump command; N clears and saves the route. HUD and Help describe the same
controls, phase and real fuel requirement. A continued route takes precedence
over an old host menu destination; explicit clearing also clears that cache.

Progress derives from retained completed jump receipts and real World visits.
Arrival alone leaves AwaitingDock. Physical docking at the expected next stop
advances the bookmark; the final physical berth completes it. Successful manual
off-route jumps cancel it, and a busy clear refuses rather than cancelling or
refunding the spool. Route planning/install/clear never supply fuel, cash, cargo,
a launch, a jump or a berth. An insufficient-fuel next leg must use existing
paid refuelling and actual flight commands. Current installation is station-only;
original in-flight installation/spool cancellation is deliberately not claimed.
Trade/profit routes, additional arrivals and complete navigation parity remain
unimplemented.

The strict current-content marker and original exposure bytes bind this new
saved route state. Codec validation rechecks installation identity, bounded path,
actual historical fitted item and jump/berth prefix against retained owners.
There is no migration or compatible-reader claim. Other original resource and
jump receipts remain unchanged. One old route-selection test intentionally
changes its obsolete ephemeral-bookmark expectation: it now checks unchanged
simulation/cash plus disk equality and cold route identity. Its destination and
visible instruction assertions remain; no failure is hidden by that update.

The new core cases cover ordinary/hidden/cross-galaxy admission, real two-leg
stock flight and physical berths, paid refuelling, scoped/stale reviews, failed
publication, loss of fitted computer, real off-route cancellation, busy clear
and malformed path/item/owner/length refusal with atomic restoration. The main
journey uses genuine public G02 consent as a protected pilot and retains held
cargo; it is not acceptance of a new mission. The separate manual-jump case
labels its public XYZZY fitting setup and switches unlimited fuel off. Desktop
cases exercise actual chart search/keys, presented confirmation, the real save
writer failure, fresh leased Profile Continue, installed-route J/N authority and
Help bounds. Every fresh Profile open releases the previous exclusive lease.

Root must serially compose the shared World, Commerce, codec, App and Help
fragments with later laser/ECM ownership; this base predates that composition.
Preserve Root's ECM World-fork field and mounted-laser receipt correction. Recheck
combined Help bounds, all World literals and codec order. The separate laser
host lease correction is not part of this navigation delta. After complete
Root/G2 source review, existing G1/Dell owners run their pinned compiler, tests,
strict Clippy and platform gates, then actual input/journey/cold-Continue checks.
Retain exact joined head/tree, commands, raw failures, counts and timings. No new
review wake, compiler helper, preview or publication follows from this packet.
