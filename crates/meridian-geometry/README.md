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

## Packed region ownership and publication

The qualified bounds index passed 28 Rust tests, nine host tests, check, strict Clippy and the dev build. One changed whole-five run decoded all inputs, then refused all five constructors on Storage, leaving 15 dependent bodies UNRUN and no candidates. Earlier Operations/Storage failures and adverse work observations remain retained. Fleet work remains separate from preview delivery.

The current correction stores classified regions as exact point pools with stable cell-corner and physical-segment ordinals, alongside their unchanged protected-point trees and work metadata. A temporary sorted ordinal lookup charges its actual comparisons and shifted records. The global 3D noding pool remains admitted throughout collection and packing, with the original single charged insertion per point visit. The full parent raw-region reservation stays live during packing; each new record is admitted before allocation, the lookup releases after drop, and partial packing rolls back its own reservations on error. This removes real repeated point storage, rather than reducing the old reservation constant.

After the complete global point pool exists, reconstruction expands and consumes one region at a time under the full original raw bound. It admits full final geometry and aggregate witness ownership before moving cells, physical segments and protected points into that witness, then releases the expanded ownership before verification. All future packed regions and original source facets remain retained. The qualified index block, source ordering, globally noded geometry, affine fields, contact rules and complete global topology checks remain unchanged.

A source-independent round-trip regression covers exact fields, physical point contacts, work metadata, full overlap admission, one-short refusal/rollback and invalid ordinals. Six frozen complete solids compare every output and witness against the immediate parent, require lower charged storage peaks and retain all work observations. All prior assertions remain intact. Current 30 Rust tests and changed original-five execution are UNRUN pending review and owner qualification. Packing adds work and transient overlap; later original construction can still refuse. `integration/region-storage/DELTA.json` binds the actual parent failures, representation policy and frozen controls. No physical-memory minimum, speed or new fleet admission is claimed.

## Conservative forward-ray candidates

The qualified packed-region source passed 30 Rust tests, nine host tests, check, strict Clippy and the dev build. Its one changed whole-five batch decoded every input, then refused every construction at the original operation guard, leaving 15 dependent bodies UNRUN and no candidates. Recorded storage peaks below the cap do not prove that complete construction fits storage. All prior refusals and adverse work observations remain retained.

The current correction reuses the exact facet tree for occupancy rays of larger owners. It rejects a box only when it lies strictly behind the origin on a forward-ray coordinate, retaining closed equality and zero-axis contacts. A node must also overlap the owner box. Left-first traversal and exact owner filtering preserve source order. Every candidate still receives the original exact plane, hit, containment, ambiguity and parity checks, with the same normal-facing directions and retries. No new rational arithmetic or contact approximation is introduced.

Small owners keep the flat scan. A cached facet count costs one additional logical record per owner and one charged update per original facet. All node visits, owner-box tests, forward-axis comparisons, leaf visits and exact candidate tests remain charged; returned ordinals and traversal scratch are admitted through scoped storage guards. This adds explicit small-case bookkeeping and does not claim universal savings.

Before authoring, the complete immediate parent and independent subdivided solids were frozen. New controls cover a 108-facet analytic cube and a buried 120-source case, checking exact volume, complete ordered output and every witness field plus public verification in both lanes. They require lower full charged work on the large-owner cases and retain all storage/work observations. An independent corner predicate checks ordered candidates, interleaved owners, signed/zero axes, boundary ambiguities, exact parity, scratch refusal and release. All prior assertions remain unchanged. Current 32 Rust tests and changed original execution are UNRUN pending one review and owner qualification. `integration/ray-pruning/DELTA.json` binds the source rationale and sealed actual failures; it makes no measured dominant-stage, speed, minimum-memory or admission claim.

## Shared closed ray interval correction

The first forward-ray join passed all-targets check, then failed its library test run: 14 tests passed and the complete subdivided-identity control failed its strict work-reduction assertion. Its preceding 108-triangle geometry, volume, witness and public-verification assertions completed. The control itself did not pass; its buried case, 17 remaining Rust target cases, nine host tests, strict Clippy, dev build and original whole-five were UNRUN. Old/new numeric totals were not emitted, and no unchanged diagnostic was run to obtain them.

The correction replaces independent coordinate half-lines with one exact shared closed ray interval. Boxes whose coordinate hit times are disjoint now reject before leaf traversal. Entry equal to exit remains eligible, including t=0 and single-point contacts. Zero axes retain closed containment. Axis-aligned rays keep the existing exact specialization. Both paths share the same ordered traversal, storage admission and unchanged exact triangle/parity/retry tests.

Each nonzero-axis interval charges two additional endpoint computations alongside the existing axis visit. Direction classification also charges its three visits. Caps, prices, the small-owner cutoff, frozen controls and strict lower-work condition remain intact. The assertion now includes old/new totals on failure, bundled with the code correction. An independent all-pairs interval oracle checks skew times, boundary equality, signed/zero rays, complete ordered candidates, exact parity and scratch refusal/release. Current 33 Rust tests and original execution remain UNRUN pending one delta review and changed owner qualification. See `integration/ray-interval/DELTA.json` for the exact failed evidence and scope. No work-saving, speed, minimum-memory or admission claim is made for the corrected source.

## Owner-local plane-result reuse

The interval correction passed check, then failed the same complete work control: 440,444 reference operations against 461,042 candidate operations. Fifteen library tests passed and one failed; the buried control, 17 other Rust target cases, nine host tests, strict Clippy, dev build and original execution were UNRUN. This is a measured regression, not a passing optimization. The raw log and sealed failure remain in `integration/plane-cache`.

The new correction reuses exact ray/plane results within one owner and ray attempt. The 108-facet frozen cube has six repeated raw plane equations. A guarded builder assigns local IDs from owner, unnormalized normal and exact offset. The first use computes the original plane result and validates any hit coordinate before caching it. Every triangle still performs its own containment, coplanar/edge ambiguity and parity checks, in original order. Different raw equations and different attempts never share results.

Persistent ordinal/count records, borrowed-key build scratch and attempt caches are admitted before allocation and released after drop. All visits, lookups, initialization and new plane computations are charged; the original per-facet and coordinate-check prices stay intact. Small owners keep the flat path. A cost bound selects caching only where repeated equations can amortize its charged initialization/computation/lookup work; other owners retain the interval path. This does not guarantee savings for every ray or alter any resource or test threshold.

The new unit regression covers owner-local IDs, raw coefficient distinctions, positive/backward hits, individual edge/coplanar ambiguity and exact/one-short admission with rollback. All 33 prior tests, the frozen 108/120 controls and their strict lower-work assertion remain byte-for-byte unchanged. Current 34 Rust tests and original execution are UNRUN pending one delta review and owner qualification. No whole-original speed, full-storage-fit, minimum-memory, cure or admission claim is made.

## Reusable ray-cache storage

The owner-plane correction passed 34 Rust tests, nine host tests, check, strict Clippy and the dev build. Its frozen identity control improved from 440,444 to 431,785 charged operations. One changed original-five batch still refused every construction on Operations, leaving 15 dependent bodies UNRUN and no candidates. Those partial storage peaks and wall times do not establish complete storage fit, speed or minimum memory.

The current correction removes repeated cache allocation and slot initialization while retaining fresh ray computation. Guarded owner tables persist only through classification. Every attempt advances a charged generation; a slot from another generation must recompute and pass every original coordinate guard before publication. Individual triangle containment, edge/coplanar ambiguity, source order and parity remain unchanged. No hit value is reused across different rays or attempts.

Owner lookup, generation updates, actual slot initialization and plane computations remain charged. Persistent slots add one generation record to the original reservation. Attempt scratch covers a new hit while a stale point remains owned. New-table initialization refusal rolls back its admission; all tables release after drop before final noding. The original scoped cache remains where its initialization is cheaper than lookup plus generation maintenance. All resource and test thresholds remain intact.

A new regression changes origins and directions, checks individual contacts, forces the coordinate guard after a previously valid hit, and checks exact/one-short table admission. All 34 prior tests and the frozen 108/120 strict controls are byte exact. Current 35 Rust tests and original execution are UNRUN pending one review and owner qualification. `integration/ray-cache-storage/DELTA.json` records the source-supported repeated work and exact parent failures without inferring a measured hotspot or new admission.

## One-sided original-facet sections

The qualified reusable-cache parent passed 35 Rust tests, nine host tests, check, strict Clippy and the dev build. Its whole-five batch still refused every constructor on Operations, with 15 dependent bodies UNRUN and no new hull admission. The exact blocker remains retained; its source markers do not locate an inner hotspot.

The new material classifier builds complete owner sections in the limiting plane on each original facet's outward side, then classifies its existing atomic samples with exact planar parity. It preserves segment multiplicity, nonconvex owners, disconnected sheets and cavities. Original physical intersections, geometry, affine fields and all publication audits remain in force. Boundary ambiguity keeps the original generic-ray policy. Every section frame has an explicit per-facet lifetime; classifier storage is admitted alongside the live arrangement. The original arrangement source and all 35 old tests stay byte exact.

Seven new tests cover nonconvex/cavity hulls against the frozen parent ray oracle, shear/mirror transforms, semantic contact/field refusals, signed/zero contacts, parity multiplicity, independent box support and exact/one-short lifetime admission. All 42 current Rust tests and changed original execution remain UNRUN pending Root/G2 review and owner qualification. No speed, full storage fit, minimum memory or fleet acceptance is claimed. See [the derivation, accounting and fixture contract](integration/sections/ALGORITHM.md).

The first section-classifier join failed the owning `cargo check --all-targets` before any tests ran. Fourteen calls in the new parity fixture used the wrong `fraction` signature. They now pass numerator/denominator strings and the budget, then propagate the result. The exact fixture values and assertions, all production code, frozen references and limits remain unchanged. The corrected 42 Rust tests, nine host tests, Clippy, dev build and whole-five execution are UNRUN pending the same-slice review and owner gates. Raw compiler evidence and the exact correction are in `integration/section-test-api`.

## Joint section collection

The corrected section revision passed compilation and 24 library tests, then failed the unchanged strict overlap work comparison. Its failing operation pair was not logged and remains unknown. The remaining 17 Rust tests, nine host tests, Clippy, dev build and original execution stayed UNRUN. The larger 108/120 strict controls passed; they do not override the overlap failure.

The new collector combines the physical and outward tree traversals and shares the exact distances/intersections needed by both representations. It preserves their ordered outputs and all downstream geometry and field checks. Two new tests compare against the unchanged separate routines and check complete admission/rollback. All 44 current Rust tests and nine host tests are UNRUN pending review and owner qualification. See [the correction and accounting](integration/section-collection/ALGORITHM.md). No new original hull is accepted.

The joint collector passed check, all 44 Rust tests and nine host tests. Its strict overlap control passed with 106,146 reference operations and 105,378 candidate operations; the earlier failed pair remains unknown. Strict Clippy then stopped on the joint function's complex return type, leaving dev build and original execution UNRUN. A private alias now names that exact tuple. All non-type source, tests, assertions, limits and references are unchanged. The alias revision is UNRUN pending bounded review and owner qualification; `integration/section-result-type` retains the actual passing scope and failed raw Clippy context.

## Completed identity-map lifetime

The qualified joint collector passed all 44 Rust tests, nine host tests, check, strict Clippy and dev build. Its changed original batch still refused two inputs on Storage and three on Operations, leaving 15 dependent bodies UNRUN and no candidates. Full fleet fit remains unproved.

The new correction drops the original facet-ID and owner-ID sets immediately after their last audit/count use, then releases only their actual record memberships from the unchanged initial reservation. All original geometry, styles, field checks, caps, primitive prices and 44 prior test bodies remain unchanged. This releases 3,098 memberships for each storage-refused input at all later guards; it does not establish a complete fit or resolve the three work refusals. One independent lifetime-boundary regression brings the current total to 45 Rust tests plus nine host tests, all UNRUN pending review and owner qualification. [The evidence and precise remaining work blocker](integration/identity-lifetime/ALGORITHM.md) retain every case, timing and limitation.

## Verified original owner ranges

The identity-map lifetime revision passed 45 Rust tests, nine host tests, check, strict Clippy and dev build. Its original-five run stayed UNRUN. The last original batch still has two Storage and three Operations refusals and zero complete candidates.

Original topology auditing now verifies ascending owner runs and audits each borrowed facet slice with the unchanged per-owner checks. Unordered input uses the prior collector. This removes two full input scans per owner from the ordered path and saves 71,467 charged operations before the previous SH016/SH017 refusal points and 56,287 before SH166. These are source-derived prefix reductions only. The 47 Rust tests and nine host tests on this revision are UNRUN. [The derivation and limits](integration/owner-ranges/ALGORITHM.md) preserve the exact old failures and remain separate from any later full-fit result.

## Authenticated style lookup and refusal context

The owner-range revision passed 47 Rust tests and nine host tests. Its original-five run still refused every constructor and returned zero hulls. The adapter now looks up each authenticated canonical style encoding before repeating hexadecimal decoding. This removes 38,228 charged operations for SH010/011, 41,572 for SH016/017 and 37,316 for SH166 before construction.

Budget also retains its first rejected charge with the caller, active phase, stage work totals and exact retained/requested values before cleanup. The actual adapter publishes that snapshot in the case report. The 50 Rust tests and nine host tests for this revision are UNRUN pending review and owner execution. [The source-derived work, schema and independent comparisons](integration/adapter-context/CONTRACT.md) define the validation scope.

## Deferred noding pool and owner pruning

The style/context revision passed 50 Rust tests, nine host tests, check, strict Clippy and build. The changed whole-five still returned five constructor refusals and no candidates. The exact new contexts identify occupancy scratch with retained global points for SH010/011 and charged section/field work for SH016/017/166.

The noding point pool now builds after classification and index cleanup from the same packed cell/protected records. Each region visits repeated local references once. Proven uniform-owner nodes also exclude ineligible outward sections while physical cuts remain independent. All 50 existing test bodies, frozen controls and limits stay unchanged. The current 52 Rust tests and nine host tests are UNRUN; full fit remains unproved. [The complete causal correction and ownership contract](integration/live-overlap/CONTRACT.md) includes the exact prior receipts and new validation criteria.

## Actual run lifetimes and sparse support

The deferred-pool revision passed 52 Rust tests, nine host tests, check, strict Clippy and build. Its one changed whole-five still refused all constructors. The private arrangement counter now retires only corners already dropped by run extension or cell deduplication, keeps the existing 16-membership price and guards transfer scratch before allocation. Current-facet outward support also skips exactly zero coefficients after one charged normal scan.

All 52 prior test bodies and immutable controls are preserved. Three new independent comparisons bring the current total to 55 Rust tests plus nine host tests, all UNRUN. Full fit remains unproved. [The causal evidence, accounting and qualification contract](integration/live-runs/CONTRACT.md) retain every old outcome and the strict unchanged caps.

## Incremental membership bookkeeping

The live-run qualification passed check and 36 library tests, then failed the unchanged exact work comparison at 325 versus 322. The remaining 18 Rust tests, nine host tests, Clippy, build and original-five stayed UNRUN. The separate run/cell recount operations are now eliminated: membership follows the existing charged point push, pop and dedup visits. Original geometry charges, the 322 assertion, transfer guard, all 55 old test bodies and caps remain unchanged.

One new lifetime regression brings this source to 56 Rust tests and nine host tests, all UNRUN. [The causal delta and source-bound failure](integration/incremental-run/CONTRACT.md) retain the complete actual failure and unaffected-source proof. No new original result or full-fit claim is made.
