# GitHub preview releases

## Delivery policy

Publish produced, qualified Meridian previews on GitHub Releases for macOS and Windows only. Each preview has both portable platform ZIP archives, actual launch/save instructions, applicable license/provenance notices, SHA256SUMS.txt and preview.json with its exact source SHA and release version. G1 owns macOS packaging/signing; Dell owns Windows packaging. A separate existing release session, currently Researcher, owns publication, download verification, the actual GitHub receipt and retention cleanup. Root continues gameplay authoring and integration independently. Preserve installed previews and real profiles.

The macOS archive must contain a Developer ID Application signed app. Verify its extracted bundle with codesign; the publisher repeats that check. State notarization accurately. The current Apple agreement HTTP403 is not authorization to retry or accept terms. Windows code-signing trust is separate from compilation; do not claim an unsigned binary is signed or that build tests establish native gameplay acceptance. No certificate purchase or paid service enrollment is automatic.

The retained v0.5 Mac archive was reassessed on 17 November: strict deep Developer ID verification passes, while Gatekeeper reports `Unnotarized Developer ID` and stapler finds no ticket. Its signature is valid; re-signing alone does not remove this warning. Resolving the pending Apple developer agreement is the external prerequisite before a changed notarization attempt, ticket stapling and verification of a new immutable preview. Existing published archives and tags are preserved.

The first public preview is [v0.1.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.1.0-preview.1), with both platform archives, SHA256SUMS.txt and preview.json. All downloadable hashes and the extracted downloaded Mac signature were verified. Dependency notice coverage is factually reviewed for 80 Mac dependencies and the separate 99-package Windows set, including the canonical hexf CC0 text binding. Preserve original-input obligations, upstream author-file absence, attribution and the Apple SDK policy caveat; factual review is no blanket commercial/legal clearance. macOS is signed but not notarized; Windows is unsigned and interactive Windows gameplay remains unobserved.

## Automatic versions and tags

Use tools/preview_release.py through make preview-plan and make preview-release. It reads Conventional Commit messages between the latest published managed preview's exact source and the selected committed-main SHA. The first preview uses the workspace Cargo version with preview.1, currently v0.1.0-preview.1. Later breaking commits, including a breaking footer or ! marker, increment major; feat increments minor; fix/perf increments patch. Take the largest bump in the batch and reset the preview ordinal to one. Documentation and maintenance-only batches retain the base version and increment the preview ordinal. Release version lives in its tag and preview.json; this tool does not rewrite already signed app bundles or Cargo source versions.

Each release uses an annotated immutable tag such as v0.2.0-preview.1 pointing to the exact qualified committed-main source. Never move a published tag. The publisher verifies remote-main ancestry and successful exact-source checks, including main Sonar, before publication. The first version is a source version, not a percentage or promise of completed gameplay.

Plan without publication:

```sh
make preview-plan PREVIEW_COMMIT=EXACT_40_CHARACTER_MAIN_SHA
```

After owner packaging, independent review, qualification and notice resolution, prepare one directory under `~/.local/share/meridian-preview-releases/`, for example `durable-v8`. It must contain `Meridian-macos-arm64.zip`, `Meridian-windows-x64.zip` and `RELEASE-NOTES.md`. Copy only the identified qualified packages and public notes into that directory. Package identifiers permit lowercase letters, digits and hyphens. Symlinked directories/files and paths outside release staging refuse. The publisher writes its new `release-receipt.json` there with exclusive creation; it never accepts arbitrary read/write paths from CLI arguments.

```sh
make preview-release PREVIEW_COMMIT=EXACT_40_CHARACTER_MAIN_SHA PREVIEW_PACKAGE=durable-v8
```

The release operator runs from an isolated release checkout on macOS with gh authenticated for stephenlclarke/meridian and the source history available locally. Prefer the MBA when its existing authentication is valid; use an already authenticated host in the same separate release session if MBA authentication is unavailable. Never transfer credentials to bypass an invalid login. No signing keys are uploaded by this script. Use reviewed packages; never upload player profiles or local execution reports. The runner permits only the fixed release executables. CLI and remote Git SHAs are validated and reconstructed before entering argv; workspace versions are numeric SemVer. Metadata files open fixed generated paths before writing notes as data. The release source SHA can differ from the publisher's newer documentation/tooling commit; record both in ordinary delivery evidence.

## Asynchronous publication and three-preview retention

Platform owners hand the release operator complete, source-bound package manifests and signing/notice evidence. The operator verifies those inputs and publishes without waiting for another Root acknowledgement once the existing qualification requirements pass. Publication stays bound to its selected qualified main SHA while development main advances. Uploads, download checks and retention cleanup do not block later feature authoring or compiler/native validation. Release failures stay with the release operator and are reported with actual evidence. No second publisher, new session or scheduler is created.

The publisher creates the annotated Git tag and a draft prerelease, uploads the two archives plus checksums and source metadata, downloads every asset and verifies its SHA256, then publishes the prerelease. It verifies downloads again and durably saves the actual receipt before retention cleanup. A failed signature, exact-main check or download verification refuses progression. A failed draft/upload remains an explicit partial publication to inspect and correct; do not silently overwrite or move its tag.

Only the newest three published managed previews are retained, ordered by publication time. After successful new publication and receipt capture, remove older managed preview releases with their matching preview tags. Never delete stable releases, unrelated tags, drafts, active branches or local worktrees through this rule. There were no releases or tags at the initial inspection, so no historical cleanup was needed. This runs within the existing delivery cycle; it adds no scheduler or compiler/graphics job.
