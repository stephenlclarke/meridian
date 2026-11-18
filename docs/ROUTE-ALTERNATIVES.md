# Original visible-scope route alternatives

## Actual unmet prerequisite and status

The existing M2 roadmap requires navigation-computer effects before new mission
families. `OriginalNavigation` currently exposes only a full-G01 `fewest_jumps`
BFS. Original `OriginalWorldNavigation.swift` also restricts routes to explicitly
visible systems and provides a Safer alternative scored by hops plus arrival
hostile exposure. Current core navigation data lacks the original exposure field.
That concrete read-only prerequisite is the scope of this increment.

The source starts at qualified `2d5f4a57ae05db68ec7f233f9b695f3ecc4b2f75`, tree
`16391c8e179353703b32ddb8d65e69f361f51dcd`. Eight Rust cases are authored **UNRUN**.
The three Python importer cases pass (0.002s), exact pinned re-import and source
checks pass, as do direct rustfmt and all 50 Markdown files. Compiler, Clippy,
native and gameplay acceptance remain unrun for this delta.
It is independent of the laser and ECM composition: no World/flight, damage,
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

## Implemented boundary

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

## Meaningful authored cases

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

## Root binding and acceptance

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
