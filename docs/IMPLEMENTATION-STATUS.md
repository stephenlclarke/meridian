# First implementation slice

Date: 15 November 2026.

## Authored

- Dependency-free meridian-core: one mutable World, explicit held inputs, fixed 1/60 tick, normalized xyzw quaternion, finite/range checks and transactional rejection. SH025 speed/acceleration/rotation parameters are source-derived reference units. Input signs, brake/coast behavior and tick integration are explicit prototype conventions awaiting trajectory parity.
- Separate scoped G02 freight domain model and meaningful headless tests. This does not validate real consent, receiver identity, cargo, paid resources, wallet publication, trusted receipts or saved state. It is not connected to the flight shell.
- meridian-desktop: winit window/input lifecycle, pause/fullscreen/quit, bounded catch-up, direct wgpu WGSL depth pass, reusable CPU/GPU vertex storage, resize/minimize and surface failure handling. Renderer only reads snapshots.
- Original SH025 display triangles and palette imported with source/output hashes and retained MIT notice. Navigation rings are prototype fixtures, not station geometry. The view is a chase camera; cockpit, HUD text rendering, UV textures and physical contacts are later work.
- Pinned Cargo dependencies/toolchain and Makefile workflows. No unsafe application code is permitted by the workspace lint.

## Validation

Python import tests passed: source mismatch refusal, committed mesh hash/size/first indexed vertex and retained notice. Re-importing the exact pinned TOML reproduces the committed mesh and manifest. Rust formatting parses/formats the authored source; this does not type-check it or validate WGSL. Cargo.lock was resolved offline from existing registry metadata without compilation.

The joined slice now contains 19 authored Rust tests (12 core and 7 desktop-side tests). The pause regression covers a resume after 600 seconds without paused redraws; host pause also resets timing and requests one status repaint. These tests are authored, not executed.

Cargo check, Rust tests, clippy, actual GPU launch, frame inspection and performance are UNRUN. G1 retains sole compiler/native/GPU ownership while the current Swift 548-case qualification and updated app take priority. The source is ready for that validation when a safe execution slot exists; there is no playable or commercial acceptance claim.

## Next concrete acceptance

1. G1 validates the exact joined revision with make check, make test and make clippy when app delivery permits it; fix actual errors in the same slice.
2. Launch the wgpu prototype on an actual Mac. Confirm mesh, color/depth, controls, pause/focus/fullscreen/resize/minimize/quit and clean error reporting. Record the actual adapter/backend and image evidence.
3. Implement the real station/flight contact and admission loop plus source route/fuel/economy. Replace navigation fixtures with qualified content, not a dock flag shortcut.
4. Integrate the G02 finite source, consent/custody, receiver and failed-publication semantics; implement versioned save/quit/continue before claiming the complete first playable.

The authoritative milestone remains [FIRST-PLAYABLE.md](FIRST-PLAYABLE.md). Do not close it on a flight prototype, unit-test count or source review alone.
