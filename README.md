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

## Current delivery

[Meridian Cockpit Services Preview v0.5.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.5.0-preview.1) is published for macOS arm64 and Windows x64 from protected main `07f1678` ([PR 37](https://github.com/stephenlclarke/meridian/pull/37)). It adds the flight HUD, galaxy, system, showroom, trading and equipment pages, keyboard and mouse navigation, presented transaction reviews, same-item modular storage/refitting and paid standard missiles. It retains the earlier flight, docking, finite markets, freight and lessons. This is the initial modular equipment increment; the full equipment catalogue and all-hull fitting remain open.

Start the installed Mac preview with `~/Applications/Meridian Cockpit Services Preview - 07f1678/Launch Meridian.command`. Downloaded packages use the adjacent launcher and isolated `saves-v13` directory. Keep the app, launcher, notices and save folder together. Keep earlier previews and profiles separate. The extracted Mac app is Developer ID signed but not notarized. After a blocked first launch, use **System Settings → Privacy & Security → Open Anyway**, as described in [Apple's instructions](https://support.apple.com/en-us/102445).

Mac, Dell Linux and native Windows each passed 372 top-level Rust tests and 88 Python tests, strict workspace Clippy and fresh locked release builds. Actual Mac qualification covered all five docked pages, keyboard/mouse, trade and modular reviews, refusal paths, nine missile indicators, flight views, failed-save preservation and Save/Quit/cold Continue. Main CI and exact-main Sonar passed with zero open findings. Compiled `beab907` and tagged main share 194 identical tracked files; two Markdown files differ. This is not a main rebuild or a repeated full journey.

All four public assets were downloaded anonymously and checked. The complete extracted Mac and Windows inventories and Mac signature were verified. The immutable annotated tag and release receipt are retained. The newest three previews are v0.5, v0.4 and v0.3. Windows is unsigned, requires the Visual C++ x64 runtime, and has no observed native interactive gameplay qualification.

Full game, fleet, missions, memory, window/focus, frame timing/race, notarization and commercial acceptance remain open. See [implementation status](docs/IMPLEMENTATION-STATUS.md).

## Following NPC development

The [finite traffic core](docs/NPC-TRAFFIC.md) is composed on the current preview source for further development. Sixteen core, three host and nine renderer cases remain authored **UNRUN**. Actual SH025 traders and pirates have finite stock, movement and contact logic; the actual SH025 renderer is joined in source, while reciprocal ordnance, player damage, legal proof and final profile/observer joins remain required before execution or release. Police, miners, bounty hunters, aliens and AI constructs retain explicit source and controller dependencies. No additional hull or NPC gameplay is accepted by this source composition. Only our verified authored content is eligible; no Elite/Oolite asset is introduced.

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

The HUD, five docked screens and initial modular equipment candidate passed three-platform qualification and bounded gameplay checks in the new signed Mac app. Current compiled correction `beab907` passed 372 Rust tests and 88 Python tests on each platform plus all five hosted checks. Its actual reports cover 256 of the original failed 306 changed lines (83.66 percent), with unmapped lines counted as uncovered. Public release remains held for final protected integration and exact-main Sonar acceptance. Full equipment, mining/salvage, EMP, bounties, further authored hulls and NPC populations remain core work before new missions.

In the next-preview development candidate, `/` toggles controls help and ordinary flight uses a compact overlay above the cockpit. This change is not in the published v0.4 binary.

The [planet and sun implementation](docs/CELESTIAL-SHADING.md) binds core-owned bodies and procedural surfaces, with swept flight safety and explicit native acceptance limits.

See [docked preview qualification](docs/DOCKED-PREVIEW.md) for controls, actual tests and native results, release gates and explicit limits. XYZZY/Commander Clarke is a separate following feature.
