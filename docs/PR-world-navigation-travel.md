# feat(core): add World navigation and physical destination binding

Fresh World profiles now own original Avenor/SH025 identity, the actual starter account and timed stock-drive travel. Avenor to Renas takes600 accepted ticks and charges34 fuel tenths once on arrival. Arrival remains in flight; destination admission is created only after the existing geometry checks admit an actual physical entry.

Original G01 coordinates retain exact source/output hashes and the MIT notice. Routes use stable minimum hops and explicitly assume later refuelling. Public snapshots remain descriptive. The private-field dock context binds owner, ship, system, main-orbital actor, visit, sequence and initial/physical origin; launch invalidates it. See [implementation and reference](WORLD-NAVIGATION.md) and [problem](ISSUE-world-navigation-travel.md).

Validation: all13 Python import tests and exact catalogue re-import pass; Rust formatting and whitespace pass. Seven new Rust tests are authored but UNRUN pending the existing Dell executor. Full joined Rust/Clippy, independent review, hosted checks and native UI acceptance remain pending. This is not save support or the complete playable freight journey. The original isolated scene's missing other actors/celestials, conservative collision and source parity limits remain.
