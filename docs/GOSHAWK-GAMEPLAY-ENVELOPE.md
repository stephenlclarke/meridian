# SH008 Goshawk Explorer envelope candidate

Goshawk Explorer is the next unresolved original ship in catalogue order after Mule. This slice binds the original SH008 detailed model, original native candidate, and R3 contract. It prepares a hull-specific gameplay envelope for review and qualification. Ordinary placement, player acquisition, paid effects and crew custody still need their own source authority.

The staged forest-silver display resource keeps its original 3,484 triangles, winding, materials, texture identities and UVs. Its positions use the authorized 30 m detailed length calibration of 10000000/1028273 metres per source unit. The original 964-triangle native candidate uses the same coordinate convention and exact rational calibration. It remains unqualified. The staged resource publishes only a detailed model. Preserve that distinction and the original 512-face exact qualification limit.

Run the importer with Python assertions enabled. Both generation and verification refuse optimized Python before opening any assets. The source-import regression rebuilds all nine outputs in a temporary directory and compares them byte for byte with the shipped files. It also checks that each altered original is refused, and that verification detects a damaged triangle file without writing over it.

The candidate encloses both submitted point sets with 69 local convex supports plus a 2 cm spherical offset. The partition keeps the wishbone rails, drive arms, crossbraces, pressure body, cockpit and engine assemblies distinct. Each pressure well has its own piece, with its local recess struts where present. The broader structures represented in both inputs can share a piece. Each original index-connected component stays in one piece, and each original triangle is enclosed by convexity. This is an approximate gameplay envelope. Native exact geometry and combined surface fit require separate evidence.

The authored clearance test carries a small physical triangle through ten metres of the original forebody channel and requires strict separation. It also checks a real left drive-arm point blocks contact. A source-only scalar check finds at least 0.746 m of separating-plane slack for the channel fixture across all 69 rounded supports. That result applies to the selected fixture. The four core tests passed on Mac, Linux and Windows. The normal-size Mac Showroom also showed the open channel. The surface-fit audit below gives bounds against both originals. GPU cost and ordinary World docking remain open. Mule's local native-well decision applies only to Mule. SH008 has no approved local alterations or stand-off exceptions.

## Meridian Showroom

At a station berth, open Ship Showroom with 3 and search for Goshawk Explorer. The preview submits all 3,484 calibrated detailed triangles in their original order with the original forest-silver material assignments and repeated texture coordinates. The same project-authored diffuse maps already bundled for SH001 are reused by source identity. [Visual provenance](../assets/fleet/sh008/VISUAL-PROVENANCE.json)

The preview rotates with the existing Showroom controls. It draws the opaque detailed hull and a separate close grazing shimmer from the 69 rounded pieces in the geometry packet. The union shades at most one exposed surface per ray and reaches zero opacity at 64 m. The preview uses fixed illumination. The controlled World uses the adopted stock mass and inertia. Ordinary placement, mass-dependent flight, cockpit operation, gameplay docking and acquisition still need acceptance. The native Mac Showroom displayed the Goshawk in Compare and single-ship views. Rotation exposed the opening between its arms. The normal-size view passed. GPU draw cost remains open.

## Metal shimmer evidence

The Mac owner's offscreen visual check runs the production Showroom renderer at a fixed 2× scale and keeps the exact preview pose, material, projection and depth for each pair. It compares the opaque hull against the ordinary shimmer, then repeats the ordinary draw and changes only the shader phase in a separate sample. The independent verifier checks the front, yaw-oblique and elevated hull-local directions and the raw pixel pairs. Each shimmer phase stays within 24 sRGB codes per channel of the opaque hull, and the two phases stay within seven codes of each other. The verifier can write amplified difference images. Inspect the upper-view captures for the open wishbone.

The visual check and its verifier run independently of timing. When Metal exposes timestamp queries, a separate check measures one shimmer draw in an isolated render pass after the opaque depth is established. The raw timestamp pairs include the attachment load and store for that pass. The diagnostic writes the raw pairs and period before checking their order across all three poses. It requires a single valid queue timestamp period and rejects invalid Metal counters. On the M5 Pro, the baseline and trailing-render runs both returned overlapping intervals and a zero final end tick for every pose. Both timing runs failed validation. GPU draw cost remains unmeasured.

No new full-fleet hull or public-preview count follows from this source packet.

The offscreen M5 Pro run at `1311359` passed independent pixel verification at all three poses. The maximum differences from the opaque hull across the two phases were 15, 22 and 17 sRGB codes per channel for the front, oblique and upper views respectively. Advancing only the animation phase changed each channel by at most two codes in every view. Same-phase repeats matched byte for byte.

## Minimum Showroom size

The minimum-window regression drives the station Showroom's search, rotation and Compare controls at 640×360 logical. It projects all 10,452 original Goshawk face vertices through the production camera for front, oblique and elevated views in both modes. It requires at least two logical pixels of clearance inside each Goshawk pane and checks that the model panes stay apart and clear of the station controls.

The Mac owner's optional minimum-size check renders the production docked model pass into a 1,280×720 offscreen target at 2× scale. It captures the complete target for solo and combined Compare, then draws each Compare model separately from the same frame and pane. The independent checker measures both models’ own gutters and verifies that their isolated pixels reproduce the combined Compare image.

The Mac run at `b17edd2` passed the projection check and the twelve-image check. The smallest Goshawk gutter was 6.23 logical pixels in the elevated view. Each combined Compare frame exactly matched the two isolated ships. Native window resizing and compositor output remain unobserved.

## Original station docking candidate

The SH008 station helpers use the same strict rounded support checks as Mule. The actual SH028 aperture controls fit, and the original station triangles control continuous clearance. The exterior launch starts from Goshawk's own aft support. The entry check keeps the existing crossing, tunnel, attitude, speed and roll thresholds. It accepts a physically clear aligned crossing in the authored case and requires refusal at the actual speed and roll boundaries.

The physical lip fixture uses original Goshawk rail vertex 6 and SH028 triangle 482. Source arithmetic places the rail point strictly inside the lip triangle during the proposed two-metre sweep while the actor centre remains outside the opening. All four helper cases passed on Mac, Linux and native Windows.

## Controlled World approach

A dedicated World starts an inbound SH008 outside the SH028 opening with a stationary SH025 observer. Each accepted default input advances one fixed tick. The SH008 keeps its initial orientation and commanded starting speed until the approach ends. Each proposed translation checks the actual station mesh and the observer's native triangles. An enclosing-radius test skips the observer mesh only when the bodies are strictly separated for the entire tick. Crossing the original port plane additionally requires strict aperture fit, inbound attitude, and speed below 144 m/s. Contact or entry refusal leaves the actor at the previous clear pose. Successful entry stops at the first admitted pose across the plane.

Each snapshot shares the same immutable 69 pieces used by collision and carries its World scope and tick. The controlled World starts without a commander, a player ship, or any ordinary fleet placement. The four preceding core/SH028 cases passed the hosted Linux and Mac lanes and the owned Mac workspace. The ordinary Mac Showroom displays SH008. The controlled release-build Mac run at `8f8fff2` passed all five routes. Aligned entry took 70 ticks. Overspeed and off-aperture entry were refused at ticks 31 and 70. Station lip and observer contact occurred at ticks 20 and 58 respectively.

Run the approach regression in a release build to measure collision outside the simulation. Its five routes cover aligned entry, overspeed, the independent SH028 lip witness, a clear sweep outside the aperture, and contact with SH025.

```bash
cargo test --locked --release -p meridian-core \
  flight::goshawk_validation::tests::goshawk_world_approach_reports_physical_contact_and_entry_clearance \
  -- --exact --nocapture
```

Record each `GOSHAWK_WORLD_COLLISION_COST` line with the tested commit, machine and runtime. The test samples `World::step` through each route's terminal tick and reports the tick count, median, p95, maximum, and terminal nanoseconds. Scenario setup, snapshots, assertions, and reporting occur outside the timed interval.

## Full-surface fit evidence

The audit measures directed Euclidean distance from the exposed rounded union
to each calibrated source triangle set separately. The nominal 20 mm rounding
expands the convex supports. The gap to an original concave surface
depends on the convex fill and on differences between the originals.

| Original | Certified exposed lower bound | Largest exposed sample | Certified full-surface upper bound | Worst sampled region |
| --- | ---: | ---: | ---: | --- |
| Detailed | 558 mm | 559.27 mm | 699 mm | Ends of native-candidate body wells 0 and 1 |
| Native candidate | 1,098 mm | 1,099.73 mm | 1,105 mm | Aft corners of detailed body wells 0 and 3 |

Excess above the 20 mm nominal padding is therefore bounded by 538–679 mm
against detailed and 1,078–1,085 mm against the native candidate. The upper
bounds also cover hidden member surfaces, rounded edges, and corners. The
29,057 guarded exposed samples cover 68 of the 69 pieces. The cockpit
crossmember has no retained sample and remains unresolved for exposure.
Every piece, including the crossmember, has a full rounded-surface upper
bound. The exact lower certificates reconstruct the two largest exposed
samples for each mesh and check every triangle of the compared mesh.
[Per-piece results and witnesses](../integration/sh008-goshawk/SURFACE-FIT.json)

The accepted relative forebody-channel sweep has at least 1.467 m of certified
clearance from all pieces. Both aft body-well mouths measured about 1.733 by
3.330 m with a 0.311 m recess depth. The source representations place those
mouth centers 1.567 m apart. The current envelope covers both mouth centers.
The detailed mouth center is over 130 mm from every detailed triangle and
over 72 mm from every native triangle. The native mouth center is over
310 mm from every native triangle and over 34 mm from every detailed
triangle. The detailed left arm control has a 1.412 by 1.633 m mouth and
0.311 m recess depth. Its covered mouth center is over 130 mm from every
detailed triangle and over 48 mm from every native triangle.

Two mandatory detailed rim vertices, 1288 and 1397, are more than 1,082 mm
and 1,081 mm from every native triangle. Native vertices 326 and 315 are
each more than 318 mm from every detailed triangle. Any envelope enclosing
both original triangle sets must retain those point locations. Changing the
convex partition can remove empty space in a well mouth, but closing the
cross-source discrepancy requires selecting or revising an authoritative
source.

The chosen shield bridges all four measured service-well mouths with the
existing 69 rounded pieces. The channels between the forebody arms remain
open. The collision regression uses independent calibrated coordinates for
the two aft body wells and two left arm wells. A 1 mm probe starts clear on
each original approach and is blocked at the mouth and during the complete
sweep through the gameplay collision API.

The 91-piece comparison trial retained every original triangle and would
admit spherical probes to those mouth centers. The four certified straight
routes support probe diameters of at least 106.716, 30.782, 59.596 and
15.256 mm respectively. The selected 69-piece shield blocks a point probe
on each of the same routes. Both original triangle sets and the forest-silver
Showroom asset are preserved.

The source-only audit can be repeated without a simulator or asset import.
Use NumPy 2.5.5, SciPy 1.18.1, trimesh 5.2.1, and rtree 1.4.1.

```bash
python3 -B tools/measure_goshawk_surface_fit.py
```

The audit independently checks the encoded triangle assignments. Exact
arithmetic validates the radial coverage of the triangulated convex pieces.
Dyadic patch witnesses and outward error allowances bound every rounded
piece surface against each original. Floating-point nearest-face searches
nominate candidate witnesses. The certificates check their geometric bounds
independently. This is a source-only audit.

## Adopted stock engineering

The Goshawk Explorer's stock dry mass is **33,719 kg**. The two luminous
recesses are included in the existing drive mechanisms' mass. The selected
centre of mass is approximately `[-0.000013111, 0.311774780, -2.362604393]`
metres from the calibrated hull origin. The full symmetric XYZ inertia
tensor is expressed about that centre in kg·m².

The controlled SH028 World publishes the adopted stock mass properties from
the same authenticated SH008 hull that supplies its collision geometry.
The certification snapshot makes the hull-local tensor available and
transforms the centre of mass into station coordinates. The approach
uses the source hull frame for contact and port fit. The mass properties
are stock data for this controlled path. Ordinary placement and
mass-dependent flight remain separate qualification.

The regression checks the exact mass, three centre coordinates and all nine
tensor entries against the independent MC1 baseline, then checks the centre
in the World before and after a fixed tick. [Retained MC1 proposal](../integration/sh008-goshawk/SH008-STOCK-ENGINEERING-PROPOSAL.json)
