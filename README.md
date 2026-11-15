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

Status: the original SH028 station launch/docking slice is integrated in [PR 1](https://github.com/stephenlclarke/meridian/pull/1). Linux and native Windows pass workspace checks, all 46 Rust tests and strict Clippy. All nine final Python importer tests and exact original-source re-import pass; independent source review and the final PR SonarCloud check are clear. The new station executable has not started: the Dell interactive desktop is locked, so native station graphics, L-launch and player-controlled docking remain unverified. The previous Windows flight preview remains separately human-verified. Connected navigation/freight, economy and durable saves are still being implemented; the complete first playable is unfinished. The private GitHub repository is [stephenlclarke/meridian](https://github.com/stephenlclarke/meridian), cloned locally at `~/github/meridian`.

## Start here

- [Architecture](docs/ARCHITECTURE.md): ownership, module boundaries, graphics, persistence and failure handling.
- [First playable milestone](docs/FIRST-PLAYABLE.md): one complete journey and its acceptance criteria.
- [Legacy milestone breakdown](docs/LEGACY-MILESTONES.md): G3 source-grounded roadmap from the first playable to full mission and fleet parity.
- [Legacy reuse](docs/LEGACY-REUSE.md): exact reference revision, content provenance and mechanics parity.
- [Development and delivery](docs/DEVELOPMENT.md): machine roles, checks and release evidence.
- [Decision: Rust, wgpu and ownership](docs/decisions/0001-rust-wgpu-ownership.md).

Read [implementation status](docs/IMPLEMENTATION-STATUS.md) before running the prototype. When the execution lane is available, use `make check`, `make test`, `make clippy` and `make run`. Click inside the game window to give it keyboard focus. The station shell uses L to launch while docked, W/S for thrust/deceleration, Up/Down for pitch, A/D for yaw, Q/E for roll, Space for brake, P for pause, F11 for fullscreen and Escape to exit. Speed is displayed in reference gameplay units, not a qualified physical calibration.

Run `make docs-check` to lint the documents and `make assets-check` for headless Python import checks. The legacy milestone breakdown was prepared by the existing G3 session from immutable source. The full journey, native docking acceptance, economy, mission authority, saves and a packaged playable app remain in progress; the first milestone is not complete.

## License and quality checks

Meridian's new code is proprietary/source-visible under [LICENSE](LICENSE). The public repository permits inspection under GitHub's terms; it is not a permissive open-source grant. Separately licensed original L337 assets retain their applicable notices. Only verified original project-authored assets are eligible for reuse; original Elite/Oolite or other third-party art, models, textures, audio and fonts are excluded.

GitHub Actions checks formatting, workspace compilation, tests and strict Clippy on Linux, Windows and macOS. It also runs the asset/import tests and documentation lint, and produces Rust LCOV, Clippy JSON and Python coverage reports. SonarQube Cloud analysis publishes only from committed `main`, uses the exact commit SHA as the analysis version, verifies the effective Previous version new-code policy and waits for the quality gate. Badges show the service's measured status; passing headless checks do not imply native GPU or full playable acceptance.
