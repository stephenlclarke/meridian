# Development and delivery

## Parallel work

| Lane | Current role | Constraint |
| --- | --- | --- |
| G1, existing Mac execution owner | Current Swift app cold build, 548 runtime cases, app qualification and next playable delivery | Keep this priority and sole native/compiler ownership |
| G4, existing MBA fleet owner | Continue original fleet source implementation and meaningful controls | Stay on its existing isolated branch; no duplicate fleet authoring |
| G3, existing source session | Source-grounded Meridian reimplementation milestone breakdown | Read immutable reference only; no competing compilation or native work |
| Root | Meridian architecture, first-playable criteria, reference manifest design and coordination | No new workers, machine settings or competing native jobs |

The Meridian plan does not cancel or relabel the current Swift delivery. The verified cold build started on 15 November 2026; runtime cases were still unrun at that observation. Later run status belongs in the existing coordination evidence rather than being inferred from this static plan.

## Machines

The Dell Latitude 7420 has 16 GB RAM, Windows 11 and WSL, as reported by Stephen. Once charged and access is available, it can host Rust core development, headless Linux tests under WSL and offline content tooling. Native Windows execution is a distinct lane for the wgpu backend and Windows packaging/input; a WSL pass cannot qualify it. CPU/GPU details, toolchain and access are not yet verified. No installation or remote setup is performed by this planning slice.

The current MBA keeps its fleet work. The offered 16 GB M2 MacBook Pro remains unavailable until Stephen says otherwise. Allocate Rust compilation and GPU qualification explicitly when hardware and existing ownership permit it; do not launch additional jobs merely because the new repository exists. Apple signing, packaging and Mac App Store checks require a qualified Mac delivery lane.

## Planned implementation checks

When Rust code is introduced, provide repository Makefile targets for formatting, cargo check, unit/integration tests, clippy, headless journey replay, asset validation, actual GPU smoke tests and packaged-app qualification. Pin toolchain/dependencies and keep Cargo.lock. Run meaningful regression and property tests for transactions, save validation, identities, bounded queues and numeric invariants. The current planning-only Makefile implements docs-check only; no pretend green CI targets exist.

Aim for at least 90 percent focused new-code line coverage per shared guidance, with evidence-backed exceptions. Coverage is not mission acceptance. Add sanitizers or interpreter checks appropriate to owned unsafe/platform code when introduced. Qualify each shipped OS/backend on actual hardware and preserve failures. Required checks must pass for the exact final source revision; do not treat the Swift project's existing coverage or acceptance as Meridian evidence.

## Release progression

1. Source and tests authored, with content provenance.
2. Headless core and asset checks pass.
3. Actual renderer/input and complete journey pass.
4. Save, restart, failure and performance checks pass in the packaged app.
5. Signing, sandbox, distribution, notices and final-head checks pass for the intended channel.
6. User receives the identified app with limitations and evidence.

Do not add runtime downloads of executable gameplay logic to simplify the first release. Package the first playable content. Store platform persistence and distribution requirements as explicit adapter tests. Commercial release scope, final licensing, support policy, accessibility and minimum system requirements remain product work to complete; no App Store submission is authorized by merely creating this repository.
