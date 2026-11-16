# Meridian

[![CI](https://github.com/stephenlclarke/meridian/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/stephenlclarke/meridian/actions/workflows/ci.yml)
[![License: proprietary/source-visible](https://img.shields.io/badge/license-proprietary%20%2F%20source--visible-orange)](LICENSE)
[![Rust 1.99](https://img.shields.io/badge/Rust-1.99.0-93450a?logo=rust)](rust-toolchain.toml)
[![wgpu 29](https://img.shields.io/badge/wgpu-29.0.4-blue)](crates/meridian-desktop/Cargo.toml)
[![Build targets](https://img.shields.io/badge/targets-macOS%20%7C%20Windows%20%7C%20Linux-blue)](.github/workflows/ci.yml)

[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=alert_status)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=coverage)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Security](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=security_rating)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Reliability](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=reliability_rating)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Maintainability](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=sqale_rating)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Bugs](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=bugs)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Vulnerabilities](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=vulnerabilities)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Code Smells](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=code_smells)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Technical Debt](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=sqale_index)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Duplicated Lines](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=duplicated_lines_density)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Lines of Code](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=ncloc)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)

Meridian is a fresh Rust reimplementation of the existing Elite/L337 game. It reuses selected assets and intentional mechanics through a traceable import and parity process. Graphics use wgpu, as requested by Stephen on 15 November 2026. The existing Swift application continues its separate delivery and fleet work.

Status: the signed local MBP Economy Preview at main `edaa29f` contains seventeen-commodity trading, flight, launch/docking, Avenor/Renas travel, original G02 freight and Save/Continue. Its actual packaged Metal journey passed: eight identified handovers, once-only 730 CR/trusted clue, seven-delivery and completed cold Continue. Mac and Dell validation passed 126 top-level Rust tests and 37 importer tests on the applicable source; main CI and exact-main SonarQube passed. The installed preview is functional local delivery, not full release qualification. Its original in-flight leak scan reported 416 objects/19,936 bytes; a separate instrumented run did not reproduce it and does not establish a cure. Compact resize, sustained minimize/focus-loss, performance/race evidence, the notarization account agreement and a dependency notice remain open. The paid equipment slice below is authored source awaiting independent review and owned execution; earlier results do not qualify it. The public repository is [stephenlclarke/meridian](https://github.com/stephenlclarke/meridian), cloned locally at `~/github/meridian`.

## Start here

- [Architecture](docs/ARCHITECTURE.md): ownership, module boundaries, graphics, persistence and failure handling.
- [First playable milestone](docs/FIRST-PLAYABLE.md): one complete journey and its acceptance criteria.
- [Legacy milestone breakdown](docs/LEGACY-MILESTONES.md): G3 source-grounded roadmap from the first playable to full mission and fleet parity.
- [Legacy reuse](docs/LEGACY-REUSE.md): exact reference revision, content provenance and mechanics parity.
- [Paid cargo bay content](docs/PAID-CARGO-BAY-CONTENT.md): one original SH025 equipment contract, provenance and reference fixtures for the next joined equipment slice.
- [Development and delivery](docs/DEVELOPMENT.md): machine roles, checks and release evidence.
- [Decision: Rust, wgpu and ownership](docs/decisions/0001-rust-wgpu-ownership.md).

Read [implementation status](docs/IMPLEMENTATION-STATUS.md) before running the prototype. When the execution lane is available, use `make check`, `make test`, `make clippy` and `make run`. Click inside the game window to give it keyboard focus. The station shell uses L to launch while docked, W/S for thrust/deceleration, Up/Down for pitch, A/D for yaw, Q/E for roll, Space for brake, P for pause, F11 for fullscreen and Escape to exit. Speed is displayed in reference gameplay units, not a qualified physical calibration.

Run `make docs-check` to lint the documents and `make assets-check` for headless Python import checks. The legacy milestone breakdown was prepared by the existing G3 session from immutable source. The core packaged journey has passed; the remaining platform, memory, performance and distribution gates keep full first-playable acceptance open.

## Integrated freight, trading and continuation

N starts a new commander; C continues the save. L launches; J requests the other journey system; T selects the current station contact; R reviews; Enter confirms one reviewed action; B cancels; F buys missing fuel. M opens the market; [ and ] select among the seventeen original commodities; G/H buy or sell one native unit of the selection using the same real account and finite stock. Food is selected initially. The market shows actual held/reserved quantities and native units; kg/g goods use no hold tonnes. These economy controls are part of the delivered Economy Preview. The HUD uses original code-drawn glyphs, without an imported font or texture. See [the World/HUD packet](docs/PR-world-flight-hud.md).

F5 saves, F9 reloads, and Escape or normal window close saves before exit. An existing save requires a second N to confirm a new commander; the old file remains until the new game is explicitly saved or normally quit. Invalid saves remain untouched without fallback or repair. The host holds an OS lease and bounds reads; the core validates state, content bindings and producer accounting before replacement. Save failure keeps the window and World open. See [the save/continue packet](docs/PR-save-title-continue.md).

Tested source `9aa927670eeb9990ab9c2a075475339ba2a80553` is merged as `df2b1301369b90272df1c58917b6303df8efeefd`. Both Dell targets pass 75 core and 27 desktop tests, including separate-process seven-to-eight receiving, once-only settlement, completed continuation and file custody/failure checks. Windows builds the desktop executable; it remains unlaunched. [Main CI 43765301032](https://github.com/stephenlclarke/meridian/actions/runs/43765301032) passes, including exact-main SonarQube. These results do not establish actual Metal rendering, held input, native window-close or packaged full-journey acceptance.

## Paid equipment slice

O opens the equipment panel. K purchases and fits the original Large Cargo Bay for 2,800 CR at an eligible station. U stores the owned bay; Y refits it. Equipment actions require the panel to be open. The bay adds 35 tonnes while fitted, bringing SH025's hold from 92 to 127 tonnes. Storing keeps the same paid item and refuses when all ordinary and mission cargo would not fit in 92 tonnes. Fitting and storing do not debit or refund credits. The panel and market show actual current hold and mount usage.

This slice uses save format 3 and refuses older formats without migration, repair or fallback. Keep the delivered Economy Preview and its profiles separate until this candidate is qualified. The equipment code and controls are not yet a delivered playable update. See the [combined slice](docs/PR-paid-equipment.md) and [original content contract](docs/PAID-CARGO-BAY-CONTENT.md).

## Ordinary impact source slice

The authored core predecessor records actual front Pulse station impacts, original legal windows and permanent T02 failure, with version 7 persistence and exact version 4-6 readers. The player damage reducer is prepared; finite missile launch/motion and its actual saved ordnance remain the successor. The joined desktop shows the failed attempt and accepted station-strike incident. Coordinated compilation, tests and native qualification are pending. This is source work, not an installed preview update. See [ordinary impact rules and limits](docs/ORDINARY-IMPACTS.md).

## License and quality checks

Meridian's new code is proprietary/source-visible under [LICENSE](LICENSE). The public repository permits inspection under GitHub's terms; it is not a permissive open-source grant. Separately licensed original L337 assets retain their applicable notices. Only verified original project-authored assets are eligible for reuse; original Elite/Oolite or other third-party art, models, textures, audio and fonts are excluded.

GitHub Actions checks formatting, workspace compilation, tests and strict Clippy on Linux, Windows and macOS. It also runs the asset/import tests and documentation lint, and produces Rust LCOV, Clippy JSON and Python coverage reports. SonarQube Cloud analysis publishes only from committed `main`, uses the exact commit SHA as the analysis version, verifies the effective Previous version new-code policy and waits for the quality gate. Badges show the service's measured status; passing headless checks do not imply native GPU or full playable acceptance.

## Freight dialogue and explicit choices candidate

The next source slice adds V to review receiving all held mission units and X to review abandonment with Arlen. Enter confirms the displayed choice; B cancels. I shows current mission details. The review panel word-wraps original contract dialogue and warnings, joined with the existing mission owner. These controls are authored and unqualified until the complete artifact passes review and validation; see [the implementation packet](docs/PR-freight-dialogue-controls.md).
