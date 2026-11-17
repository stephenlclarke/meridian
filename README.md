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
[![Notarization unconfirmed](https://img.shields.io/badge/notarization-unconfirmed-orange)](docs/IMPLEMENTATION-STATUS.md)
[![Source-visible license](https://img.shields.io/badge/license-proprietary%20%2F%20source--visible-blue)](LICENSE)

Meridian is a Rust reimplementation of the original L337 project, with direct wgpu graphics, one authoritative World and traceable original content. New code is [proprietary/source-visible](LICENSE); imported original content retains its own notices. Original Elite/Oolite assets are excluded.

## Previews and validation

Download qualified macOS arm64 and Windows x64 prereleases from [GitHub Releases](https://github.com/stephenlclarke/meridian/releases). Use the launcher and Read Me included with your download. Keep its app, notices and versioned save folder together; preserve earlier previews and profiles separately. Releases use Conventional Commit versions and immutable annotated tags, and retain the newest three published previews after verified downloads and a durable receipt.

Mac apps are Developer ID signed and independently verified after archive extraction. Signing is separate from notarization. Apple accepted the latest submission, but subsequent status requests returned HTTP401; the final decision and stapling remain unconfirmed. The account owner reports no outstanding agreements. Windows games remain unsigned; each package documents its runtime requirements. Native Windows interactive gameplay remains unobserved.

## Current Rust checkpoint

Preview 6 candidate `a1ee135` adds paid Beam and Mining Lasers, Extra Energy System and Fuel Scoops, shared resource accounting, physical asteroid work and persistent pending ore. Mac qualification passed 534 workspace Rust tests; Dell Linux and native Windows passed those tests plus three explicit observer cases, or 537 each. All passed 94 Python tests, 46 Markdown checks, formatting, all-target checks, strict Clippy and locked release builds. Hosted PR checks passed. Nested subprocess summaries are not extra cases.

The installed Developer ID signed Mac preview was operated through the supported computer-use adapter. An explicitly funded headless setup was continued in its isolated profile; native purchases fitted the Mining Laser for 3300 CR and Fuel Scoops for 1900 CR, leaving 3314.5 CR. Two real front-contact strokes reduced one asteroid from 100 to 50 work and then exhausted it, producing exactly one pending Alloys unit. Both partial work and terminal pending identity survived Save/Quit and Continue in new processes. Cash and cargo stayed unchanged. This validates the bounded mining and persistence path; funding was setup, not player-earned native acceptance.

Physical ore pickup and sale remain unavailable without eligible original body geometry. Fitted scoops do not grant cargo or income. The current scope does not include XYZZY, the subsequent navigation-computer candidate or destroyed-trader custody work. Both Preview 6 internal archives are prepared; protected integration, exact-main CI/Sonar and verified GitHub publication remain separate stages.

Earlier qualified previews retain their own native flight, physical docking, trading, finite missiles, freight deliveries, once-only 730 CR reward and trusted-clue evidence. Those journeys are not relabelled as execution of this binary. Native Windows interactive gameplay, remaining populations and equipment, memory qualification, window/focus edges and measured frame timing/race remain open. Full original **1337 visual and gameplay parity** and the expanded mission/fleet acceptance remain open. The candidate writes private save v16; future development has no backward-compatibility guarantee.

See [implementation status](docs/IMPLEMENTATION-STATUS.md), [mining scope](docs/ASTEROID-MINING.md) and [source-grounded roadmap](docs/LEGACY-MILESTONES.md).

## Following navigation and custody candidate

Compiled candidate `991fd91` passed owned Mac, Linux and native Windows qualification: 551 Rust tests each, 94 Python tests, 47 Markdown checks, strict Clippy and locked release builds. It reads and writes only V17 and refuses earlier formats without changing the file. In the signed isolated Mac app, supported native input purchased an Intermediate Navigation module for 3000 CR, stored and refitted the same paid item, and retained it across a new-process Continue. Module reach was 14 LY; stored reach fell to the SH025 floor of 9 LY. The chart correctly showed the actual 9.6 LY fuel limit. No serviced flight benefiting from a leg beyond 9 LY is accepted: only Avenor and Renas services exist, and the required cumulative fuel exceeds tank capacity.

The next source increment adds the original Lareria G01-S091 main orbital. The paid Intermediate grade enables the 9.93 LY Avenor–Lareria leg, costing 99 fuel units. Actual berthing provides paid refill for a return journey. Exact Lareria market data is imported, and the desktop admits this one additional destination. Rust compilation and native Lareria acceptance remain pending.

A separate explicitly prepared public-core setup supplied four nonfatal contacts and a live trader. One supported native final shot from a paid missile produced the exact terminal SH025 Food manifest, preserving its native endpoint and source pose across Save/Quit and new-process Continue. Cargo and cash stayed unchanged; the retained hull stopped the player on physical contact. Setup funding, piloting and earlier damage are not native journey credit. Physical pods, pickup, sale and salvage income remain unavailable. Independent audit cleared these bounded observations; integration and publication remain open. Preview 6 keeps its separate source and release evidence.

## Play

N starts a commander; C continues a current-format V17 save. Earlier formats refuse without changing the file. Use a separate profile for this candidate. A second N confirms replacing the current commander, and the old disk file remains until an explicit save or normal saved quit. Click the window for keyboard focus.

| Controls | Action |
| --- | --- |
| 1 / 2 / 3 / 4 / 5 when docked | Galaxy / System / Ships / Market / Equipment |
| L; W/S; arrows; A/D; Q/E; Space | Launch; thrust; pitch; yaw; roll; brake |
| J; T; R; Enter; B; F | Travel; select contact; review; confirm; cancel; buy fuel |
| M; [ / ]; G/H | Market; commodity selection; buy/sell one native unit |
| O; K; U; Y | Equipment panel; buy/fit cargo bay; store; refit |
| I; V; X | Mission details; review all held freight deliveries; review abandonment |
| F1; F2; F3; F4 | Next ship/practice lock; arm/safe; launch one missile; safe/unlock |
| F7; F8; F10 | Review attempt closure; reception; fresh consent |
| F6; F12; C; 0/1/2/3 | Flight lesson consent/reception; cancellation; details; view direction |
| P; F11; F5; F9 | Pause; fullscreen; save; reload the current qualified save |
| Escape / window close | Save before exit; failure keeps the app open |

The freight journey requires eight identified one-tonne deliveries and awards 730 CR plus the trusted clue once. Markets use finite stock and the same account as mission and equipment transactions. Invalid saves refuse without migration, repair or fallback.

## Build and contribute

The workspace pins Rust and dependencies in [rust-toolchain.toml](rust-toolchain.toml) and [Cargo.lock](Cargo.lock). Use `make ci` for documentation, formatting, importers, compilation, tests and strict Clippy. `make run` starts the desktop app in an owned graphics session. G1 owns Mac compilation/native validation; Dell owns Linux/native Windows validation; MBA retains its existing fleet work.

- [Development and repository maintenance](docs/DEVELOPMENT.md)
- [Architecture](docs/ARCHITECTURE.md) and [core API](crates/meridian-core/README.md)
- [First playable criteria](docs/FIRST-PLAYABLE.md) and [source-grounded roadmap](docs/LEGACY-MILESTONES.md)
- [Content provenance and reuse](docs/LEGACY-REUSE.md)
- [Finite missile behavior and save boundary](docs/FINITE-STARTER-MISSILES.md)

GitHub Actions validates Linux, Windows and macOS. Protected main requires pull requests, current required checks, linear history and resolved conversations; force pushes and deletion are blocked, including for administrators. SonarQube runs after merge from committed main with the exact commit SHA and Previous version policy. Historical change descriptions live in merged GitHub PRs and Git history rather than duplicate issue/PR Markdown files.

## Next two milestones

These are the source-grounded M2 and M3 boundaries; completed entries below describe bounded reusable Rust slices, not completed milestones or all 64 mission acceptances.

| State | M2: flight, equipment and ordinary economy | M3: mission families and trusted clues |
| --- | --- | --- |
| Completed bounded slices | Seventeen finite commodity markets; basic paid equipment/storage/refitting; Pulse and finite missile/damage/save mechanics; bounded SH025 trader/pirate journey. | Freight custody, once-only receiving/payment/clue and cold-Continue foundations; bounded gunnery and navigation/trade lesson mechanics. |
| In progress | Qualified NPC preview delivery; rapid/advanced lasers and energy profiles; authored mining and paid Fuel Scoops source; typed utility/XYZZY UI integration. Positive ore collection still needs eligible original body geometry. | Core prerequisites come first. Existing family mechanisms and source walkthroughs do not close the expanded mission acceptance ledger. |
| Not yet implemented or accepted | EMP, positive salvage and bounty/legal consequences; remaining equipment and population gameplay acceptance. | Rust crew/passenger/rescue, investigation and Wayfarer family journeys, then their individual consent/failure/reward/cold-Continue acceptance. All 64 expanded runtime acceptances remain open. |

Fleet construction is a separate M4 lane. Its current changed original-five run returned no candidate meshes; no new hull admission follows from focused tests. Original mesh/material/colour fidelity and 1337 visual/mechanics parity remain requirements across these milestones. See the [source-grounded roadmap](docs/LEGACY-MILESTONES.md).

## XYZZY core author packet (unqualified)

The [explicit XYZZY core](docs/XYZZY.md) adds Commander CLARKE, once-only finite cash, current supported hardware with secret provenance, and accounted unlimited fuel on exact e1cc86b. Twenty new Rust cases are authored and unrun; host HELP/XYZZY composition, independent review and compiler/native execution remain required before preview delivery. The [36-action inventory](docs/XYZZY-ACTIONS.json) records actual coverage and missing owners; full legacy parity is incomplete.
