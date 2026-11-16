# Development and delivery

## Ownership and validation

Rust is the core product. G1 is the sole Mac compiler, native Metal, controls, packaging and save/install owner. Root authors/integrates Rust and audits immutable source/result identity after actual execution starts. Dell owns Linux and native Windows compiler/tests/Clippy/build/portability. G3 authors core source; MBA/G4 retains the original fleet on its existing branch. Assignment alone is not an active execution result. Keep source, test and actual-game acceptance distinct.

The core must build independently of GPU/window/filesystem/clock adapters. Use deterministic meaningful regressions and transactional refusal tests. Keep original assets, reference identities, notices and acceptance requirements intact. Never edit/import the human working Elite checkout or copy real player profiles.

## Commands

Use the pinned Rust toolchain and Cargo.lock. `make ci` runs documentation lint, formatting, Python importers, workspace check/tests and strict Clippy. `make run` launches the desktop app in an owned graphics session. Coverage reports and SonarQube are part of GitHub CI. Aim for at least 90 percent focused new-code coverage with explicit evidence-backed exceptions; coverage is not gameplay acceptance.

On native Windows, use the MSVC developer environment:

```powershell
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python -B -m unittest discover -s tools -p 'test_*.py' -v
```

Keep Linux and Windows checkouts and Cargo target directories separate. Record native Windows Python separately from WSL Python. SSH uses the existing pinned host keys and credentials; never commit credentials or private key material.

## Review and integration

Bundle implementation, meaningful tests, criteria, exact delta and execution preparation into one complete review. Use one existing independent reviewer at a time. Correct findings with a changed delta and retain unaffected proof. Preserve raw failures; do not rerun an unchanged failed candidate or weaken assertions.

Protect main through pull requests, current required GitHub checks, linear history and resolved conversations, with administrator enforcement and no force pushes/deletion. Retain independent source review. Publish SonarQube only from committed main using the exact 40-character SHA as projectVersion/scm.revision, verify Previous version policy and wait for its quality gate. The main-only Sonar job is a postmerge gate.

## Repository maintenance

Update README, implementation status and affected guides in the same change as behavior or acceptance. Keep a single current status page; identify historical contracts as historical. Replace stale pending/UNRUN wording when exact evidence exists, while preserving unresolved limits and failures. Keep PR descriptions aligned with their final scope and validation.

After each integration, remove the exact merged remote branch. For squash merges, verify the PR head SHA and merge before deleting; ancestry alone may not show integration. Retain active owner branches and unresolved work. Review open PRs/issues during the existing delivery cycle: update actual status, close genuine duplicates or completed/superseded items with evidence, and keep unresolved bugs open. Never close on age alone. Enable GitHub automatic deletion of merged PR branches as a backstop.

Remove obsolete ISSUE-*.md and PR-*.md handoff copies once their useful content is in current guides or GitHub records. Use Git history for closed implementation history. Keep temporary preparation receipts, local execution reports, screenshots, build products and player data out of the repo. Preserve source fixtures, content manifests, notices, licenses, lockfiles and unresolved acceptance requirements. Do not create replacement inventory documents or empty issue files to satisfy a process count.

## Release progression

1. Author and independently review exact implementation/tests and provenance.
2. Qualify the reviewed source on the owned compiler/platform lanes.
3. Qualify actual rendering/input, journey and cold continuation.
4. Merge with final-head protected checks, then verify exact-main CI/Sonar.
5. Package/sign and deliver the identified app with evidence and limits.

Keep earlier installed previews and real profiles unchanged. Signing/local functional delivery does not establish notarization, commercial distribution or full first-playable acceptance. [Implementation status](IMPLEMENTATION-STATUS.md) records the latest qualified scope.
