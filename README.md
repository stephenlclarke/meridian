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

## Qualified NPC development

The [finite traffic increment](docs/NPC-TRAFFIC.md) at `bd506f9` has passed independent source review, all three owned platform qualifications and a bounded native Metal journey. Each platform passed 460 top-level Rust tests (288 core, 169 desktop and three explicit observer cases), 88 Python tests, strict Clippy and a fresh release build. All five hosted PR checks passed on that exact source.

Actual Mac play showed SH025 traders and pirates, scanner contacts and four selected camera views, two paid player-owned missile impacts, incoming NPC Pulse damage, safe/unlock controls, physical docking and normal Save/Quit/fresh-process Continue. Cash, Food and finite ammunition persisted; the complete 18-line public state projection matched across cold Continue. The NPC increment merged through PR 38 at main `8b81a77`. All six hosted jobs passed, and Sonar returned a 93.8 percent new-code coverage gate with no ignored condition, but also reported 20 open critical source findings. The explicit-import and helper-extraction correction passed independent source review, 460 Rust and 88 Python tests on each owned platform, all five hosted checks and a current signed Mac title/render/normal-close smoke. The next preview remains held for corrected exact-main quality verification and new source-bound archives; v0.5 remains the public preview.

Successful player Pulse contact, an NPC kill and more than three actual projectiles were not observed in this native journey. Police, miners, bounty hunters, aliens, AI constructs and NPC-owned missile launch producers remain open, as do full original 1337 visual/gameplay parity and additional hull acceptance. Private save format 15 refuses draft 14 and retains explicit published readers; earlier previews and real commander profiles stay separate.

## Play

N starts a commander; C continues. A second N confirms replacing the current commander, and the old disk file remains until an explicit save or normal saved quit. Click the window for keyboard focus.

| Controls | Action |
| --- | --- |
| L; W/S; arrows; A/D; Q/E; Space | Launch; thrust; pitch; yaw; roll; brake |
| J; T; R; Enter; B; F | Travel; select contact; review; confirm; cancel; buy fuel |
| M; [ / ]; G/H | Market; commodity selection; buy/sell one native unit |
| O; K; U; Y | Equipment panel; buy/fit cargo bay; store; refit |
| I; V; X | Mission details; review all held freight deliveries; review abandonment |
| F1; F2; F3; F4 | Next ship/practice lock; arm/safe; launch one missile; safe/unlock |
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

The current traffic correction yields the native station approach to an aligned inbound player and selects actual hull-surface aim points for NPC beams and player-owned missiles. It retains the player collision checks and original native first-contact geometry. The combined correction has 86 additional authored cases, all **UNRUN**; the last tested source remains `42f599c` with 223 core passes and 53 failures. Owning validation must establish the correction before another playable preview.

Latest NPC qualification on `2b7b350` reached 282 core passes and four failures. The four-case follow-up changes public test piloting and format-specific fixtures while preserving collision, contact and save-history assertions; it is unrun and awaiting review. The delivered public preview remains unchanged.

The next NPC core run on `4e83640` reported **284 passed, two failed**. The remaining source-only fixture work speed-matches the trader before launch and plans the return around current live NPCs and retained wrecks. The capacity and legacy cases passed in that run. Production guidance, collisions and limits remain unchanged; corrected execution is still pending.

### NPC qualification: strict Clippy correction

Exact `3a53949719267cd1717da71a9b8c001a13b49fdf` Mac qualification passed
286 core and 169 desktop tests (455 top-level cases), including both previously
failing pilot journeys. Nested cold-process summaries are not additional cases.
The 88 Python tests, 40 Markdown files, formatting and all-target check passed.
Strict Clippy then stopped qualification with six unique diagnostics.

The following source correction only collapses four nested conditions into
short-circuit let chains, replaces an unsigned modulo predicate with
`is_multiple_of(4)`, and moves an unchanged test module after the existing v13
fixture helper. No assertion, collision rule, resource or save behavior, lint
setting or gate is weakened. The corrected source is uncompiled and untested
until independent delta review and owning execution. Three observer example
cases, release build, coverage, signing and native gameplay remain unrun.
There is no new qualified preview or full 1337 parity claim.

Mac qualification passed all 455 workspace Rust cases on the current NPC source, while Linux retained one checked-save failure in the seven-round case. A source-consistency correction now unifies new NPC movement and receipts on the exact native ray endpoint, with two authored unrun regressions and an independently frozen native-geometry arithmetic reference. The actual Linux failed guard and values remain unknown; this is not a verified Linux cure.

### Combined endpoint and renderer correction

Root composes the canonical NPC ray correction with the equivalent renderer
let-chain fix from the actual Mac `75a88c0` Clippy failure. That Mac run passed
455 workspace Rust tests and 88 Python tests, then failed desktop Clippy;
observer, release and native acceptance remained unrun. Both Dell platforms
failed the earlier seven-round cold checked-save case after 285 passing core
tests. The actual rejecting guard and values remain unknown.

The independent endpoint counterexample proves a source consistency defect,
not the missing Dell state or a tested cure. All composed tests, compiler,
Clippy, release and native validation remain pending on this exact correction.
Qualified previews follow actual validation; full original 1337 visual and
gameplay parity remains an explicit requirement.
