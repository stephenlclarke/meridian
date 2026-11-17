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

[Preview 7 v0.7.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.7.0-preview.1) is published for macOS arm64 and Windows x64 from main `a05c69e`, compiled `076639d`. It includes contextual H/? Help on every station screen and in flight, station navigation instructions, and the explicit Ctrl+Shift+X XYZZY testing menu. All three owned platforms passed 603 Rust tests, 99 Python tests, 48 Markdown checks, formatting, all-target checks, strict Clippy and release builds. All six exact-main CI jobs and Sonar passed with zero unresolved issues and 91.8% new-code coverage. The two documentation changes between compiled and published source leave all game, asset, lock and test bytes unchanged.

All four public downloads, complete archive payloads, extracted Mac Developer ID signature and annotated tag were verified before the durable release receipt and retention cleanup. The newest three previews are v0.7, v0.6 and v0.5. The new signed Mac binary passed title/System Help, blocked modal input, explicit XYZZY activation and one intentional cold Continue across two launches and two observed normal quits. Its real save decoded through the new qualified core. The broader `c98fc86` native journey below retains its own binary identity. Windows GUI gameplay, full original XYZZY parity, native Lareria travel/refill, memory and measured performance remain open. Current saves use V17; no backward compatibility is required.

### Earlier Preview 6 mining evidence

[Preview 6 v0.6.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.6.0-preview.1) is published for macOS arm64 and Windows x64 from main `07bb22c`, compiled `72dcc28`. All three owned platforms passed 539 Rust and 94 Python tests; exact-main CI and Sonar passed with zero open issues. All four public downloads, archive contents, Mac signature and annotated tag were verified before retention. The updated Mac build passed title/render/Escape-close smoke without a commander. Its earlier bounded mining journey retains the separate `a1ee135` source identity. Windows interactive gameplay and normal exit remain unobserved. This release includes neither the help/XYZZY candidate nor Lareria.

Preview 6 candidate `a1ee135` adds paid Beam and Mining Lasers, Extra Energy System and Fuel Scoops, shared resource accounting, physical asteroid work and persistent pending ore. Mac qualification passed 534 workspace Rust tests; Dell Linux and native Windows passed those tests plus three explicit observer cases, or 537 each. All passed 94 Python tests, 46 Markdown checks, formatting, all-target checks, strict Clippy and locked release builds. Hosted PR checks passed. Nested subprocess summaries are not extra cases.

The installed Developer ID signed Mac preview was operated through the supported computer-use adapter. An explicitly funded headless setup was continued in its isolated profile; native purchases fitted the Mining Laser for 3300 CR and Fuel Scoops for 1900 CR, leaving 3314.5 CR. Two real front-contact strokes reduced one asteroid from 100 to 50 work and then exhausted it, producing exactly one pending Alloys unit. Both partial work and terminal pending identity survived Save/Quit and Continue in new processes. Cash and cargo stayed unchanged. This validates the bounded mining and persistence path; funding was setup, not player-earned native acceptance.

Physical ore pickup and sale remain unavailable without eligible original body geometry. Fitted scoops do not grant cargo or income. That Preview 6 scope excludes XYZZY and the subsequent navigation/custody increments. Its completed publication retains its separate source and receipt.

Earlier qualified previews retain their own native flight, physical docking, trading, finite missiles, freight deliveries, once-only 730 CR reward and trusted-clue evidence. Those journeys are not relabelled as execution of this binary. Native Windows interactive gameplay, remaining populations and equipment, memory qualification, window/focus edges and measured frame timing/race remain open. Full original **1337 visual and gameplay parity** and the expanded mission/fleet acceptance remain open. That earlier Preview 6 binary writes private save V16; Preview 7 uses V17 and refuses earlier formats.

See [implementation status](docs/IMPLEMENTATION-STATUS.md), [mining scope](docs/ASTEROID-MINING.md) and [source-grounded roadmap](docs/LEGACY-MILESTONES.md).

## Navigation and custody evidence before Preview 7

Compiled candidate `991fd91` passed owned Mac, Linux and native Windows qualification: 551 Rust tests each, 94 Python tests, 47 Markdown checks, strict Clippy and locked release builds. It reads and writes only V17 and refuses earlier formats without changing the file. In the signed isolated Mac app, supported native input purchased an Intermediate Navigation module for 3000 CR, stored and refitted the same paid item, and retained it across a new-process Continue. Module reach was 14 LY; stored reach fell to the SH025 floor of 9 LY. The chart correctly showed the actual 9.6 LY fuel limit. No serviced flight benefiting from a leg beyond 9 LY is accepted: only Avenor and Renas services exist, and the required cumulative fuel exceeds tank capacity.

The next source increment adds the original Lareria G01-S091 main orbital. The paid Intermediate grade enables the 9.93 LY Avenor–Lareria leg, costing 99 fuel units. Actual berthing provides paid refill for a return journey. Exact Lareria market data is imported, and the desktop admits this one additional destination. That exact Lareria-only source passed 555 top-level Rust and 94 Python tests on Mac, Linux and native Windows, with strict Clippy and locked release. Native Lareria acceptance remains pending. These results do not qualify the following changed help/XYZZY layout.

A separate explicitly prepared public-core setup supplied four nonfatal contacts and a live trader. One supported native final shot from a paid missile produced the exact terminal SH025 Food manifest, preserving its native endpoint and source pose across Save/Quit and new-process Continue. Cargo and cash stayed unchanged; the retained hull stopped the player on physical contact. Setup funding, piloting and earlier damage are not native journey credit. Physical pods, pickup, sale and salvage income remain unavailable. Independent audit cleared these bounded observations. Their source is included in Preview 7 through later joins; these native observations retain the earlier compiled identity. Preview 6 keeps its separate source and release evidence.

## Play

N starts a commander; C continues a current-format V17 save. Earlier formats refuse without changing the file. Use a separate profile for this candidate. A second N confirms replacing the current commander, and the old disk file remains until an explicit save or normal saved quit. Click the window for keyboard focus.

H or ? opens a dismissible help popup for the current screen, including station details and transaction reviews. Every station popup lists 1 Galaxy, 2 System, 3 Showroom, 4 Market, 5 Equipment, Tab/Shift+Tab and Escape/back. Escape, H or ? closes help and retains the screen, selection, search and review. Gameplay and underlying actions wait while help is open. During galaxy text search, H remains a typed letter and ? opens help.

| Controls | Action |
| --- | --- |
| H / ? | Context help; Escape, H or ? dismisses |
| Ctrl+Shift+X when docked | Explicit XYZZY testing menu; Enter reviews then confirms |
| 1 / 2 / 3 / 4 / 5 when docked | Galaxy / System / Ships / Market / Equipment |
| L; W/S; arrows; A/D; Q/E; Space | Launch; thrust; pitch; yaw; roll; brake |
| J; T; R; Enter; B; F | Travel; select contact; review; confirm; cancel; buy fuel |
| M; [ / ]; G / Shift+G | Market; commodity selection; buy/sell one native unit |
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

## Context help and XYZZY testing

Preview 7 combines screen-specific H/? help with explicit [XYZZY testing](docs/XYZZY.md). Every station help popup explains how to reach Galaxy, System, Showroom, Market and Equipment. Help pauses host stepping and preserves the current screen, selection, search and review. The large permanent control list is removed from the flight HUD.

At a station berth, **Ctrl+Shift+X** opens the labelled XYZZY testing menu. Use Up/Down to choose, Enter to review, then Enter again to confirm and save; Escape/B cancels the review or closes the menu. H/? opens help above the menu. First activation changes the current commander to CLARKE and sets cash to 1,000,000 CR once. It grants/refits the supported SH025 outfit with explicit secret provenance; missiles and cargo stay finite. The menu shows actual cash, current jump reach, unlimited-fuel state and missing-system limits. Reading or closing it grants nothing. Other profiles are unchanged.

Source `c98fc86` passed Mac qualification: 603 top-level Rust tests (378 core, two compatibility integrations, 220 desktop and three explicit observer cases), 94 Python tests, 48 Markdown checks, formatting, all-target checks, strict Clippy and locked release. The signed isolated app passed supported native checks of all five station Help cards, navigation, blocked controls, hidden search with nested testing Help, explicit activation, visible grant provenance and a real paid Food purchase. Repeating activation preserved 999,977.4 CR and the purchased tonne. Accounted fuel was switched off, flight Help displayed the fitted Naval Laser, manual pause survived Help, and actual Save/Quit/new-process Continue retained CLARKE and the same cash, cargo and fuel mode. Precise native clock, OS exit status and a Lareria docking/refill journey were not measured. Independent native acceptance and artifact audit passed. Dell Linux and native Windows also passed the same 603 Rust and 94 Python tests, strict Clippy and locked release; native Windows importer checks used native Python. The [36-action inventory](docs/XYZZY-ACTIONS.json) retains every unsupported or partial original action. Preview 7 now publishes these features using the corrected binary described below.

PR 47 merged the composition at `de7d6bb`; main CI and the Sonar quality gate passed, but nine unresolved maintainability findings held publication. Correction `076639d` extracts four helpers and replaces five wildcard test imports with independently reviewed behavior and unchanged assertions. Its new Mac, Linux and native Windows binaries passed 603 Rust tests, 99 Python tests, strict Clippy and release builds. The separately signed Quality Preview passed bounded Help, explicit XYZZY activation and cold Continue checks. PR 48 merged at `a05c69e` after all final-head checks passed. All six main CI jobs and exact-main Sonar passed with zero unresolved issues; v0.7.0-preview.1 was published and its public downloads verified. The broader journey above remains attributed to `c98fc86`.

The Help candidate now lists mission details and flight-lesson review/cancel/details in both the station HUD and flight popups. Its retained minimum-window regression checks the concise HUD hint, all those popup controls, complete card text, bounded rendering and unchanged saved state through docked, launch and all four flight views. The regression passed on the qualified Mac source; minimum-size native resizing remains unobserved.
