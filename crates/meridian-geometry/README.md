# Meridian exact facet geometry

This crate ports original-component source triangles, exact finite plane sections, within-facet exposed regions and certified contact-star checks into Rust. It takes deterministic owned inputs and returns an immutable audited candidate or a typed refusal. `OriginalAsset` binds the five unresolved original input sets by complete byte equality with compiled assets. Source decoding and reconstruction are explicit operations.

The source adapter preserves transformed component and local facet identities, exact coordinates, original winding, material/texture style and per-corner affine UVs. Construction clips finite constraints to each original triangle, builds canonical vertical strips, nodes physical contacts and shared boundaries, triangulates positive cells while keeping vertices, and verifies mutual exact region coverage, complete physical interval coverage, reciprocal incidence, connected vertex links and positive volume. Output triangles resolve their complete affine field from one original facet.

The original upper bounds are 2,000,000 charged operations, 100,000 logical storage memberships, 4096 bits per source or emitted coordinate/UV rational, and 512 complete candidate triangles. Callers can tighten these guards. The Rust implementation uses conservative live-record reservations, component bounds and closed facet-box rejection. It retains canonical strips and cell triangulation. The Python reference's constrained diagonal-flip minimizer and hierarchical face/ray BVH remain reference material for later source-grounded work. These choices can cause earlier refusals under the same caps.

The previous five-hull execution at `0c2677b5` exhausted operations on every original, leaving 15 constructor-dependent tests unrun. Its exact result, input source pins, prior complete cost assertions and unmet cost attribution remain in `fixtures`. The fleet baseline is 23/162 ships and 4/6 stations. This crate issues geometry candidates. Physical registration, engineering stock, source permissions and native acceptance remain consumer responsibilities.

## Validation

The current exact-pruning correction and its two new Rust regressions are **UNRUN**. Parent joined source `1287511` passed locked check, all 21 Rust tests, nine host tests, strict Clippy and executable build on Dell Linux. The actual original-five run returned three operation refusals and two storage refusals after all five decodes passed. All 15 dependent bodies remained UNRUN and zero candidates returned. The author lane used rustfmt to parse and format the authored source. G1 owns actual compiler execution on the committed packet. The first compiler run should apply `integration/workspace-and-lock.patch` in the joined workspace and run the following checks.

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

## Executable original-five validation

`cargo build --locked -p meridian-geometry --bin original_five` builds the explicit host harness. It uses the same pinned `OriginalAsset` and public `reconstruct` adapter for SH010, SH011, SH016, SH017 and SH166. Each hull declares five bodies for input decoding, construction, source/affine fields, facet certificates and closed topology/limits. A refused construction leaves its three dependent candidate bodies UNRUN. Original operation, storage, rational and triangle guards apply to the whole case. Dependent checks receive only the remaining original work/storage allowance.

`tools/run_original_five.py` accepts the frozen joined workspace, its Root source manifest, the owner-built binary and a new output directory. It verifies source/lock/input hashes before launch, records the actual process/binary/source binding, checks the five-case result protocol and seals every output with SHA-256. The Rust host records monotonic timing, per-body operation deltas/terminal peaks and typed refusals. Complete returned candidates retain all exact triangle/source/style/UV data and full facet/contact witnesses. Files are labelled UNQUALIFIED-CANDIDATE because fleet/native admission remains a later consumer decision.

```sh
python3 crates/meridian-geometry/tools/run_original_five.py --project FROZEN_JOIN --source-binding ROOT_SOURCE_JSON --binary BUILT_ORIGINAL_FIVE --out NEW_ABSOLUTE_OUTPUT
```

Execution remains held for bounded G2 review and the G1/Root changed-run coordination. `integration/WHOLE-FIVE-CRITERIA.json` defines the 25 construction/dependent bodies, output binding and compiler/host checks. The six Rust harness regressions use typed failure state, residual budgets, JSON escaping and a small independent tetrahedron. Nine host tests cover source and terminal binding, mutation/escape refusal and dependency propagation. Existing library tests and qualified geometry source remain byte exact.

## Typed harness outcomes

The same classification applies at decoding, construction and dependent checks. Operations, Storage, Rational and TriangleLimit failures report REFUSED. InvalidInput, Degenerate, ConflictingFields, NonManifold, UncertifiedContact and InvalidWitness report FAIL. Both outcomes stop dependent bodies with UNRUN. Host validation requires the typed failure, checks its kind against the body status and rejects missing/unknown failures or semantic failures presented as guard refusals. The current correction and its regressions remain UNRUN.

## Original-component audit lifetime

The corrected input audit borrows and audits one complete original component at a time while retaining all source facets. It releases that component's topology maps and borrowed reference list before the next component. The per-facet reservation policy and every edge, signed-volume and connected-link check remain in force. All sheets of one component share an audit. The completed candidate still receives one global topology audit.

Storage output distinguishes the largest attempted logical reservation, the largest admitted reservation, and the retained budget ledger at the terminal stage. A refused attempted peak can exceed 100,000. These counters describe reservations and do not measure allocated bytes or a minimum storage requirement. `integration/component-audit/CAUSAL.json` binds the five actual refusal records to the exact source formula and records the proposed per-component reservation bounds. Current correction tests and whole-five execution are UNRUN.

## Exact conservative pruning

The corrected facet adapter builds a bounded, charged component-box map after original-source audits. It rejects strictly outside owners before occupancy ray traversal and rejects disjoint closed facet boxes before finite plane sections. Boundary equality remains eligible for face, edge and point contacts. The exact same-plane section result supplies the positive-normal coplanar candidates for complete field agreement and deterministic facet ownership. The box map releases before final noding.

A facet with no exposed classified cells now releases its temporary arrangement without retaining an empty region/witness record, matching the original witness publisher. Authenticated source inputs remain retained. Complete geometry and every exposed contact/field witness remain required. Pre-successor frozen analytic controls compare the full candidate and all exposed witness fields against the exact parent implementation, include public witness verification in charged totals, retain unfavorable small controls and require complete-work savings on disconnected solids. The actual savings and original outcomes are UNRUN.

## Shared original style storage

The qualified pruning parent passed 23 Rust tests, nine host tests, check, strict Clippy and the dev harness build. Its one changed whole-five run decoded all five inputs, then refused all five constructions on Storage, leaving 15 dependent bodies UNRUN and returning no candidate. Its complete identity control charged 30,792 operations against the earlier 30,277; that unfavorable result remains recorded. These counters do not establish physical performance or minimum memory.

The current correction shares exact immutable canonical style text within each original decode. Every original facet and its exact point/UV/style/provenance remains present. A private ownership variant preserves ordinary owned String behavior for the public constructor and output facets. Input accounting counts owned strings separately and shared allocations by actual allocation identity, including their two reference-count slots. Decoder pool handles, lookup work and input-accounting scratch remain guarded. No global pool or mutable shared state is introduced.

Each pinned asset contains six distinct styles with 228 text memberships. Sharing removes 38,216 repeated input memberships for SH010/SH011, 41,560 for SH016/SH017 and 37,304 for SH166. These are source-derived reservation changes, not measured allocator results or proof that later hull work fits. All original guards stay in force. The unchanged host harness also retains its conservative per-facet text upper bound for dependent checks.

The immediate parent certificate, source and decoder were frozen before this correction. Six complete independent analytic controls compare ordered exact geometry, volume and every facet/contact witness field, including public witness validation in both lanes. They require lower charged storage peaks and retain all work counters. Separate regressions check pinned style text, owned versus shared clones, equal text in distinct allocations, and the exact/one-short scratch boundary. New compiler tests and original-five construction remain UNRUN until bounded review and the owner-controlled changed run. See `integration/style-storage/DELTA.json` and `fixtures/style-storage-reference/FROZEN.json` for the evidence and criteria.

The first style-storage join passed 26 Rust tests and nine host tests, then failed strict Clippy on nine test unwraps and one fixed-size chunk iterator. No dev harness build or original-five run followed that failure. The bounded correction propagates oracle parse errors, preserves expected-error assertions and uses fixed-size chunks with an explicit empty-remainder assertion. Production geometry and frozen controls remain byte exact. Corrected checks and execution are UNRUN.

## Immutable facet bounds index

The qualified style-storage correction passed 26 Rust tests, nine host tests, check, strict Clippy and the dev harness build. One changed whole-five run decoded all inputs, then refused all five constructors on Operations with 15 dependent bodies UNRUN and no complete candidate. Earlier Storage failures and increased identity-control work remain recorded. No later geometry, field or gameplay admission follows from these results.

The current correction computes each immutable exact facet box once and builds a bounded tree over contiguous source ranges. Queries reject only strictly separated closed boxes, test individual cached boxes at surviving leaves and emit candidates in original source order. Finite sections, affine fields, occupancy rays and contact/topology checks retain their existing rules. Caching alone would leave the flat visit/meet scan plus actual decode work above the original operation cap for every sealed input; disjoint group pruning removes those visits as well as repeated box construction.

Every cache build, tree-node construction/visit, box test, leaf visit and candidate processing step is charged. Cache records, conservative node slots, headers, traversal/recursion stack and candidate ordinals are admitted through the same storage guard. The index releases before final noding. The index adds retained storage during collection; complete original construction can still refuse on later work or storage. No physical-memory or speed claim is made.

The immediate parent certificate and unchanged independent solids were frozen before editing. A complete comparison checks ordered geometry, volume, every facet/contact witness and public verification in both lanes, requires lower charged work on all six solids and retains all work/storage counters. An independent corner-pair oracle checks ordered candidates across tree sizes, equality contacts and exact/one-short scratch bounds, including release on error. Current 28 Rust tests and changed original-five execution remain UNRUN pending bounded review and owner qualification. Evidence and criteria are in `integration/bounds-index/DELTA.json` and `fixtures/bounds-cache-reference/FROZEN.json`.
