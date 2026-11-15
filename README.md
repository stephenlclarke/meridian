# Meridian

Meridian is a fresh Rust reimplementation of the existing Elite/L337 game. It reuses selected assets and intentional mechanics through a traceable import and parity process. Graphics use wgpu, as requested by Stephen on 15 November 2026. The existing Swift application continues its separate delivery and fleet work.

Status: the original SH028 station launch/docking slice is integrated in [PR 1](https://github.com/stephenlclarke/meridian/pull/1). Linux and native Windows pass workspace checks, all 46 Rust tests and strict Clippy. All nine final Python importer tests and exact original-source re-import pass; independent source review and the final PR SonarCloud check are clear. The new station executable has not started: the Dell interactive desktop is locked, so native station graphics, L-launch and player-controlled docking remain unverified. The previous Windows flight preview remains separately human-verified. Connected navigation/freight, economy and durable saves are still being implemented; the complete first playable is unfinished. The private GitHub repository is [stephenlclarke/meridian](https://github.com/stephenlclarke/meridian), cloned locally at `~/github/meridian`.

## Start here

- [Architecture](docs/ARCHITECTURE.md): ownership, module boundaries, graphics, persistence and failure handling.
- [First playable milestone](docs/FIRST-PLAYABLE.md): one complete journey and its acceptance criteria.
- [Legacy milestone breakdown](docs/LEGACY-MILESTONES.md): G3 source-grounded roadmap from the first playable to full mission and fleet parity.
- [Legacy reuse](docs/LEGACY-REUSE.md): exact reference revision, content provenance and mechanics parity.
- [Development and delivery](docs/DEVELOPMENT.md): machine roles, checks and release evidence.
- [Decision: Rust, wgpu and ownership](docs/decisions/0001-rust-wgpu-ownership.md).

Read [implementation status](docs/IMPLEMENTATION-STATUS.md) before running the prototype. When the execution lane is available, use `make check`, `make test`, `make clippy` and `make run`. Click inside the game window to give it keyboard focus. The station shell uses L to launch while docked, W/S for thrust/deceleration, Up/Down for pitch, A/D for yaw, Q/E for roll, Space for brake, P for pause, F11 for fullscreen and Escape to exit. Speed is displayed in reference gameplay units, not a qualified physical calibration.

Run `make docs-check` to lint the documents and `make assets-check` for headless Python import checks. The legacy milestone breakdown was prepared by the existing G3 session from immutable source. The full journey, native docking acceptance, economy, mission authority, saves and a packaged playable app remain in progress; the first milestone is not complete.
