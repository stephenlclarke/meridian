# World save and continue

Historical save-core source status: version-one encoding, decoding, semantic validation and six round-trip/tamper tests were authored on paid-services base `5531b8407fd690ca5a5f4155014574a6838c4de1`. Rustfmt parsing/formatting passes. Compilation, Rust tests, Clippy, remote quality checks and native UI acceptance are UNRUN on this exact successor. No filesystem operation or actual process quit/Continue has been performed by this implementation.

## Public memory API

`World::save_bytes()` returns `Result<Vec<u8>, save::SaveError>`. It validates the complete owned World before emitting a bounded binary envelope. `World::from_save_bytes(bytes)` constructs and validates an isolated candidate. `World::restore_bytes(bytes)` validates the entire candidate and invalidates its transient UI state before one final assignment replaces the live World. Any error leaves the live World unchanged.

`World::active_freight_job()` returns the existing opaque active custody handle for a continued receiving UI. It does not create a job or grant contact, docking or receipt authority; the normal current NPC review/confirm flow remains required. The partial-seven continuation test checks this projection and completed continuation returns no active job.

The core has no filesystem, window, GPU, wall-clock, global identity allocator or external process dependency. Root's host must own atomic disk writing, file custody, real quit/restart interaction and user-visible errors. There is no migration, best-effort repair, fallback new game, restock or compatibility alias.

## Format and retained state

The current commodity successor uses magic `MERSAVE1`, explicit version `3`, rules `3`, exact compiled rules/content binding plus a fingerprint of eight actual simulation resource byte streams, fixed-layout typed fields and a final 64-bit corruption checksum. The original version-one format had five resource streams; versions one and two now refuse atomically without migration or reset. See [paid equipment](PAID-EQUIPMENT.md) for the current successor’s source-only validation status. The content binding pins G02 content, the selected station/hull source, the navigation data, Food market data, original seventeen commodity definitions and both full native markets and the SH025 cargo-bay equipment record. Inputs over 65,536 bytes, truncated numbers, invalid tags, extra trailing data, unsupported versions, changed content and mismatched checksums refuse. All arrays have compile-time bounded sizes; input-provided counts cannot allocate an unbounded collection.

Persisted state includes the actual flight pose, quaternion, speed, tick, mode/refusal; owning commander/ship/drive IDs; current system, visit, admission sequence and initial-versus-physical dock origin; full pending jump destination/source/countdown/fuel/drive binding; the sole account's credits, fuel, owned equipment identities/components/provenance/fittings and actual cargo fields; all sixteen supplier unit identities through allocations, jobs, held/delivered masks and transfer histories; issuer/receiver station facts, completion receipts and first clue; both real station Food stocks and their prices; service/travel accounting totals; session epoch, pause state and existing NPC roster identity.

Current World has no mutable RNG stream. Navigation and market source data are immutable and content-bound; the fresh profile seed is its persisted commander identity. No RNG is invented or reseeded during load. A future random producer must add its complete state and update the save version/content boundary before it is admitted.

The selected contact and pending confirmation review are transient and are never restored. Continue advances the private UI review and session sequences. In-place restore uses the greater of saved and live sequences before advancing, so repeatedly restoring the same file cannot resurrect an earlier live review. Actual current dock context, NPC entity generation, property, cargo, balance and progress remain retained. No dock is inferred from a presentation mode or proximity, and no fresh owned-fit/market/source constructor runs during load. Only immutable station/navigation resources are rebound after content checking.

## Semantic validation

Each private component decodes its own representation, then World validates their agreement before publication:

- Navigation owner, ship, drive, system, current dock, visit/launch/jump chronology and pending travel must agree. An initial authored dock remains Avenor visit/sequence one; a physical admission has a real later sequence/tick. Fuel cost, reach, countdown and drive checks use the current immutable navigation rules.
- Equipment IDs remain nonzero, distinct, owner/hull bound and exactly the supported World's retained starter inventory. Unsupported paid provenance, missing fit, invalid cargo capacity, duplicate/foreign IDs and other invalid property refuse. Version one also refuses nonzero ordinary non-Food cargo and Food reservations, because this World has no producers for them; it does not discard or create those fields.
- Jobs occupy exactly their allocated source slots. Unit IDs match the owner/batch serials. Transfer records form the actual one-or-all-remaining prefix, with ordered unique transaction sequences, correct source/destination dock identities and exact custody masks. Completion must equal the last full transfer, and the clue must equal the first completed source receipt. An active or seven-unit job cannot claim a completion, clue or reward.
- Actual service producers now retain per-site buy/sell counts, refill calls/units and paid/credited totals. These mutate only in their existing checked candidate, before joint account/stock publication. Stock must reconcile against finite initial stock and real trades; Food against net buys/sells; charges against unchanged source prices. Sale-credit saturation remains distinct from G02 overflow refusal.
- The account revision equals freight, service and completed-jump revisions together. Cash equals initial cash plus once-per-completed-job reward and actual sale credits minus service payments. Fuel equals original fuel plus actual purchased refill units minus charged jumps. The accounting is checked with wider arithmetic; load creates no balancing entries, refunds, refills or receipts.

The checksum detects corruption. It is not a signature, anti-cheat mechanism or proof that someone with full write access could not construct a different internally consistent history. Semantic rejection covers invalid or inconsistent records even when a test supplies a replacement valid checksum. Trusted host custody/authentication is a separate concern; do not present this core format as external authentication.

## Authored verification

Six new tests exercise normal public World commands for fresh/preaccept, seven-unit, completed and in-flight countdown round trips. The positive path buys real Food, accepts the job, performs actual flight/jump/physical entry, hands over seven original units, continues, and pays only on the real eighth. A completed continuation retains the same clue and source exhaustion state and cannot pay again. Mid-spool continuation preserves exact pose/countdown and charges fuel once. A later real refuel preserves the same paid account on another round trip. Pause remains paused; initial and physical dock origins remain distinct.

Negative checks use malformed/truncated/oversized/version/content/checksum/extra data and non-finite pose inputs. Replacement-checksum semantic edits try to invent cash/Food/fuel, alter inventory ownership, duplicate equipment/goods IDs, mark an untransferred job delivered, restock a market and alter the completed clue receipt. Every refusal checks both public observables and the live World's complete encoded bytes for no mutation. Repeated in-place restores must invalidate old pending reviews even after another current review is created.

All pre-existing test identities remain. The G02 receiving block is extracted into a private preparation helper to address the actual `rust:S3776` finding, retaining validation order, original account/fact binding, overflow refusal and candidate-only mutations. No suppression or warning waiver is added.

Required next: one whole-packet independent source review; exact Dell compilation, all tests and strict Clippy; final quality checks; Root's atomic file writer and real process quit/continue UI evidence. Authored test names and formatter success are not execution results.
