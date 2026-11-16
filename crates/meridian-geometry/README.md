# Meridian exact facet geometry

This crate ports original-component source triangles, exact finite plane sections, within-facet exposed regions and certified contact-star checks into Rust. It takes deterministic owned inputs and returns an immutable audited candidate or a typed refusal. `OriginalAsset` binds the five unresolved original input sets by complete byte equality with compiled assets. Source decoding and reconstruction are explicit operations.

The source adapter preserves transformed component and local facet identities, exact coordinates, original winding, material/texture style and per-corner affine UVs. Construction clips finite constraints to each original triangle, builds canonical vertical strips, nodes physical contacts and shared boundaries, triangulates positive cells while keeping vertices, and verifies mutual exact region coverage, complete physical interval coverage, reciprocal incidence, connected vertex links and positive volume. Output triangles resolve their complete affine field from one original facet.

The original upper bounds are 2,000,000 charged operations, 100,000 logical storage memberships, 4096 bits per source or emitted coordinate/UV rational, and 512 complete candidate triangles. Callers can tighten these guards. The initial Rust implementation uses conservative live-record reservations and all-source traversal. It retains canonical strips and cell triangulation. The Python reference's constrained diagonal-flip minimizer and BVH caches remain reference material for later source-grounded work. These choices can cause earlier refusals under the same caps.

The previous five-hull execution at `0c2677b5` exhausted operations on every original, leaving 15 constructor-dependent tests unrun. Its exact result, input source pins, prior complete cost assertions and unmet cost attribution remain in `fixtures`. The fleet baseline is 23/162 ships and 4/6 stations. This crate issues geometry candidates. Physical registration, engineering stock, source permissions and native acceptance remain consumer responsibilities.

## Validation

The current two-lint correction is **UNRUN**. Parent joined source `de4e3bf` passed locked check and all 13 Rust tests, including both original-input decoder tests. Doc-tests declared zero cases. Strict Clippy failed on fixed-size byte chunking and a collapsible nested condition. The correction uses fixed two-byte array chunks behind the existing even-length guard and a short-circuiting let-chain for the same strip-finalization path. The author lane used rustfmt to parse and format the authored source. G1 owns actual compiler execution on the committed packet. The first compiler run should apply `integration/workspace-and-lock.patch` in the joined workspace and run the following checks.

```sh
cargo check --locked -p meridian-geometry --all-targets
cargo test --locked -p meridian-geometry
cargo clippy --locked -p meridian-geometry --all-targets -- -D warnings
```

Tests cover independent analytic box volumes, buried/overlapping/disconnected/shared-face originals, exact affine transforms, UV provenance, all 27 signed section patterns, point and vertical contacts, source and witness corruption, open/nonmanifold inputs, exact work/storage/rational/triangle boundaries, byte-bound original decoding and conservative dense-fixture refusal. The five original decoders are exercised without invoking their union constructors. Whole-five construction remains held for G2 whole-source review and explicit execution coordination.

## Source and dependencies

`PROVENANCE.json` records the immutable Swift snapshot, the sealed original source revision, retained references and deliberate Rust port differences. `fixtures/ORIGINAL-BINDINGS.json` contains all eight source pins per hull and exact export hashes. `tools/export_bound_originals.py` authenticates those pins with the genuine recipe's `authenticate_inputs` before exporting exact input records. It never calls the union constructor.

The crate pins num-bigint 0.4.8, num-rational 0.4.2 and num-traits 0.2.19. Published archive checksums, declared licenses and upstream Cargo manifests were checked. All MIT and Apache notices for these packages and num-integer 0.1.46 are retained in `third-party`. The root lock and workspace edits are supplied as a patch for serial integration. Dependency resolution remains UNRUN until the compiler owner checks the exact joined workspace.

## Incoming witness storage

Standalone `FacetWitness::verify` admits the checked aggregate of all incoming cells, corners, triangles, physical segments, protected points, contact-star records and sectors. That reservation remains live through scratch validation and releases on success or error. Construction reserves final geometry and each final witness once before publication and reuses that admission for its internal validation. A direct public regression covers the 40-segment/100-slot finding, aggregate nested cells/stars, exact and one-short scratch bounds, release on error, and retained final construction ownership. The authored regression remains UNRUN until G1 executes the corrected joined source.

## Protected local facet contacts

An original triangle can contribute two disjoint exposed regions that meet at one physical protected point. The overlap fixture reaches that case on component 0, facet 9. The corrected local certificate retains every sector and checks each connected link component as a regular boundary path or cycle at protected points. Unprotected local vertices and the final closed 3D mesh retain their connected-link requirements. The new source-derived regression also checks that removing physical protection refuses the local shape and that the same disconnected sectors refuse as a complete closed-mesh link. Existing analytic volume, source-field, witness and invalid-input tests are retained byte for byte.
