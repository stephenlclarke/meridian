# Pure docking geometry from the immutable reference

## Problem

Meridian's first playable needs real local flight and docking. The existing headless flight shell has no source-backed entrance frame, entry geometry or whole-mesh aperture predicate. Its navigation rings supply no station, collision or admission authority.

## Bounded implementation

Add a dependency-free pure docking module, its public module export, core documentation and meaningful headless tests. Use only immutable Elite/L337 `a11fe16924d87d06021816f9037952afcbac08db`. Preserve source coordinates, port dimensions, column orientation/local-roll order, strict versus inclusive bounds, full mesh projection and finite/refusal semantics. Keep World as the sole mutable authority. Do not edit flight, freight, renderer, Cargo or human Elite WIP; do not compile, run native/GPU jobs, create workers or commit during G1's current execution lease.

## Acceptance

One coherent source/test/reference/criteria handoff records exact source hashes, positive and negative source fixtures, independent orientation/clearance expectations and actual validation limits. G1 later runs compilation/tests/clippy on the integrated candidate. Real station assets, triangle/celestial/actor sweeps, current motion/source binding, admission, persistence and actual player-controlled launch-flight-dock remain separate first-playable acceptance work. Neither an authored test nor a true geometry predicate closes those requirements.

## Links

- [Implementation, references and criteria](PR-docking-geometry-primitives.md).
- [First playable](docs/FIRST-PLAYABLE.md).
