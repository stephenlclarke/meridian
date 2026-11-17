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

## Previews and validation

Download qualified macOS arm64 and Windows x64 prereleases from [GitHub Releases](https://github.com/stephenlclarke/meridian/releases). Use the launcher and Read Me included with your download. Keep its app, notices and versioned save folder together; preserve earlier previews and profiles separately. Releases use Conventional Commit versions and immutable annotated tags, and retain the newest three published previews after verified downloads and a durable receipt.

Mac apps are Developer ID signed and independently verified after archive extraction. Signing is separate from notarization: the required Apple developer agreement remains unresolved, so notarization and Gatekeeper trust remain open. Windows games remain unsigned; each package documents its runtime requirements. Native Windows interactive gameplay remains unobserved.

## Current Rust checkpoint

Compiled correction `4e5afab` passed independent combined source review and qualification on Mac, Dell Linux and native Windows: 293 core plus 176 desktop plus three explicit observer cases, or **472 top-level Rust tests**, alongside 88 Python tests, 40 Markdown files, formatting, all-target checks, strict Clippy and fresh locked release builds. Nested cold-process summaries are not additional cases. The current signed Mac app rendered its title, a fresh ordinary commander, System and Trading frames, Back/HUD and normal exit; the saved commander belongs only to its new isolated validation slot.

The earlier bounded Metal NPC journey remains tied to `bd506f9`: SH025 traders and pirates, scanner contacts and four views, two paid player-owned missile impacts, incoming NPC Pulse damage, safe/unlock, physical docking and Save/Quit/fresh-process Continue. The complete 18-line public state projection matched across cold Continue. Current source review supports scoped behavior-equivalence reuse; that older gameplay run is not relabelled as execution of the new binary. Earlier freight acceptance separately demonstrated eight identified one-tonne handovers and the once-only 730 CR reward and trusted clue.

Exact main `80a0b16` cleared duplication to 0.0 percent and retained zero open source findings, but its coverage gate failed at 77.9 percent against 80 percent. Both new platform archives remain held and unpublished. The changed adapter correction adds five authored, unrun tests of production menu routing and HUD preparation/submission; existing native gameplay evidence retains its earlier binary identity.

The twelve new preparation/query/replay tests address the actual coverage and duplication failure on main `a43e8b9`. The original 223-line failed coverage set is retained and remeasured against actual current-source LCOV, with 204 of the original 223 lines covered (91.48 percent), unknown mappings uncovered and no helper-only credit for NPC-specific contact paths. Exact committed-main quality gates, independent archive/signature checks and verified public downloads remain required before publication. See [implementation status](docs/IMPLEMENTATION-STATUS.md) for actual source identities, measurements and retained failures.

Successful player Pulse contact, an NPC kill and more than three actual projectiles were not observed in the bounded native journey. Other populations, NPC-owned missile launch producers, additional hull admissions, full equipment, memory, window/focus edges and measured frame timing/race remain open. Full original **1337 visual and gameplay parity** is required; source tests and renderer screenshots do not establish complete game acceptance. Private save format 15 refuses draft 14 and retains explicit qualified older readers.

## Play

N starts a commander; C continues. A second N confirms replacing the current commander, and the old disk file remains until an explicit save or normal saved quit. Click the window for keyboard focus.

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
