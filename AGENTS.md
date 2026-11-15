# Meridian agent rules

If `~/.agents/AGENTS.md` exists, read it and follow it.

Meridian is a fresh Rust project with direct wgpu graphics. Read the architecture and first-playable acceptance criteria before implementation. Keep changes inside this repository. Do not edit or import the human working checkout of Elite/L337.

Keep one owner of mutable simulation state. The core must not depend on a window, GPU, filesystem or wall clock. Use explicit commands, deterministic test inputs and immutable presentation snapshots. Do not create an actor per entity or introduce a general engine framework before a concrete requirement needs it.

Trace reused content to an immutable legacy revision and retain its provenance and notices. Do not treat presence in the old repository as proof of acceptance or commercial distribution permission. Do not copy live player saves or credentials.

The existing G1 session owns the Mac native/compiler/CI/save lane for the current app delivery. The human separately authorized Meridian development and build validation on the Dell under Ubuntu WSL2 and native Windows MSVC. Keep Dell jobs on the Dell, coordinate one owner for each build target, and preserve exact source/result identity. Do not launch competing Mac compilation or GPU work. Fleet authoring remains on its existing MBA lane; blocked work must name the missing evidence without inventing activity.

This repository now contains first-slice source and tests. Read docs/IMPLEMENTATION-STATUS.md for actual validation; do not report authored tests or proposed checks as passing. Follow Conventional Commits. The user authorized storing this implementation in the public stephenlclarke/meridian GitHub repository, with the local checkout at ~/github/meridian. Keep the appropriate committed implementation synchronized with origin. App Store submission remains a separate release step.
