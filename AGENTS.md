# Meridian agent rules

If `~/.agents/AGENTS.md` exists, read it and follow it.

Meridian is a fresh Rust project with direct wgpu graphics. Read the architecture and first-playable acceptance criteria before implementation. Keep changes inside this repository. Do not edit or import the human working checkout of Elite/L337.

Keep one owner of mutable simulation state. The core must not depend on a window, GPU, filesystem or wall clock. Use explicit commands, deterministic test inputs and immutable presentation snapshots. Do not create an actor per entity or introduce a general engine framework before a concrete requirement needs it.

Trace reused content to an immutable legacy revision and retain its provenance and notices. Do not treat presence in the old repository as proof of acceptance or commercial distribution permission. Do not copy live player saves or credentials.

The existing G1 session remains the sole current native/compiler/CI/save executor. Until that ownership changes explicitly, prepare source and tests without launching competing compilation, GPU jobs or new native processes. Keep the existing app delivery and MBA fleet work moving independently.

This repository now contains first-slice source and tests. Read docs/IMPLEMENTATION-STATUS.md for actual validation; do not report authored tests or proposed checks as passing. Follow Conventional Commits. Do not publish a remote or make an App Store submission merely because this repository exists.
