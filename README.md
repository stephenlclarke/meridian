# Meridian

[![CI](https://github.com/stephenlclarke/meridian/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/stephenlclarke/meridian/actions/workflows/ci.yml)
[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=alert_status)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=coverage)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Bugs](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=bugs)](https://sonarcloud.io/project/issues?id=stephenlclarke_Meridian&types=BUG)
[![Vulnerabilities](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=vulnerabilities)](https://sonarcloud.io/project/issues?id=stephenlclarke_Meridian&types=VULNERABILITY)
[![Code smells](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=code_smells)](https://sonarcloud.io/project/issues?id=stephenlclarke_Meridian&types=CODE_SMELL)
[![Maintainability](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=sqale_rating)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Duplicated lines](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=duplicated_lines_density)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Lines of Code](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=ncloc)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Reliability Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=reliability_rating)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Security Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=security_rating)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Technical Debt](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=sqale_index)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)

[![Latest preview](https://img.shields.io/github/v/release/stephenlclarke/meridian?include_prereleases&label=preview)](https://github.com/stephenlclarke/meridian/releases)
[![Downloads](https://img.shields.io/github/downloads/stephenlclarke/meridian/total)](https://github.com/stephenlclarke/meridian/releases)
[![Open issues](https://img.shields.io/github/issues/stephenlclarke/meridian)](https://github.com/stephenlclarke/meridian/issues)
[![Open pull requests](https://img.shields.io/github/issues-pr/stephenlclarke/meridian)](https://github.com/stephenlclarke/meridian/pulls)
[![Last commit](https://img.shields.io/github/last-commit/stephenlclarke/meridian/main)](https://github.com/stephenlclarke/meridian/commits/main)

[![Rust 1.99](https://img.shields.io/badge/Rust-1.99.0-000000?logo=rust)](rust-toolchain.toml)
[![Preview platforms](https://img.shields.io/badge/preview%20platforms-macOS%20%7C%20Windows-blue)](docs/PREVIEW-RELEASES.md)
[![macOS signing](https://img.shields.io/badge/macOS%20signing-Developer%20ID-green)](docs/PREVIEW-RELEASES.md)
[![Notarization blocked](https://img.shields.io/badge/notarization-blocked-orange)](docs/IMPLEMENTATION-STATUS.md)
[![Source-visible license](https://img.shields.io/badge/license-proprietary%20%2F%20source--visible-blue)](LICENSE)

Meridian is a Rust reimplementation of the original L337 project, with direct wgpu graphics, one authoritative World and traceable original content. New code is [proprietary/source-visible](LICENSE); imported original content retains its own notices. Original Elite/Oolite assets are excluded.

## Paid missile source awaiting execution

The [paid standard missile core increment](docs/PAID-MISSILES.md) authors actual 180 CR purchase, nine-position SH025 fitting, same-item storage/refit and retained later launches with explicit v12 saves. Its new tests and runtime qualification are **UNRUN**. It is the first increment toward the required full modular equipment system; that broader system, docked screen integration and all-hull compatibility remain open. Published previews and their profiles remain unchanged.

## Current delivery

The signed local **Meridian Flight Lesson Preview** is installed from qualified gameplay main `a583b61` ([PR 31](https://github.com/stephenlclarke/meridian/pull/31)) and published as [v0.3.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.3.0-preview.1). It adds the [original flight-and-reception lesson](docs/FLIGHT-RECEPTION.md) and World-owned front/rear/left/right views. It retains flight, physical docking, Avenor/Renas travel, seventeen-commodity trading, original G02 freight, paid cargo equipment, front Pulse fire, practice beacons/reception, station impacts and three finite owned missiles. Save v10 retains the lesson, missile sessions and attempt history, with explicit v4-v9 readers.

Start the installed app with `~/Applications/Launch Meridian Flight Lesson Preview.command`. It uses its separate `Library/Application Support/Meridian Flight Lesson Preview/saves-v10` directory. Downloaded portable packages use the adjacent empty `saves-v10` directory. Keep earlier previews and their dedicated profiles separate.

F6 reviews lesson consent or reception, F12 reviews cancellation and C shows lesson details while playing. Keys 0/1/2/3 select front/rear/left/right views. Actual Mac qualification completed measured closing flight, swept return, separate zero-cash reception and first trace, cancellation/retry, four views and two cold Save/Quit/Continue pairs. A transient native pulse flash remains unobserved. The [safe-gunnery lifecycle](docs/SAFE-GUNNERY-ATTEMPTS.md) retains F7 closure, F10 fresh consent and I history without reward, refund or ammunition grants.

Mac, Linux and native Windows each passed 231 top-level Rust tests and 74 Python tests (54 importers plus 20 release tooling), strict Clippy and release builds on compiled source `152ac26`. Tagged main `a583b61` retains 136 of its 139 tracked blobs; only README, implementation status and the flight guide differ. These distinct trees do not imply a main rebuild. Exact-main CI and Sonar passed with zero open findings.

The installed signed app passed title/render/normal-close smoke without creating a commander save. Public downloads and the extracted Mac Developer ID signature were independently verified. Three published previews are retained. This remains a functional preview: full game acceptance, memory qualification, window/focus edges, frame timing/race evidence and commercial acceptance remain open. See [implementation status](docs/IMPLEMENTATION-STATUS.md) for exact scope and retained failures.

## Play

N starts a commander; C continues. A second N confirms replacing the current commander, and the old disk file remains until an explicit save or normal saved quit. Click the window for keyboard focus.

| Controls | Action |
| --- | --- |
| L; W/S; arrows; A/D; Q/E; Space | Launch; thrust; pitch; yaw; roll; brake |
| J; T; R; Enter; B; F | Travel; select contact; review; confirm; cancel; buy fuel |
| M; [ / ]; G/H | Market; commodity selection; buy/sell one native unit |
| O; K; U; Y | Equipment panel; buy/fit cargo bay; store; refit |
| I; V; X | Mission details; review all held freight deliveries; review abandonment |
| F1; F2; F3; F4 | Next practice lock; arm/safe; launch one missile; safe/unlock |
| F7; F8; F10 | Review attempt closure; reception; fresh consent |
| F6; F12; C; 0/1/2/3 | Flight lesson consent/reception; cancellation; details; view direction |
| P; F11; F5; F9 | Pause; fullscreen; save; reload (lesson and attempt history in v10) |
| Escape / window close | Save before exit; failure keeps the app open |

The freight journey requires eight identified one-tonne deliveries and awards 730 CR plus the trusted clue once. Markets use finite stock and the same account as mission and equipment transactions. Invalid saves refuse without migration, repair or fallback.

## Preview releases

[Download Flight Lesson Preview v0.3.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.3.0-preview.1) for macOS Apple silicon and Windows x64. Both public downloads were verified against their SHA256 checksums, and the downloaded Mac app passed extracted Developer ID signature verification. macOS is not notarized; Windows is unsigned and requires the Microsoft Visual C++ x64 runtime. Windows interactive gameplay remains unobserved. Versions come from Conventional Commits with immutable annotated tags; keep only the newest three published previews. See [preview releases](docs/PREVIEW-RELEASES.md).

## Build and contribute

The workspace pins Rust and dependencies in [rust-toolchain.toml](rust-toolchain.toml) and [Cargo.lock](Cargo.lock). Use `make ci` for documentation, formatting, importers, compilation, tests and strict Clippy. `make run` starts the desktop app in an owned graphics session. G1 owns Mac compilation/native validation; Dell owns Linux/native Windows validation; MBA retains its existing fleet work.

- [Development and repository maintenance](docs/DEVELOPMENT.md)
- [Architecture](docs/ARCHITECTURE.md) and [core API](crates/meridian-core/README.md)
- [First playable criteria](docs/FIRST-PLAYABLE.md) and [source-grounded roadmap](docs/LEGACY-MILESTONES.md)
- [Content provenance and reuse](docs/LEGACY-REUSE.md)
- [Finite missile behavior and save boundary](docs/FINITE-STARTER-MISSILES.md)

GitHub Actions validates Linux, Windows and macOS. Protected main requires pull requests, current required checks, linear history and resolved conversations; force pushes and deletion are blocked, including for administrators. SonarQube runs after merge from committed main with the exact commit SHA and Previous version policy. Historical change descriptions live in merged GitHub PRs and Git history rather than duplicate issue/PR Markdown files.

## Qualified source: navigation and trade

The original [T04 navigation-and-trade lesson](docs/NAVIGATION-TRADE.md) adds separate consent and current-fit/route/stock confirmation, one identified ordinary-market tonne, paid-fuel travel, destination sale and return for separate zero-cash reception. Save v11 retains the receipts with explicit v4-v10 readers. Keys 4/5 separate consent/reception and selected-commodity plan review; 6 cancels and 7 shows details. Actual buy/sale/refill commands publish the checked save before live commit.

Compiled source `bb5862b` passed whole-source review and bounded corrections. Mac, Linux and native Windows each passed 254 top-level Rust tests (192 core plus 62 desktop), 74 Python tests, 30 Markdown files, formatting, workspace checks, strict Clippy and locked release builds. Native v11 gameplay is unrun: the supported Mac UI adapter times out during app selection. Signed staging and observer preparation continue; protected integration and authoritative main Sonar remain pending. The installed and published v10 preview is unchanged.
