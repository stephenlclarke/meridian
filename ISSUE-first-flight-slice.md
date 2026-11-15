# First Rust flight slice

## Motivation

Start implementing Meridian with Rust and direct wgpu while the current Swift app and MBA fleet continue. Prove small ownership and renderer boundaries before porting broader gameplay.

## Scope

A headless fixed-step flight core, winit/wgpu chase-view shell, original SH025 display import and isolated freight allocation/settlement domain tests. Preserve source provenance and explicit incomplete acceptance. No docking, receiving, economy or save result is implied.

## Validation and limits

See [implementation status](docs/IMPLEMENTATION-STATUS.md). Rust compilation, behavioral tests and actual GPU play await G1's available execution lane. No concurrent compiler or native job was launched.

## Links

- [Change details](PR-first-flight-slice.md).
- [First playable criteria](docs/FIRST-PLAYABLE.md).
