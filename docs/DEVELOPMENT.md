# Development and delivery

## Ownership and validation

Rust is the core product. G1 is the sole Mac compiler, native Metal, controls, packaging and save/install owner. Root authors/integrates Rust and audits immutable source/result identity after actual execution starts. Dell owns Linux and native Windows compiler/tests/Clippy/build/portability. G3 authors core source; MBA/G4 retains the original fleet on its existing branch. Assignment alone is not an active execution result. Keep source, test and actual-game acceptance distinct.

The core must build independently of GPU/window/filesystem/clock adapters. Use deterministic meaningful regressions and transactional refusal tests. Keep original assets, reference identities, notices and acceptance requirements intact. Never edit/import the human working Elite checkout or copy real player profiles.

## Commands

Use the pinned Rust toolchain and Cargo.lock. `make ci` runs documentation lint, formatting, Python importers, workspace check/tests and strict Clippy. `make run` launches the desktop app in an owned graphics session. Coverage reports and SonarQube are part of GitHub CI. Aim for at least 90 percent focused new-code coverage with explicit evidence-backed exceptions; coverage is not gameplay acceptance.

Install the pinned scientific test dependencies in the active Python environment before running `make ci` or discovering the Python tests.

```bash
python -m pip install --only-binary :all: -r tools/requirements-goshawk-surface-fit.txt
```

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

Preview publication runs asynchronously in the existing release session. G1 and Dell retain their packaging and compiler responsibilities; the release operator handles tags, uploads, download verification, receipts and retention. Root continues functionality and integration without waiting for network publication. Each release remains bound to its qualified immutable source; a later main commit does not change an in-flight release. See [preview release ownership](PREVIEW-RELEASES.md).

Coverage artifacts retain their original measurements. `make coverage-paths` creates `coverage/python-sonar.xml` from the raw Python report and binds filenames to the current checkout across Cargo analysis modules. CI uses absolute workspace report paths for module sensors. Verify actual per-file coverage attribution and open findings as well as the overall gate result; an ignored small-change coverage condition does not demonstrate a missing import is fixed. Preserve Previous version and exact-main SHA analysis settings.

The Linux coverage job also launches the instrumented desktop under Xvfb with Mesa software Vulkan. It checks that an empty slot opens Commander details before writing a save, then drives System, Trading, Back to HUD, and normal quit through X11. It boots a second ordinary process in the same slot and checks that the first Commander frame matches the fresh frame and the save bytes are unchanged before quit. The job requires the expected presented frames, a decoded isolated starter save, clean exits, and native coverage hits in the viewport, scale, and GPU upload adapters. The coverage artifact contains the captures, backend and process receipt, and the app-only LLVM report alongside the merged unit and native report used by SonarQube.

The same Linux job then opens a separate instrumented process with `--internal-mule-validation dock`. It checks the paused opening and the changed hull region after single-step controls, requires a normal exit with a presented-draw timing aggregate, and verifies that the internal route creates no profile directory. Its own LLVM profile must cover the production Mule texture upload, uniform update and both draws. It must also cover at least 80 percent of the executable GPU source lines, with a floor based on the 267 lines missed by the original main analysis. Both profiles feed the workspace report. The ordinary journey's native boundary checks use its own profile, and the explicit offscreen Metal test remains a separate qualification.

The coverage helpers read build and report files at fixed checkout paths and reject symlink redirection. Python coverage combines the two helpers' actual Linux runs with host tests for path validation, native coverage marker checks, and failure evidence.

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
6. Publish qualified macOS and Windows packages as a tagged GitHub prerelease using the [preview release workflow](PREVIEW-RELEASES.md). Derive its version from Conventional Commits. Verify downloadable bytes and keep only the newest three preview releases and their tags.

Keep earlier installed previews and real profiles unchanged. Signing/local functional delivery does not establish notarization, commercial distribution or full first-playable acceptance. [Implementation status](IMPLEMENTATION-STATUS.md) records the latest qualified scope.

When a committed-main coverage gate fails, preserve its exact source, original changed-line set and raw measurements. A correction must exercise the missing behavior and recheck that original set against actual new coverage; a subsequent Previous version baseline alone is not evidence of a fix. Keep unreachability and unobserved host boundaries explicit rather than inventing gameplay states or changing assertions.
