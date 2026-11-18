# Original reference import CLI

`meridian-import` is offline Rust conversion tooling for the two selected original references, SH025 and SH028. Its production binary has no dependency on World, desktop, GPU, player profiles or mission state. A test-only dependency on the current core exercises its real public station loader. Implementation and tests are authored; compiler, CLI and test execution remain **UNRUN pending whole-source review**. Existing Python or game results are not transferred to this crate.

## Commands

After review and designated compiler qualification:

```sh
cargo run --locked -p meridian-import -- ship --source /path/to/SH025.toml --output /existing/parent/new-ship
cargo run --locked -p meridian-import -- station --repository /path/to/original-object-database --output /existing/parent/new-station
cargo run --locked -p meridian-import -- all --ship-source /path/to/SH025.toml --station-source /path/to/SH028.toml --check assets/legacy
```

Use `--help` for the complete syntax. `ship` produces `sh025.mesh`. `station` produces `sh028.mesh`, `sh028.physics`, `sh025.physics` and `sh028.port`. `all` produces all five. Repository mode reads only the two fixed `a11fe16924d87d06021816f9037952afcbac08db:path` specifications with Git text conversion and external diff disabled. HEAD and working files are not inputs. File mode requires the same exact source sizes and SHA256 values before parsing.

`--check` reads an existing reference directory and verifies selected binary bytes, original Python provenance manifests and the original notice. It does not write, repair or replace files. `--output` requires a fresh directory whose parent already exists. Existing destinations refuse. All source validation and conversion finish before directory creation. A subsequent I/O failure returns failure and may leave a partial new directory for the operator to inspect; it does not replace an earlier asset directory. The Rust receipt is written through a flushed temporary after the binary outputs and notice, then renamed to its final name.

## Formats and parity contract

| Output | Format and unchanged contract |
| --- | --- |
| `sh025.mesh`, `sh028.mesh` | Original triangle/index order expanded to LE binary32 XYZ and linear RGB, 24 bytes per vertex; no UV/textures or alpha |
| `sh025.physics`, `sh028.physics` | `MPHYS001`, two LEu32 counts, LE binary64 XYZ, LEu32 triangle indices |
| `sh028.port` | `MPORT001`, ten LE binary64 center/outward/up/depth values, LEu32 aperture count and LE binary64 aperture XYZ |

Source coordinates, signed zero, triangle order, selected display/native LODs and station aperture are retained. The sRGB8 conversion uses a frozen 256-entry binary32 table generated from the maintained Python formula. This avoids platform `pow` differences while retaining its exact final bytes. Its independent table digest and recipe identity are in `fixtures/reference.json`. Every converted artifact must also match the already committed size/SHA256 before it can be returned or written.

The library validates finite bounded positions, triangle/material indices, required material assignments, exact selected IDs/names/counts, design/appearance declarations and finite selected port axes/depth/aperture. It does not triangulate, simplify, weld, repair, normalize coordinates, invent a port or certify a complex hull. Triangle degeneracy, manifoldness, exposed-region reconstruction and hull/fidelity admission remain outside this crate and with the separate geometry owner.

## Honest producer and source provenance

The original `assets/legacy/manifest.json` and `station-manifest.json` are immutable check fixtures for their Python recipes. They remain byte-exact in the repository. They are not emitted as a Rust producer record and their `importer` fields are not changed or aliased.

New output directories instead contain `rust-import-manifest.json` with producer `meridian-import`, its actual package version, fixed source revision/paths/sizes/SHA256, historical recipe/manifest references, exact output hashes/formats and a scoped exclusion of new hull/mission/permission admission. There are no timestamps or caller path strings in this deterministic receipt. `SOURCE-NOTICE.txt` is the exact retained original notice, not a new grant.

The two TOML fixtures were read from immutable Git objects in the designated cold a11 snapshot. Their original `profiles` sections are content definitions, not commander saves. `fixtures/git-fixture.json` binds the same two original blobs and the exact original commit/path trees. Its partial object database is sufficient only for selected fixed-path lookup tests; it is not a full historic clone. No human working Elite checkout or real player profile was read or imported.

## Meaningful tests and qualification

Authored tests compare all five conversions with the independent committed assets, exercise actual CLI file/repository modes and verify output/check/refusal behavior. A real isolated Git object database uses the pinned original commit/trees/blobs while HEAD is unborn and working files contain foreign bytes. Negative tests cover hash/identity changes, malformed UTF8/TOML/types/array arity, nonfinite/bounded geometry, indices/material assignments, port shape/frame, modified reference assets/manifests/notices, unknown arguments and existing output directories. The refreshed current-consumer integration test runs the actual station CLI into an isolated Unicode path, loads its three physical outputs through `OriginalStationData::from_bytes`, checks the independently pinned triangle counts and real launch setup, and requires every single-output corruption to refuse. `--check` corruption cases cover all five binary outputs as well as both manifests and the notice. `fixtures/current-consumers.json` records exact current source/API and independent existing binary fixtures. No expected asset is regenerated by the Rust implementation to make its test pass.

After whole-source CLEAR, run the exact committed source with the existing designated owners. Required execution includes workspace `make ci` or native Windows equivalents, strict Clippy, the importer unit/integration tests and a release build of the actual `meridian-import` binary. Preserve raw source/START/toolchain/log/terminal/binary bindings. CLI conversion qualifies offline formats and source custody; it does not qualify graphics, physical hull admission or gameplay.

## Dependencies and serial integration

The direct parser uses the workspace's already locked `toml_edit 0.25.13+spec-1.1.0` and Serde version. SHA256 and receipt serialization use exactly pinned `sha2 0.11.0` and `serde_json 1.0.154`. Official package metadata/checksums, source VCS identities, complete license texts and resolved dependency evidence are retained in `LICENSE-SOURCES.json` and `licenses/`; these are factual source/notice records, not a blanket distribution certificate.

Primary API references are [toml_edit](https://docs.rs/crate/toml_edit/0.25.13%2Bspec-1.1.0), [SHA-2](https://docs.rs/sha2/0.11.0/sha2/) and [serde_json](https://docs.rs/serde_json/1.0.154/serde_json/). Keep the existing dependency versions intact during workspace integration. [Cargo documents `update --workspace`](https://doc.rust-lang.org/cargo/commands/cargo-update.html) as retaining other locked packages unless a new package is needed.

This isolated branch proposes only the new crate plus its exact workspace member/lock delta. Root owns serial reconciliation with other authored crates. No core, App, maintained Python converter, runtime asset, fixture assertion, quality threshold or exclusion is replaced by this slice.
