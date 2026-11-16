# Meridian core

`World` owns mutable simulation state. Explicit commands and fixed ticks update flight, physical docking, travel/fuel, shared account/equipment, finite markets, G02 freight, fitted weapons, practice and missile actors. Presentation uses immutable snapshots. The core has no window, GPU, filesystem or wall-clock dependency.

`save_bytes`, `from_save_bytes` and `restore_bytes` validate a complete candidate before publication. The desktop owns file custody and atomic writes. Current source writes complete version 8 missile sessions and retains exact supported untouched version 4-7 readers. Candidate 58f45c8 passed 193 top-level Rust tests and scoped native Metal cold continuation. The earlier installed version 7 preview still refuses saving/restoring after missile use; a separate version 8 installed delivery remains pending.

Use workspace `make check`, `make test` and `make clippy` in the assigned execution lane. [Implementation status](../../docs/IMPLEMENTATION-STATUS.md) records exact tested source and native acceptance. Authored tests and headless results do not qualify gameplay.

Read [architecture](../../docs/ARCHITECTURE.md), [navigation](../../docs/WORLD-NAVIGATION.md), [save ownership](../../docs/WORLD-SAVE-CONTINUE.md), [finite missiles](../../docs/FINITE-STARTER-MISSILES.md) and [original content provenance](../../docs/LEGACY-REUSE.md) for contracts and limits. Imported original content retains its [notice](../../assets/legacy/Elite-L337-MIT.txt). Display meshes do not grant collision fidelity or fleet acceptance.
