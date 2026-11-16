# Frozen Lua compatibility corpus

This packet contains executable Lua inputs, lossless typed inputs and expected canonical results or refusals from the immutable Swift/C host. All execution is UNRUN. These are source-derived expectations, not recorded interpreter traces.

Read CONTRACT.json, then CASES.json. Each case file names its exact Lua or binary input and legacy reference paths. Followup in the explicit-state case requires a fresh VM with the previous returned data. Do not retain closures between calls. Compare typed values without JSON integer or float coercion.

HOST-SCENARIOS.json holds separate event/controller/native contracts. These require a real host adapter and actual authority joins. They are not executable Lua assertions or accepted gameplay. The original command proposal is still untrusted until native validation and durable publication succeed.

REFERENCE.json pins every copied source and notice to the immutable legacy commit. Original packs beyond the named cases are reference material only. Retain licenses and provenance; source presence supplies no new distribution permission.

Existing compiler owners may execute a changed production implementation against this frozen packet. Record exact implementation commit/tree, packet manifest hash, platform/runtime identity and actual result/error traces. Do not edit expectations to fit a failed implementation. Root audits the authored fixture packet; G2 reviews other authors production code separately.
