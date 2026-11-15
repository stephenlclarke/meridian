# Meridian architecture and first playable

## Motivation

The user requested a fresh Rust reimplementation named Meridian, using wgpu graphics and selected assets/mechanics from Elite/L337. The current Swift app delivery and MBA fleet work must continue in parallel.

## Required result

Create a separate local repository with an explicit simulation/render/content/save architecture, a complete first-playable milestone and a source-grounded legacy reimplementation roadmap prepared by an existing session. Keep proposed behaviour separate from implementation and acceptance evidence.

## Compatibility and risks

No legacy saves or human WIP are changed. No source asset is yet imported. Resource-specific provenance and applicable distribution terms need per-file resolution. Rust code, dependencies, graphics execution and app packaging are later implementation work.

## Links

- [Change details](PR-meridian-architecture.md).
- [First playable](docs/FIRST-PLAYABLE.md).
