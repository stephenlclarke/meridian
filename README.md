# Meridian

Meridian is a fresh Rust reimplementation of the existing Elite/L337 game. It reuses selected assets and intentional mechanics through a traceable import and parity process. Graphics use wgpu, as requested by Stephen on 15 November 2026. The existing Swift application continues its separate delivery and fleet work.

Status: first flight and docking geometry source implemented: headless fixed-step simulation, direct wgpu desktop renderer, original SH025 display mesh and scoped freight domain rules. The native Windows flight prototype runs on the Dell's Intel Iris Xe using Vulkan. At source 4b55126, Windows compilation, all 33 tests and strict Clippy pass; Stephen confirmed the visible scene, focused flight controls and window handling. Earlier source ec2fd1e also passes compiler/tests/Clippy under Ubuntu WSL2. The complete first playable journey remains unfinished. The private GitHub repository is [stephenlclarke/meridian](https://github.com/stephenlclarke/meridian). The local checkout is `~/github/meridian`; `origin` points to that repository.

## Start here

- [Architecture](docs/ARCHITECTURE.md): ownership, module boundaries, graphics, persistence and failure handling.
- [First playable milestone](docs/FIRST-PLAYABLE.md): one complete journey and its acceptance criteria.
- [Legacy milestone breakdown](docs/LEGACY-MILESTONES.md): G3 source-grounded roadmap from the first playable to full mission and fleet parity.
- [Legacy reuse](docs/LEGACY-REUSE.md): exact reference revision, content provenance and mechanics parity.
- [Development and delivery](docs/DEVELOPMENT.md): machine roles, checks and release evidence.
- [Decision: Rust, wgpu and ownership](docs/decisions/0001-rust-wgpu-ownership.md).

Read [implementation status](docs/IMPLEMENTATION-STATUS.md) before running the prototype. When the execution lane is available, use `make check`, `make test`, `make clippy` and `make run`. Click inside the game window to give it keyboard focus. The shell uses W/S for thrust/deceleration, Up/Down for pitch, A/D for yaw, Q/E for roll, Space for brake, P for pause, F11 for fullscreen and Escape to exit. Speed is displayed in reference gameplay units, not a qualified physical calibration.

Run `make docs-check` to lint the documents and `make assets-check` for headless Python import checks. The legacy milestone breakdown was prepared by the existing G3 session from immutable source. The full journey, docking, economy, mission authority, saves and a packaged playable app remain in progress; the first milestone is not complete.
