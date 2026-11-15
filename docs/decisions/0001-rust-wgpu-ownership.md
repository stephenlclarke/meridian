# Decision 0001: Rust, wgpu and simulation ownership

Date: 15 November 2026.

Status: Rust successor, repository name Meridian and wgpu graphics directed by Stephen. The detailed module split and execution model below are proposed initial architecture, ready for implementation refinement.

## Context

The existing game has grown across simulation, physical content, missions, saves and native UI. The user wants a fresh commercial-product architecture, borrowing existing assets and intentional mechanics while the current playable app continues delivery. A rewrite must preserve game requirements and prove one complete journey before expanding.

## Decision

Use Rust for the new application and direct wgpu with WGSL for graphics. Propose winit as the desktop lifecycle/input adapter. Start with an explicit simulation World owner and pure fixed-step transitions. Keep renderer state, assets and persistence outside World. Actors are coarse ownership boundaries where concurrency helps; entities are ordinary data. No actor framework, per-entity messaging, general plugin engine, Swift bridge or async runtime is required for the first slice.

Keep the core headless and deterministic under explicit inputs/seed. Use bounded reliable domain commands and replaceable presentation snapshots if moved across threads. Keep GPU/window lifecycle together initially. Treat file writes as acknowledged effects over immutable save snapshots. Use a fresh Meridian save namespace.

## Consequences

This allows domain tests without a GPU and a portable renderer without tying rules to the window loop. It also makes Meridian responsible for rendering infrastructure, UI, audio integration, content tools and packaging that a full engine would otherwise supply. Rust does not automatically prevent excessive allocation, stack overflow, native-library faults or GPU resource growth.

A new repo isolates design and dependency changes but duplicates some development and test work. Selected asset provenance and intentional mechanics must be extracted explicitly. The first playable measures whether this architecture improves the product; it does not assume a successful rewrite or a release date.

## References

- [wgpu overview](https://wgpu.rs/).
- [wgpu API and backend documentation](https://docs.rs/wgpu/latest/wgpu/).
- [winit lifecycle](https://docs.rs/winit/latest/winit/application/trait.ApplicationHandler.html).
- [Rust ownership](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html).
- [Rust leaks and stack overflow example](https://doc.rust-lang.org/book/ch15-06-reference-cycles.html).
