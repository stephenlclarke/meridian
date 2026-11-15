# Public repository and CI-based SonarQube analysis

The repository and existing SonarQube Cloud project are public. The user selected proprietary/source-visible licensing for new Meridian code; retained asset/dependency licenses remain separate.

## Implementation

Add least-privilege GitHub Actions for cross-platform Rust checks/tests/strict Clippy, Python importer tests, Markdown lint and coverage reports. Pin actions by exact commit, toolchain and report tools by version. Only main receives the Sonar secret and publishes analysis. Verify the remote Previous version policy, use the exact main SHA and wait for the quality gate. Import Rust LCOV, Clippy JSON and Python coverage without production-code coverage exclusions. Add live build and quality badges plus truthful license/toolchain/target badges.

## Validation and limits

Workflow lint, documentation lint and existing importer tests are checked before pushing. A measured 32% Python importer baseline led to eight additional conversion/CLI/malformed-input tests; all 17 importer tests pass locally. These use temporary synthetic geometry only, never packaged content, and preserve every production source hash check. Hosted runs and the first exact-main Sonar analysis must complete before this setup is reported as operational. Existing Linux/Windows gameplay test evidence remains separate; the locked Dell native UI remains unqualified. Asset provenance is audited independently; a license label is not authorship proof. No runtime, mission or asset geometry code changes in this slice.
