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

The joined slice contains 33 authored Rust tests: the original 12 core and 7 desktop-side tests, plus 14 independently source-reviewed docking geometry tests. The pause regression covers a resume after 600 seconds without paused redraws; host pause also resets timing and requests one status repaint. Execution results remain separate from source review.

The first Dell Linux workspace check at caab7db failed with E0599 because the pinned wgpu instance descriptor has no Default implementation. At corrected 78cfaad, the Linux workspace all-target check and all 33 Rust tests passed, while Clippy failed on three mesh-decoder lints. Both failures are retained separately from the corrected results.

Source revision ec2fd1ef931342b69614c2acdd0fbafccf77bf3c, tree 4ac2381728efbde089aa05fde202705cf5e856d7, passes workspace all-target check, all 33 tests (26 core and 7 desktop) and Clippy with warnings denied on both Dell targets: x86_64-unknown-linux-gnu under Ubuntu 22.04.5 LTS in WSL2, and native x86_64-pc-windows-msvc. Both use Rust 1.99.0. A separate native Windows Rust executable also linked, ran and passed a file write/read check with MSVC 19.44 and Windows SDK 10.0.26100.0. Later documentation commits reuse this exact unchanged source evidence; they are not separate native test runs.

Actual GPU launch, frame inspection and performance remain UNRUN. The human authorized Dell Linux and native Windows builds in parallel; G1 retains the Mac execution lane for the Swift 548 baseline cases plus the new regression and updated app. There is no playable or commercial acceptance claim.

## Next concrete acceptance

1. Preserve the exact passing Dell evidence; G1 still needs Mac compiler/backend qualification when app delivery permits it. Fix actual failures within the same slice.
2. Launch the wgpu prototype on an actual Mac. Confirm mesh, color/depth, controls, pause/focus/fullscreen/resize/minimize/quit and clean error reporting. Record the actual adapter/backend and image evidence.
3. Implement the real station/flight contact and admission loop plus source route/fuel/economy. Replace navigation fixtures with qualified content, not a dock flag shortcut.
4. Integrate the G02 finite source, consent/custody, receiver and failed-publication semantics; implement versioned save/quit/continue before claiming the complete first playable.

The authoritative milestone remains [FIRST-PLAYABLE.md](FIRST-PLAYABLE.md). Do not close it on a flight prototype, unit-test count or source review alone.
