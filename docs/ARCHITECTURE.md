# Architecture

Historical feature contract and author-stage notes. For current delivered behavior, exact validation and open limits, read [implementation status](IMPLEMENTATION-STATUS.md). Earlier UNRUN statements below apply to the original handoff, not the latest delivered source.

## Product intent and scope

Build a releasable single-player space game with a portable Rust simulation and a direct wgpu renderer. The first supported release target is an Apple Silicon Mac desktop app. Windows is the next portability target, with Linux useful for core development and tests. Browser, mobile, multiplayer and full legacy parity are outside the first playable scope. Mac App Store distribution remains a later qualified delivery target, not an automatic result of using Rust.

Rust and wgpu are chosen. winit is the proposed desktop window/input adapter. The first implementation pins wgpu 29.0.4, winit 0.30.13, pollster 0.4.0 and bytemuck 1.25.2 in Cargo.lock, with Rust 1.99.0. Those locally available APIs were inspected for source compatibility; compile/GPU compatibility is not yet verified. Deployment OS minimums remain open. The workspace contains a real headless core and desktop implementation.

## Ownership and execution

Use coarse actor-style ownership where work crosses threads, with a synchronous deterministic core underneath. An actor is an ownership boundary, not a requirement to use a particular async runtime. Begin with ordinary calls and a single simulation owner; introduce worker threads only where measured stalls justify them. No actor, task or thread per ship, missile, cargo item or mission.

| Owner | Owns | Receives | Produces |
| --- | --- | --- | --- |
| Desktop main thread | Window, lifecycle, input translation, surface presentation | OS events, latest presentation snapshot | Typed input commands, frame requests |
| Simulation host | Exactly one mutable World, clock, RNG and transaction sequence | Ordered commands, validated content, explicit tick | Domain events, immutable render/save snapshots |
| Renderer | wgpu device, pipelines, textures, meshes and frame resources | Render snapshot and immutable asset handles | Presented frames and graphics diagnostics |
| Asset worker, when needed | Parsing and conversion buffers | Explicit load requests | Validated immutable asset data or errors |
| Save worker, when needed | File handles and atomic file replacement | Immutable save snapshot and destination | Durable completion or error acknowledgement |

GPU ownership initially stays in the desktop event loop to keep window/surface lifecycle simple. Domain state never contains wgpu handles. The renderer cannot award credits, complete objectives or change collision results. Asset and save workers cannot mutate World. Any future Swift bridge is a platform adapter and does not create a second simulation owner.

At a thread boundary, use bounded queues. Reliable domain commands have sequence numbers and explicit rejection/backpressure; never silently drop purchase, mission or save commands. Mouse motion can be coalesced before commands are formed. Presentation is latest-snapshot delivery with at most two retained snapshots; stale render snapshots may be replaced. Save completion is reliable and is shown as successful only after durable acknowledgement. Snapshots must not recursively clone the complete asset catalogue or historical World.

## Proposed workspace boundaries

These are initial logical modules; split into crates only when a dependency boundary benefits from enforcement. Avoid empty crates and scaffolding for unimplemented systems.

| Module | Responsibility | Forbidden dependency |
| --- | --- | --- |
| core | World, IDs, units, ticks, flight, collision, economy, mission transitions and deterministic RNG | wgpu, winit, filesystem, global wall clock |
| content | Typed definitions and validation for ships, stations, systems, equipment and missions | Mutable gameplay or GPU state |
| assets | Offline import/conversion, provenance manifest, runtime decoded assets | Implicit authority to change mechanics |
| render | Camera, visibility, mesh/material upload, WGSL passes and HUD drawing | Mutable World or mission authority |
| save | Versioned DTOs, validation and atomic storage adapter | Renderer or automatic legacy-profile mutation |
| desktop | Event loop, input bindings, audio adapter, lifecycle and composition | Gameplay rules embedded in callbacks |

The first executable should wire only the modules needed by the first playable. Use typed IDs and explicit money, mass, distance and time units. Credits use an integer smallest unit, preserving the reference rules. Entity removal invalidates generational handles; content IDs remain stable across save/load.

## Simulation contract

A proposed fixed tick of 60 Hz advances authoritative state; this is an implementation default to verify against reference mechanics, not a claim that the legacy game used it. Rendering interpolates presentation state without changing physics. Wall time feeds a bounded accumulator in the host; long stalls do not generate an unlimited catch-up loop or silently advance mission time. Pause, menus, hyperspace and station transitions each have explicit game-clock semantics drawn from the legacy acceptance fixtures.

Commands are validated and applied transactionally. A rejected action leaves relevant state unchanged. Mission completion, original-item custody, paid equipment, arrival authority and once-only payment are domain invariants. Loading descriptive save fields never manufactures an action authorization. Use seeded RNG, canonical serialization and command logs for replay. Same-build/platform deterministic replay is the initial promise; cross-architecture bit identity needs separate proof, especially for floating-point collision and rendering.

## Graphics with wgpu

Use direct wgpu with WGSL shaders and winit for desktop lifecycle/input. wgpu provides native Metal and Windows/Linux backends, but each shipped backend requires actual-device validation. Reference: [wgpu](https://wgpu.rs/) and [winit lifecycle](https://docs.rs/winit/latest/winit/application/trait.ApplicationHandler.html).

The first renderer draws the selected player ship/cockpit view, required station geometry, celestial bodies, navigation cues and readable HUD/UI. Introduce one depth-tested opaque pass, then only the transparency or overlay passes the scene actually needs. Keep render meshes separate from collision representations. Preserve source transforms, coordinate conventions, winding, units, UVs and materials through explicit importer tests. Existing Swift/Metal shader code is reference material; translating it to WGSL requires visual validation.

Handle zero-sized/minimized surfaces without configuring or drawing invalid frames. Resize and lost/outdated surfaces trigger controlled recreation; device loss becomes a visible recoverable error or clean exit, never a save mutation. Cap per-frame uploads and reuse GPU allocations based on measured scene requirements. Record CPU and GPU allocation categories separately. Do not claim that Rust bounds GPU memory use automatically.

## Persistence and errors

Meridian uses its own versioned save schema and profile directory. Existing Swift profiles remain untouched. Legacy save migration is deferred until an explicit mapping and read-only fixtures prove it; it is not required to play a new Meridian commander.

Save at a committed simulation boundary. Encode the complete current state needed for the journey, including item identities, balances, paid equipment, mission progress, clock and RNG state. Write a temporary file on the destination filesystem, flush and atomically replace the owned save using the platform adapter. Preserve the last valid save on failure. Validate version, content references and semantic invariants before replacing the in-memory state. Unsupported/corrupt saves produce a useful error and do not partially load or silently repair themselves.

Use Result and typed domain errors for expected failures. Panics are defects, not gameplay control flow. Keep application-owned unsafe code absent initially; any required platform/FFI boundary must be isolated and tested. Rust ownership checks do not replace allocation profiling, queue bounds, deadlock analysis or GPU diagnostics.

## Acceptance and open decisions

The first complete test is the journey in [FIRST-PLAYABLE.md](FIRST-PLAYABLE.md). Tests exercise real domain commands and rendered gameplay, with source references recorded separately from actual execution evidence. Performance targets are proposed there and must be measured, not described as already met.

Open implementation decisions: exact dependency/toolchain versions; UI text/font and audio adapters; precise first-journey content closure; source asset distribution status; reference physical numerical tolerances; approved minimum OS; release signing identity and distribution channel. None requires interrupting the current Swift app delivery to prepare this plan.
