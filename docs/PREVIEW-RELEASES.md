# GitHub preview releases

## Local Mac Preview 9

Open `Meridian HUD and Mule Preview.app` in the `Meridian Preview 9` folder under your user Applications folder. Apple notarized this build. The extracted archive and installed app passed signature, ticket and Gatekeeper verification. Its save directory is `Meridian Preview 9/saves-v17-0962f14` under Application Support. The GitHub prerelease is pending.

## Current preview

[Preview 8 v0.8.0-preview.1](https://github.com/stephenlclarke/meridian/releases/tag/v0.8.0-preview.1) contains the paid EMP utility and retains contextual Help and XYZZY testing for macOS arm64 and Windows x64. It was published on 18 November 2026 at 07:26:13 UTC. Protected main is `bf31ab72f2e4845b8963219cf9e507da6da090de`; compiled source is `2d5f4a57ae05db68ec7f233f9b695f3ecc4b2f75`. All 300 committed files and modes have the same tree `16391c8e179353703b32ddb8d65e69f361f51dcd`.

All six exact-main checks passed, with Sonar revision/project version bound to that main, zero unresolved findings and 98.8% new coverage. All four anonymous public downloads, full package bytes/modes, all raw Mac ZIP metadata and the immutable annotated tag were verified. The actual release receipt was flushed before removing only v0.5 and its tag. Retained previews are v0.8, v0.7 and v0.6.

| Public archive | Bytes | SHA-256 |
| --- | --- | --- |
| [macOS arm64](https://github.com/stephenlclarke/meridian/releases/download/v0.8.0-preview.1/Meridian-macos-arm64.zip) | 4,617,820 | `55e5a6897a8486764d72159ed66010b71e7c4761865a439d48f8053f62c001f2` |
| [Windows x64](https://github.com/stephenlclarke/meridian/releases/download/v0.8.0-preview.1/Meridian-windows-x64.zip) | 4,510,571 | `a54073e6ba3aa5be67c7c74c536edc0e2593d061ebceb12f8a080ecc9966b02e` |

[Checksums](https://github.com/stephenlclarke/meridian/releases/download/v0.8.0-preview.1/SHA256SUMS.txt) and [source metadata](https://github.com/stephenlclarke/meridian/releases/download/v0.8.0-preview.1/preview.json) are the other two verified assets. Release ID is `449647335`; annotated tag object is `5f6c10aa67b35f5eface7a5e6e0078df18256ba1`. The operator's actual 16,317-byte release receipt has SHA-256 `cee41df160053e886c1142b75b9e7a59b767adc1b8545a67c2b5edec482567dd`. This identifies the real saved receipt; local execution files are not additional downloads.

The Mac app passed strict extracted app/game/launcher signatures, stapler validation and Gatekeeper assessment as Notarized Developer ID. Apple accepted new EMP job `e0b916de-abcd-4d2e-99b9-770fc41bdcc7`; its ticket is stapled. The canonical app preserves all 453 original files and modes and adds only the 1,674-byte ticket. No older ticket was reused. The outer wrapper has 474 payload files; its 1,465 raw ZIP entries include 732 AppleDouble metadata files consumed by ditto. Windows has 331 files, an unsigned executable and no bundled DLLs. Its external [Microsoft Visual C++ x64 runtime](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) is required; interactive Windows gameplay remains unobserved.

Extract the complete folder. Open `Meridian EMP Sonar Preview.app` on macOS or `Launch Meridian.cmd` on Windows. Mac saves use `Meridian EMP Sonar Preview/saves-current-emp-2d5f4a5` under Application Support; Windows uses adjacent `saves-emp-2d5f4a5`, empty in the download. No real commander profile is packaged or migrated. The new binary's bounded native smoke and earlier full `5e3e98a` EMP journey remain separate evidence; see [implementation status](IMPLEMENTATION-STATUS.md).

## Delivery policy

Publish produced, qualified Meridian previews on GitHub Releases for macOS and Windows only. Each preview has both portable platform ZIP archives, actual launch/save instructions, applicable license/provenance notices, SHA256SUMS.txt and preview.json with its exact source SHA and release version. G1 owns macOS packaging/signing; Dell owns Windows packaging. A separate existing release session, currently Researcher, owns publication, download verification, the actual GitHub receipt and retention cleanup. Root continues gameplay authoring and integration independently. Preserve installed previews and real profiles.

The macOS archive must contain a Developer ID Application signed app. Verify its extracted bundle with codesign; the publisher repeats that check. Notarization is specific to the submitted artifact. For Preview 8, verify the actual new accepted job and stapled ticket on the final downloaded app; an older acceptance cannot qualify new code. Historical agreement HTTP403 and status HTTP401 observations do not describe the completed Preview 8 submission. No unchanged retry or acceptance of legal terms is implied. Windows code-signing trust is separate from compilation; do not claim an unsigned binary is signed or that build tests establish native gameplay acceptance. No certificate purchase or paid service enrollment is automatic.

The former v0.5 Mac archive was reassessed on 17 November: strict Developer ID verification passed, while Gatekeeper reported Unnotarized Developer ID and stapler found no ticket. Those historical results remain attached to that older artifact. Only its managed release and tag were removed after verified Preview 8 publication and a durable receipt; retained v0.7 and v0.6 archives/tags were not rewritten or re-signed. The new EMP acceptance does not retrospectively notarize older archives.

The first public preview was v0.1.0-preview.1, with both platform archives, SHA256SUMS.txt and preview.json; it was later removed under the three-preview retention rule. All downloadable hashes and the extracted downloaded Mac signature were verified. Dependency notice coverage is factually reviewed for 80 Mac dependencies and the separate 99-package Windows set, including the canonical hexf CC0 text binding. Preserve original-input obligations, upstream author-file absence, attribution and the Apple SDK policy caveat; factual review is no blanket commercial/legal clearance. That first Mac archive was signed without the new EMP ticket; Windows was unsigned. Current artifact-specific notarization is recorded above, while Windows interactive gameplay remains unobserved.

## Automatic versions and tags

Use tools/preview_release.py through make preview-plan and make preview-release. It reads Conventional Commit messages between the latest published managed preview's exact source and the selected committed-main SHA. The first preview uses the workspace Cargo version with preview.1, currently v0.1.0-preview.1. Later breaking commits, including a breaking footer or ! marker, increment major; feat increments minor; fix/perf increments patch. Take the largest bump in the batch and reset the preview ordinal to one. Documentation and maintenance-only batches retain the base version and increment the preview ordinal. Release version lives in its tag and preview.json; this tool does not rewrite already signed app bundles or Cargo source versions.

Each release uses an annotated immutable tag such as v0.2.0-preview.1 pointing to the exact qualified committed-main source. Never move a published tag. The publisher verifies remote-main ancestry and successful exact-source checks, including main Sonar, before publication. The first version is a source version, not a percentage or promise of completed gameplay.

Plan without publication:

```sh
make preview-plan PREVIEW_COMMIT=EXACT_40_CHARACTER_MAIN_SHA
```

After owner packaging, independent review, qualification and verified retained notice bindings for authorized internal testing, prepare one directory under `~/.local/share/meridian-preview-releases/`, for example `durable-v8`. It must contain `Meridian-macos-arm64.zip`, `Meridian-windows-x64.zip` and `RELEASE-NOTES.md`. Copy only the identified qualified packages and public notes into that directory. Package identifiers permit lowercase letters, digits and hyphens. Symlinked directories/files and paths outside release staging refuse. The publisher writes its new `release-receipt.json` there with exclusive creation; it never accepts arbitrary read/write paths from CLI arguments.

```sh
make preview-release PREVIEW_COMMIT=EXACT_40_CHARACTER_MAIN_SHA PREVIEW_PACKAGE=durable-v8
```

The release operator runs from an isolated release checkout on macOS with gh authenticated for stephenlclarke/meridian and the source history available locally. Prefer the MBA when its existing authentication is valid; use an already authenticated host in the same separate release session if MBA authentication is unavailable. Never transfer credentials to bypass an invalid login. No signing keys are uploaded by this script. Use reviewed packages; never upload player profiles or local execution reports. The runner permits only the fixed release executables. CLI and remote Git SHAs are validated and reconstructed before entering argv; workspace versions are numeric SemVer. Metadata files open fixed generated paths before writing notes as data. The release source SHA can differ from the publisher's newer documentation/tooling commit; record both in ordinary delivery evidence.

## Asynchronous publication and three-preview retention

Platform owners hand the release operator complete, source-bound package manifests and signing/notice evidence. The operator verifies those inputs and publishes without waiting for another Root acknowledgement once the existing qualification requirements pass. Publication stays bound to its selected qualified main SHA while development main advances. Uploads, download checks and retention cleanup do not block later feature authoring or compiler/native validation. Release failures stay with the release operator and are reported with actual evidence. No second publisher, new session or scheduler is created.

The publisher creates the annotated Git tag and a draft prerelease, uploads the two archives plus checksums and source metadata, downloads every asset and verifies its SHA256, then publishes the prerelease. It verifies downloads again and durably saves the actual receipt before retention cleanup. A failed signature, exact-main check or download verification refuses progression. A failed draft/upload remains an explicit partial publication to inspect and correct; do not silently overwrite or move its tag.

Only the newest three published managed previews are retained, ordered by publication time. After successful new publication and receipt capture, remove older managed preview releases with their matching preview tags. Never delete stable releases, unrelated tags, drafts, active branches or local worktrees through this rule. There were no releases or tags at the initial inspection, so no historical cleanup was needed. This runs within the existing delivery cycle; it adds no scheduler or compiler/graphics job.
