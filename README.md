# Meridian

[![CI](https://github.com/stephenlclarke/meridian/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/stephenlclarke/meridian/actions/workflows/ci.yml)
[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=alert_status)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=coverage)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)

Meridian is a Rust reimplementation of the original L337 project, with direct wgpu graphics, one authoritative World and traceable original content. New code is [proprietary/source-visible](LICENSE); imported original content retains its own notices. Original Elite/Oolite assets are excluded.

## Current delivery

The signed local **Meridian Finite Missiles Preview** is delivered on the primary MBP graphics target from gameplay main `d5be89b` ([PR 17](https://github.com/stephenlclarke/meridian/pull/17)). It includes flight, physical docking, Avenor/Renas travel, seventeen-commodity trading, original G02 freight, paid cargo equipment, front Pulse fire, practice beacons/reception, ordinary station impacts and three finite owned missiles. Actual Metal gameplay and scoped cold continuation checks passed. Mac, Linux and native Windows validation passed 179 top-level Rust tests and 48 Python importer tests; exact-main CI and SonarQube passed.

**Earlier installed v7 preview limit:** any missile use, including ended projectile history, disables saving/restoring that session. Before firing, F4 clears lock/arm so saving can resume. A failed save preserves the previous file and keeps the app open; Q explicitly quits without saving. Use the preview's dedicated launcher and save directory. The repository now supports complete missile Save v8. Reviewed candidate `58f45c8` passed 193 Rust tests, 48 importer tests, strict Clippy and builds on all three platforms, plus scoped actual Metal cold continuation for armed, live, impacted, exhausted and dock/relaunch states. A new installed preview is pending final protected merge/main CI and packaging. See [durable missile sessions](docs/DURABLE-MISSILE-SESSIONS.md); the existing v7 preview remains unchanged.

This is a functional local preview. Memory qualification, sustained minimize/focus-loss, frame timing/race evidence and commercial distribution remain open. See [implementation status](docs/IMPLEMENTATION-STATUS.md) for the exact scope and limits. Existing preview apps and real player profiles remain separate.

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
| P; F11; F5; F9 | Pause; fullscreen; save; reload (complete missile state in v8) |
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
