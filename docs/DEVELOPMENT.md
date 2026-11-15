# Development and delivery

## Parallel work

| Lane | Current role | Constraint |
| --- | --- | --- |
| G1, existing Mac execution owner | Current Swift app cold build, 548 baseline runtime cases plus one regression, app qualification and next playable delivery | Keep this priority and sole Mac native/compiler ownership |
| G4, existing MBA fleet owner | Continue original fleet source implementation and meaningful controls | Stay on its existing isolated branch; no duplicate fleet authoring |
| G3, existing source session | Source-grounded Meridian reimplementation milestone breakdown | Read immutable reference only; no competing compilation or native work |
| Dell provisioning/build owner | Meridian Linux and native Windows toolchains and exact-source validation | Keep jobs on the Dell; separate Linux and Windows checkouts and build outputs |
| Root | Meridian implementation, first-playable criteria, reference provenance and coordination | No competing Mac native jobs; preserve current app priority |

The Meridian plan does not cancel or relabel the current Swift delivery. The verified cold build started on 15 November 2026; runtime cases were still unrun at that observation. Later run status belongs in the existing coordination evidence rather than being inferred from this static plan.

## Machines

The Dell Latitude 7420 is configured with Ubuntu 22.04.5 LTS in WSL2 and native Windows Rust MSVC builds. Both use the repository's pinned Rust 1.99.0 with rustfmt and Clippy. MSVC 19.44 and Windows SDK 10.0.26100.0 are installed. Native Rust executable linking, execution and a filesystem roundtrip pass. Both platforms pass Meridian's check, 33 tests and strict Clippy at the exact revision recorded in [implementation status](IMPLEMENTATION-STATUS.md). GPU adapters, presentation and Windows packaging/input remain unqualified.

From the Mac, `ssh latitude` opens Ubuntu through the native Windows SSH bridge; `ssh latitude-windows` opens Windows. Both routes verify pinned host keys. Windows and WSL can also connect back through their separate dedicated SSH keys. Credentials and private key material do not belong in this repository.

Linux checkout: `/home/sclarke/github/meridian`. Native Windows checkout: `C:\Users\steph\github\meridian`. Both track the existing private GitHub repository. Keep Linux work under the Linux filesystem and Windows build outputs in the Windows checkout; do not share a Cargo target directory across platforms.

In Ubuntu, enter the checkout and use `make check`, `make test` and `make clippy`. In a Windows Developer PowerShell with the MSVC environment available, enter the Windows checkout and run:

```powershell
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

These commands compile and test without launching the game renderer. `cargo run --locked -p meridian-desktop` starts the prototype and needs an explicitly owned graphics validation session. Preserve actual backend and hardware evidence separately from headless test results.

The current MBA keeps its fleet work. The offered 16 GB M2 MacBook Pro remains unavailable until Stephen says otherwise. Allocate Rust compilation and GPU qualification explicitly when hardware and existing ownership permit it; do not launch additional jobs merely because the new repository exists. Apple signing, packaging and Mac App Store checks require a qualified Mac delivery lane.

## Planned implementation checks

Keep repository Makefile targets for formatting, cargo check, unit/integration tests, Clippy and asset validation; add headless journey replay, actual GPU smoke tests and packaged-app qualification as those systems become executable. Pin toolchain/dependencies and keep Cargo.lock. Run meaningful regression and property tests for transactions, save validation, identities, bounded queues and numeric invariants. The Makefile implements docs-check, assets-check, fmt/fmt-check, check, test, clippy, ci and run. Current Dell compiler/test evidence is distinct from Mac qualification, actual gameplay and release checks.

Aim for at least 90 percent focused new-code line coverage per shared guidance, with evidence-backed exceptions. Coverage is not mission acceptance. Add sanitizers or interpreter checks appropriate to owned unsafe/platform code when introduced. Qualify each shipped OS/backend on actual hardware and preserve failures. Required checks must pass for the exact final source revision; do not treat the Swift project's existing coverage or acceptance as Meridian evidence.

## Release progression

1. Source and tests authored, with content provenance.
2. Headless core and asset checks pass.
3. Actual renderer/input and complete journey pass.
4. Save, restart, failure and performance checks pass in the packaged app.
5. Signing, sandbox, distribution, notices and final-head checks pass for the intended channel.
6. User receives the identified app with limitations and evidence.

Do not add runtime downloads of executable gameplay logic to simplify the first release. Package the first playable content. Store platform persistence and distribution requirements as explicit adapter tests. Commercial release scope, final licensing, support policy, accessibility and minimum system requirements remain product work to complete; no App Store submission is authorized by merely creating this repository.
