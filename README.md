# Meridian

Background [asteroid-mining core source](docs/ASTEROID-MINING.md) is under development. Its paid-tool, chart/render and runtime acceptance joins remain pending; it is separate from the next HUD, docked-menu and modular-equipment preview.

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

The subsequent [physical asteroid renderer](docs/ASTEROID-RENDERER.md) uses the authoritative mining geometry and read-only chart/contact cues. Its actual work-input/publication hook and coherent native mining acceptance remain separate from the published cockpit preview.

## Modular mining-tool source awaiting execution

The [modular equipment increment](docs/MODULAR-EQUIPMENT.md) authors genuine paid front Mining Laser fitting, same-item Pulse storage/refit, shared resource preparation and a ship-resource readout that remains valid without Pulse fitted. Thirteen standalone and two Root-join cases are **UNRUN**. Actual mining/screen/profile/fleet composition and the full remaining modular catalogue are open. Published previews and profiles are unchanged.

## Paid missile source awaiting execution

The [paid standard missile core increment](docs/PAID-MISSILES.md) authors actual 180 CR purchase, nine-position SH025 fitting, same-item storage/refit and retained later launches with explicit current-format saves. Its new tests and runtime qualification are **UNRUN**. It is the first increment toward the required full modular equipment system; that broader system, docked screen integration and all-hull compatibility remain open. Published previews and their profiles remain unchanged.

## Current delivery

The signed local **Meridian Trade Lesson Preview** is published as [v0.4.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.4.0-preview.1), from qualified main `8da5d64` ([PR 33](https://github.com/stephenlclarke/meridian/pull/33)). It adds the [navigation and trade lesson](docs/NAVIGATION-TRADE.md): actual commodity purchase, fuel-debited journeys, destination sale, physical return and separate zero-cash reception. It retains flight, physical docking, seventeen-commodity trading, original G02 freight, cargo equipment, Pulse fire, finite missiles and the [flight lesson](docs/FLIGHT-RECEPTION.md). Published saves use v11 with explicit v4-v10 readers.

Start the installed app with `~/Applications/Launch Meridian Trade Lesson Preview.command`. It uses its separate `Library/Application Support/Meridian Trade Lesson Preview/saves-v11` directory. Downloaded packages use the adjacent `saves-v11` directory. Keep the app, launcher, notices and save folder together, and keep earlier previews and profiles separate. The macOS app is Developer ID signed but not notarized. After a blocked first launch, use **System Settings → Privacy & Security → Open Anyway**, as described in [Apple's instructions](https://support.apple.com/en-us/102445).

The qualified Food journey bought at 22.6 CR, sold at 19.6 CR and paid 6.8 CR for fuel: a real loss of 9.8 CR, with no tutorial payment. The complete journey belongs to compiled source `bb5862b`; the later `c3a71e9` correction separately passed the current invalidated-plan hint smoke. Three held/sold/completed cold Save/Quit/Continue pairs and four real writer refusals were audited. Current corrected Mac and native Windows qualification passed 254 Rust tests, 74 Python tests, strict Clippy and release builds. The earlier Linux full-journey source passed the same test counts; no corrected-source Linux run is implied.

Tagged main `8da5d64` has 145 of its 148 tracked code/test/dependency/asset files byte-identical to compiled `c3a71e9`; only three Markdown files differ. Exact-main CI and Sonar passed with zero open issues. The separate installed title/render/normal-close smoke passed without creating a commander. Both public downloads, full payload inventories, the extracted Mac signature and annotated tag were independently verified. Only the newest three previews are retained.

This is a functional preview. The next HUD/menu/modular-equipment candidate is still under development and is not part of this published binary. Full game, fleet, missions, memory, window/focus, frame timing/race, notarization and commercial acceptance remain open. Windows is unsigned and requires the Visual C++ x64 runtime; native interactive Windows gameplay is unobserved. See [implementation status](docs/IMPLEMENTATION-STATUS.md).

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
| P; F11; F5; F9 | Pause; fullscreen; save; reload (published lesson history in v11) |
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

Compiled source `bb5862b` passed whole-source review and bounded corrections. Mac, Linux and native Windows each passed 254 top-level Rust tests (192 core plus 62 desktop), 74 Python tests, 30 Markdown files, formatting, workspace checks, strict Clippy and locked release builds. The signed native Mac T04 journey passed: actual Food purchase, physical Renas entry, same-tonne sale, paid refuelling, physical return and separate zero-cash reception, with held/sold/completed cold Continue pairs. The trip lost 98 tenths through ordinary costs. Root verified the sealed evidence. Current correction `c3a71e9` passed Mac and native Windows qualification plus all required hosted checks; its signed Mac hint smoke confirms trade controls 4/5/6 and unchanged save/live facts. The fixture helper uses a per-process sequence to keep temporary paths distinct. Protected main `8da5d64` subsequently passed exact-main CI/Sonar and shipped as v0.4.0-preview.1 with separate v11 profiles. The next development candidate is not yet a qualified release.

The cockpit instrument implementation and its current validation limits are described in [Cockpit instruments](docs/COCKPIT-HUD.md).

The next playable preview is being developed around a working cockpit HUD, docked galaxy/system/showroom/trading/equipment screens and real modular equipment. The current integration candidate is unqualified; authored source and focused owner tests are not a new playable release. Mining/salvage, EMP, bounty hunting and the other ship systems precede new mission delivery.

In the next-preview development candidate, `/` toggles controls help and ordinary flight uses a compact overlay above the cockpit. This change is not in the published v0.4 binary.

The [planet and sun implementation](docs/CELESTIAL-SHADING.md) binds core-owned bodies and procedural surfaces, with swept flight safety and explicit native acceptance limits.

The next preview is under development: cockpit HUD, five docked pages and durable modular equipment controls are composed in source. This is not a new playable release. See [docked preview integration](docs/DOCKED-PREVIEW.md) for controls, exact scope and pending native acceptance.
