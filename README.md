# Meridian

[![CI](https://github.com/stephenlclarke/meridian/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/stephenlclarke/meridian/actions/workflows/ci.yml)
[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=alert_status)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_Meridian&metric=coverage)](https://sonarcloud.io/dashboard?id=stephenlclarke_Meridian)

Meridian is a Rust reimplementation of the original L337 project, with direct wgpu graphics, one authoritative World and traceable original content. New code is [proprietary/source-visible](LICENSE); imported original content retains its own notices. Original Elite/Oolite assets are excluded.

## Current delivery

The signed local **Meridian Durable Missiles Preview** is installed from gameplay main `ab238c3` ([PR 19](https://github.com/stephenlclarke/meridian/pull/19)). It includes flight, physical docking, Avenor/Renas travel, seventeen-commodity trading, original G02 freight, paid cargo equipment, front Pulse fire, practice beacons/reception, ordinary station impacts and three finite owned missiles. Save v8 retains the complete missile session, including live/ended projectiles and consumed rounds. Mac, Linux and native Windows each passed 193 top-level Rust tests, 48 importer tests, strict Clippy and builds. Exact-main CI and SonarQube passed.

Start it with `~/Applications/Launch Meridian Durable Missiles Preview.command`. This uses the preview's separate `Library/Application Support/Meridian Durable Missiles Preview/saves-v8` directory. Actual signed Metal cold checkpoints passed for armed/locked, paused live, impacted, exhausted and physical dock/relaunch states, plus copied v4-v7 Continue and invalid-save refusal. Final installed title/render/normal-close smoke exited zero without creating a commander save. All 24 installed files/modes matched staging; 197 prior artifact files/modes were unchanged.

The earlier **Finite Missiles Preview** remains available with its v7 save refusal after missile use. Keep its dedicated profiles separate; its warning still applies to that older app. The new Durable Missiles Preview saves complete v8 state. See [durable missile sessions](docs/DURABLE-MISSILE-SESSIONS.md) for scope and limits.

This is a functional local preview. Memory qualification, sustained minimize/focus-loss, frame timing/race evidence and commercial distribution remain open. See [implementation status](docs/IMPLEMENTATION-STATUS.md) for the exact scope and limits. Existing preview apps and real player profiles remain separate.

## Current source under development

The [safe-gunnery attempt lifecycle](docs/SAFE-GUNNERY-ATTEMPTS.md) adds reviewed F7 closure at the actual Avenor berth, explicit F10 fresh consent and retained attempt history in save v9. I shows completed, failed and abandoned counts and the once-only first trace. Closure grants no reward, refund or ammunition. These changes and tests are authored; independent review, compiler/host/native qualification and delivery are pending. The installed v8 preview above is unchanged. Use a separate isolated save directory for candidate v9 execution.

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
