# Physical asteroid renderer

This renderer increment is based on Root mining commit `e3aedb1a65d54d249af0c783968e1cc58135948d`. It renders the genuine immutable `World::mining_belt_snapshot()` with the same `ASTEROID_FACES` and world-space vertices used by mining contact and swept collision. No second placement generator, ore ledger, target capability, tool grant or catalogue model is introduced. The preserved earlier author worktree remains unchanged.

## Geometry and cues

The twelve current bodies produce 288 vertices. Every physical face is emitted through the existing station/chase/pilot camera and existing flight-view rotation. The reused GPU buffer reserves those vertices plus twelve possible mining-flash vertices. Existing depth testing and projection apply. Depletion changes the rock colour, never its location, size, faces or collision surface. Facet lighting uses the actual celestial scene sun position; an absent light retains ambient shading.

The flight cue re-resolves the selected chart rock address against the current World snapshot. It reports that body's native commodity unit, remaining/initial quantity, player-relative range, bearing and elevation. Without a selected rock it may describe an actual front contact. The belt-centre sentinel and foreign/unavailable addresses do not create a rock. A front-contact label is descriptive and does not establish mining eligibility.

## Admitted-shot integration

`MiningVisual { opening, shot }` accepts the actual opening `Snapshot` and successful `MiningShot`; it cannot perform work. The beam starts at the recorded player position and ends at the committed contact point. A five-tick visual lifetime, matching source/current system and visit, active flight and no travel are required. A mismatched commodity, zero extraction or nonfinite contact refuses. The renderer never replays work or changes a save.

Root still owns work input and durable transaction publication. The minimal App renderer adapter currently passes `mining_visual: None`. At the actual successful work publication boundary, retain `MiningVisual` from the opening World snapshot and returned committed shot, then pass that optional value through `SceneEffects`. Clear it with input/profile/focus reset and when no longer actively flying; do not synthesize a successful flash from a selected chart address, refusal or failed save. The existing Pulse and missile paths remain intact. This packet does not claim that the work trigger or durable mining journey is playable.

## Verification and limits

Focused evidence accompanies the frozen delivery outside the source tree. The initial compile failure exposed the commodity error enum's lack of `Display`; the renderer now uses its supported debug description. Focused renderer and existing mesh tests exercise exact physical faces/camera transform, read-only save invariance, lighting/depletion, native units, invalid system/geometry/depletion, partial-output rollback, empty docked presentation, shot age/visit/finite-contact refusal and exact chart address resolution. Tests using modified immutable snapshots or descriptive shot fixtures are renderer tests, not paid-tool admission evidence.

Desktop strict Clippy is checked separately from the current Root core's `validate_accounting` argument-count finding. That workspace finding is not hidden or waived. This is a renderer source delivery with focused execution evidence, not a full CI/native/game acceptance or public release. The reviewed public cockpit preview and its profiles remain unchanged.

Before playable mining acceptance, Root must join the actual work-input/publication hook and qualify the coherent source: navigate manually to a chart-selected physical rock; compare chart/cue/render/contact positions; buy and fit a real Mining Laser; observe only admitted work flashes, resource use and native ore transfer; deplete a body while preserving its solid geometry; check Pulse obstruction and ordinary collision; sell ore and verify cash; then Save/Quit/cold Continue and return to verify finite depletion, module identity, cargo and resources. Check all flight views and ordinary compact/full-screen rendering at that checkpoint. Never infer those results from these renderer tests.
