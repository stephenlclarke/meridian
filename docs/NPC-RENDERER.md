# SH025 traffic presentation

This is source awaiting combined review and execution, based on Root's `27b212f86350ef5040c12b2456851c62064b4306` NPC composition. Nine new geometry tests are authored **UNRUN**. Formatting is a source check, not compilation, a passing test or native acceptance. The published v0.5 preview is separate and unchanged.

The renderer borrows `World::traffic_snapshot()` through `SceneEffects::traffic`. Root owns the App join: retain the actual current-frame snapshot in a local value and pass its reference alongside the same player snapshot and current station frame. Root also owns traffic admission, Profile, observers, save codec and all shared World changes. This renderer packet alone is not a runnable or accepted composition. Do not activate invisible physical traffic in a shipping build.

## Bodies and scene frame

Actual trader and pirate actors identify SH025. Each uses the renderer's existing decoded `assets/legacy/sh025.mesh`, including every source triangle and color. The authoritative `RigidPose` supplies full orientation and station-space translation; there is no proxy primitive, uniform rescaling, role recoloring, screen-space marker or second model import. Admission and collision remain exclusively World decisions. Unsupported roles, hulls, foreign systems and excess actor counts refuse the traffic append instead of substituting geometry.

All phases except `Complete` remain visible, including `Destroyed`. Complete is the core's retained native-entry/berthed state; the renderer does not infer completion from distance, speed or health. A destroyed actor remains the same solid mesh at its retained pose. This does not invent debris, loot or a salvage grant.

Traffic shares the existing station/player camera, four flight-view transforms, homogeneous projection, near/far clipping and depth-tested flight pass. No CPU face removal or artificial distance cutoff drops segments. Camera-space vertices retain depth until the existing WGSL projection; station, hull and beam occlusion use that same depth buffer. Allocation is bounded by `MAX_TRAFFIC_ACTORS * actual_SH025_vertex_count + 12` additional vertices, with the checked existing frame-capacity gate before upload. The render buffer is reused; this source check does not establish measured FPS or memory behavior.

## Retained beam segments

The latest traffic beam renders only at world-tick ages zero through four. A future or older retained receipt does not replay. Its actual retained origin and contact point supply the segment; current victim movement, target labels and `direction * invented_range` never replace the endpoint. The existing Pulse ribbon and near-emitter flash implementation is shared, without manufacturing a `PulseShot` to render an NPC discharge. A matching real player Pulse projection is drawn once when its item, tick, origin and endpoint match the retained player traffic receipt. Other player discharges keep their existing path.

A beam must name a retained victim in the current system. Non-finite mesh or beam transforms refuse and roll back the complete traffic append. These are presentation guards; immutable descriptive snapshots do not create proof of a hit, damage or a spawned actor. The renderer never spends resources or updates simulation state. Only the latest retained beam is presented; this is not a claim to display every simultaneous discharge history entry.

## Verification and remaining integration

The nine authored tests cover public-World SH025 birth identities and all source triangles/colors, arbitrary pose rotation/translation, all ten phases, four cardinal flight views and actual projection depth, exact retained beam endpoints independent of current victim movement, tick lifetime and player-pulse identity, all unsupported populations/hulls, atomic non-finite rejection, and capacity bounds. Beam, phase and camera variants are explicitly descriptive geometry fixtures, not positive World damage, native entry or admission evidence. The public birth fixture supplies actual actor IDs without a spawn setter or synthetic `NpcId`.

Root must join App input and immutable snapshots, then obtain whole-composition independent review before the owning executor runs the focused tests and required CI. The new App field is deliberately a required composition dependency, not a default that silently hides actors. Actual native qualification must show the real hull poses and retained beams in supported views, correct depth/occlusion, player controls and save/quit/cold continuation on that exact reviewed binary. Missing NPC/player/ordnance/legal joins and all seven-role progression remain governed by [NPC traffic](NPC-TRAFFIC.md). No compiler, tests, Clippy, GPU/native run, new save or release was performed by this authoring packet.
