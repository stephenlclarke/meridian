# Implementation and validation status

Date: 15 November 2026.

## Current station slice

[PR 1](https://github.com/stephenlclarke/meridian/pull/1) is merged as `7aee6b6d80d7c9ac32c52148a2134cc26d6ef62b`, with the exact reviewed `830b731` tree. World now owns the explicit docked start, tick-bound L launch, flight and automatic physical entrance admission. The desktop renders the original SH028 display mesh, uses an exterior docked observer and a pilot camera near the entrance, and preserves contact notices across catch-up ticks. Conservative contact stops at the last clear pose; original damage, impulse, sliding and full display-surface collision parity are not claimed.

At Rust source `502fa654a9aa94e4c47b9b34bcd84c65b055c448`, both Dell Linux and native Windows pass workspace/all-target checks, all 46 Rust tests (35 core and 11 desktop), and strict Clippy. Windows builds the station executable with SHA-256 `d00e3c712eda8e68c1d54c34aa46895fbd211abfd2bc1e8765d27f765a5414f6`. The later Python-only `830b731` correction leaves Rust, Cargo and assets byte-identical. Nine final Python importer tests and exact immutable-source re-import pass; formatting and all 26 Markdown files pass lint. Independent whole-source and security-delta reviews are clear, and the final PR SonarCloud check passes. The initial source-location re-import failure and initial Sonar security failure remain historical failures, resolved by locating the exact pinned blobs and constraining the importer command respectively.

The station executable has not started. Windows interactive access is locked/inaccessible; no new GPU backend, scene, L-launch, window behavior or player-controlled docking result is inferred from headless tests. The earlier verified flight preview remains separate and available. Current station native UI acceptance remains open until interactive access returns. No new actual-game journey, legacy hull, mission or commercial release acceptance is claimed.

The sections below retain the earlier flight slice and its exact evidence. Navigation/travel/fuel and connected G02 freight are separate active source slices; their genuine World location, contact, owned fit, sole account and publication joins must be complete before full-journey qualification.

## Initial flight source

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

## Native Windows flight verification

Source 4b55126d02d7c5a9781ef76aba7bbcf041a8fa65, tree 937c172d9043fc2dfc197fb8742248a106c94d97, passes Windows workspace check, all 33 tests, strict Clippy and the native desktop build. The actual executable SHA-256 is e14baa27e885238cdf0e44a08690f4bb618aa7da361aab8f7da0b0babb7eff34. Root independently verified its running Windows session, process/image identity and clean source checkout after launch.

wgpu selected Intel Iris Xe / Vulkan, driver 101.7077. The first native run produced an actual coloured SH025/ring scene with limited depth evidence, but paused fullscreen left a stale frame. The correction requests repaint after window changes and surface recovery without advancing a paused World. Stephen confirmed the corrected scene is visible and, after clicking the window for focus, all requested held flight controls, brake, pause/resume, paused fullscreen/resize, minimize/restore and held-key focus release work. The initial all-keys report resolved after focusing the window. This is human verification of the actual native run, not an inference from the passing tests.

Corrected automated captures were white despite the human-visible scene. Keep that capture limitation and the original failed frames explicit; do not manufacture corrected screenshots. Escape exited the first run cleanly; the corrected app was intentionally left open, so normal window-close was not newly executed. Actual Mac GPU launch, performance, complete gameplay, docking, receiving and save acceptance remain open. The human authorized Dell work in parallel; G1 retains the Mac execution lane for the current Swift app correction and delivery.

## Next concrete acceptance

1. Preserve the exact passing Dell evidence; G1 still needs Mac compiler/backend qualification when app delivery permits it. Fix actual failures within the same slice.
2. Qualify the prototype on an actual Mac when its execution lane is available. Preserve the distinct Windows human verification and automated capture limits; measure performance separately.
3. Qualify the integrated station contact/admission loop in the native UI, then complete source route/fuel/economy and actual World mission joins. Keep physical admission separate from descriptive snapshots.
4. Integrate the G02 finite source, consent/custody, receiver and failed-publication semantics; implement versioned save/quit/continue before claiming the complete first playable.

The authoritative milestone remains [FIRST-PLAYABLE.md](FIRST-PLAYABLE.md). Do not close it on a flight prototype, unit-test count or source review alone.


## Joined World freight and desktop HUD candidate

The candidate documented in [the source packet](PR-world-flight-hud.md) joins frozen G3 World freight and MC1 navigation to actual desktop commands, with a live code-drawn wgpu overlay. The host allocates a new ephemeral commander identity; the core keeps ownership and authority. No font, texture or other art file was added. Existing original L337 navigation bytes have their separate authoring audit; the reduced SH028 collision-mesh lineage gap remains explicit.

Root ran source formatting, whitespace checks and 21 Python importer tests successfully. All joined Rust compilation/tests/Clippy and GPU/input acceptance are UNRUN. Newly authored desktop tests exercise explicit contact and review confirmation, cancellation, pause invalidation, bounded HUD geometry and the full public command journey. Those authored tests are not passing evidence. The HUD states that this build has no save, so no continuation or complete first-playable result is claimed.
