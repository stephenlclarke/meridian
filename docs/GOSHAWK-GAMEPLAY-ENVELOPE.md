# SH008 Goshawk Explorer envelope candidate

Goshawk Explorer is the next unresolved original ship in catalogue order after Mule. This slice binds the original SH008 detailed model, original native candidate, and R3 contract. It prepares a hull-specific gameplay envelope for review and qualification. Ordinary placement, engineering mass and inertia, player acquisition, paid effects and crew custody still need their own source authority.

The staged forest-silver display resource keeps its original 3,484 triangles, winding, materials, texture identities and UVs. Its positions use the authorized 30 m detailed length calibration of 10000000/1028273 metres per source unit. The original 964-triangle native candidate uses the same coordinate convention and exact rational calibration. It remains unqualified. The staged resource publishes only a detailed model. Preserve that distinction and the original 512-face exact qualification limit.

The candidate encloses both submitted point sets with 69 local convex supports plus a 2 cm spherical offset. The partition keeps the wishbone rails, drive arms, crossbraces, pressure body, cockpit and engine assemblies distinct. Each pressure well has its own piece, with its local recess struts where present. The broader structures represented in both inputs can share a piece. Each original index-connected component stays in one piece, and each original triangle is enclosed by convexity. This is an approximate gameplay envelope. Native exact geometry and combined surface fit require separate evidence.

The authored clearance test carries a small physical triangle through ten metres of the original forebody channel and requires strict separation. It also checks a real left drive-arm point blocks contact. A source-only scalar check finds at least 0.746 m of separating-plane slack for the channel fixture across all 69 rounded supports. That result applies to the selected fixture. The core tests, detailed distance samples, native appearance, real docking and renderer cost still need owned execution. Mule's local native-well decision applies only to Mule. SH008 has no approved local alterations or stand-off exceptions.

## Meridian Showroom

At a station berth, open Ship Showroom with 3 and search for Goshawk Explorer. The preview submits all 3,484 calibrated detailed triangles in their original order with the original forest-silver material assignments and repeated texture coordinates. The same project-authored diffuse maps already bundled for SH001 are reused by source identity. [Visual provenance](../assets/fleet/sh008/VISUAL-PROVENANCE.json)

The preview rotates with the existing Showroom controls. It draws the opaque detailed hull and a separate close grazing shimmer from the 69 rounded pieces in the geometry packet. The union shades at most one exposed surface per ray and reaches zero opacity at 64 m. The preview uses fixed illumination. The authored World placement, physical mass and inertia, cockpit, docking and acquisition still need their own source and acceptance. Native viewing and render cost are pending.

No new full-fleet hull or public-preview count follows from this source packet.

## Original station docking candidate

The SH008 station helpers use the same strict rounded support checks as Mule. The actual SH028 aperture controls fit, and the original station triangles control continuous clearance. The exterior launch starts from Goshawk's own aft support. The entry check keeps the existing crossing, tunnel, attitude, speed and roll thresholds. It accepts a physically clear aligned crossing in the authored case and requires refusal at the actual speed and roll boundaries.

The physical lip fixture uses original Goshawk rail vertex 6 and SH028 triangle 482. Source arithmetic places the rail point strictly inside the lip triangle during the proposed two-metre sweep while the actor centre remains outside the opening. The positive, negative and launch cases call the actual station source helpers.

## Controlled World approach

A dedicated World starts an inbound SH008 outside the SH028 opening with a stationary SH025 observer. Each accepted default input advances one fixed tick. The SH008 keeps its initial orientation and commanded starting speed until the approach ends. Each proposed translation checks the actual station mesh and the observer's native triangles. An enclosing-radius test skips the observer mesh only when the bodies are strictly separated for the entire tick. Crossing the original port plane additionally requires strict aperture fit, inbound attitude, and speed below 144 m/s. Contact or entry refusal leaves the actor at the previous clear pose. Successful entry stops at the first admitted pose across the plane.

Each snapshot shares the same immutable 69 pieces used by collision and carries its World scope and tick. The controlled World starts without a commander, a player ship, or any ordinary fleet placement. The four preceding core/SH028 cases passed the hosted Linux and Mac lanes and the owned Mac workspace. The ordinary Mac Showroom displays SH008. Qualification of the new World path is pending.

Run the approach regression in a release build to measure collision outside the simulation. Its five routes cover aligned entry, overspeed, the independent SH028 lip witness, a clear sweep outside the aperture, and contact with SH025.

```bash
cargo test --locked --release -p meridian-core \
  flight::goshawk_validation::tests::goshawk_world_approach_reports_physical_contact_and_entry_clearance \
  -- --exact --nocapture
```

Record each `GOSHAWK_WORLD_COLLISION_COST` line with the tested commit, machine and runtime. The test samples `World::step` through each route's terminal tick and reports the tick count, median, p95, maximum, and terminal nanoseconds. Scenario setup, snapshots, assertions, and reporting occur outside the timed interval.
