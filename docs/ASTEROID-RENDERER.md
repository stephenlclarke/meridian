# Physical asteroid renderer

The renderer descends from the physical asteroid implementation in
`b38aa1b1283288f8c68afb8bed495f633db760c5`. It adapts the MC1 renderer proposal
to the staged M02 `World::mining_belt_snapshot()` and desktop presentation.

## Geometry and work cues

The desktop captures one immutable belt snapshot for the flight frame. The
renderer projects the core's `ASTEROID_FACES` and world-space vertices with
the same camera and flight-view rotation as the rest of the scene. The frame
buffer reserves up to 288 belt vertices and 12 work-flash vertices. The
existing celestial sun supplies facet lighting when present.

Partial work leaves all triangles and native ore quantities intact. When the
snapshot reports zero remaining work energy, the spent rock contributes zero
triangles. The core retains its identity and release history. Pending ore
records are descriptive and contribute zero geometry.

The HUD can resolve a chart address to a body in the current snapshot and
display native commodity quantities, remaining work, range, bearing and
elevation. The current App presents the snapshot's actual front contact. A
chart selection, once supplied, can describe a depleted rock by its retained
identity.

## Binding an accepted mining shot

`MiningVisual` combines the opening World snapshot with a successful
`MiningShot`. The flash uses the recorded opening position and contact
point, and lasts five simulation ticks in the same active flight visit.
Partial work must report zero newly released units. Terminal work must match
the spent body's native reserve in the same belt snapshot.

The App uses the fitted tool to choose mining or ordinary fire. After a successful mining command it passes the opening snapshot and committed shot to the renderer. Input reset and departure from active flight clear the visual. The HUD uses real front contact. The current chart selects a system and has no asteroid body address. The renderer reads snapshots without changing the World or save.

## Verification

The focused renderer tests cover physical triangles and camera transforms,
partial and terminal work, native quantities, front contact and chart address
resolution, invalid snapshots, flash lifetime and endpoint, output rollback,
lighting and save invariance. Work-state variants and shot descriptions in the
renderer tests are fixtures. The tests await execution on the joined source.

On the joined build, compare the physical rock and HUD cues across the flight
views in compact and full-screen windows. Check that each accepted partial
shot retains the mesh, reduces the displayed work and ends at the recorded
contact. Check that terminal work removes only the spent rock and that a cold Continue
retains the spent state. The chart body-selection case needs its own input binding.
