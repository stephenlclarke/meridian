# Root serial mining/resource replay join

This standalone fast-laser source does not contain MC1's mining module. It refuses every Work record while `actual_resource_work_counts()` returns None. Root must bind actual sealed extraction and actual finite per-body depletion before accepting a joined mining save. A getter or displayed mining tool never grants work authority. Existing MC1 ray, source, occlusion, speed, scope/epoch/item/revision, reservation, hold and depletion checks remain.

In the actual `mining.rs` State impl add the following read-only projection of already owned simulation state:

```rust
pub(crate) fn resource_extracted_counts(self) -> [u64; BELT_BODY_COUNT] {
    self.extracted.map(u64::from)
}
```

Replace the isolated `World::actual_resource_work_counts` body in `flight/resource_history.rs` with `Some(self.mining.resource_extracted_counts())`. Paste the method from [the concrete constructor](integration/RESOURCE-WORK-CONSTRUCTOR.rs) inside the same module's World impl. This crate-private constructor only produces a prospective history; it grants neither extraction nor resource debit and is not exposed to UI.

At MC1's actual `request_mining` boundary, retain all sealed physical/cargo/source checks and the already real `self.prepare_mining_resources(tool.item_id,tick)` resource preparation. Capture `(combat, emission)` from its existing actual `reserve_missile_emission()`. Then prepare the coupled record with `self.prepare_resource_work(tool.item_id,tick,emission,contact.asteroid.ordinal())`. The asteroid index comes from MC1's accepted physical contact, never caller input.

Before the first live assignment, decode a checked prospective World using `World::decode_saved_world(&self.save_bytes()?)`, preserving MC1's actual error mapping. Put the already prepared account, mining state, pulse, combat and resource history into that private candidate and call its practice `observe_discharge(tick)`. Validate its complete `save_bytes()`. Recover the account and history by `take()` and publish those same prepared states through MC1's existing no-fallible-work-after-first-mutation boundary. This private validation decode does not advance the live epoch or publish a new World scope. Keep actual authority scope handling unchanged. On refusal, no yield, depletion, pool, emission or history changes.

Every accepted extraction must add exactly one Work event with the actual paid Mining item, current visit/tick/physical tick, accepted asteroid ordinal and real emission. Replay applies the original selected-work heat<=230/energy>1/18-tick guard. Per-asteroid Work counts must equal actual MC1 extracted counters. Work events are distinct from combat shots and cannot satisfy station/gunnery/interception receipts. Old current-format mining snapshots cannot be relabelled as fresh history. Root chooses one final combined content/layout boundary and preserves its prior failures/counterevidence.

Include [the two concrete joined cases](integration/RESOURCE-WORK-TESTS.rs) under the `resource_history` test module after adding actual mining. Both are **UNRUN** and not compiled by this isolated source. They exercise a real paid Beam burst before refitting the actual paid Mining tool, genuine belt approach/extraction, cold resource/depletion/cargo custody and coherent missing Work refusal. Retain the previous paid-tool authority/scope/restore/negative coverage too.
