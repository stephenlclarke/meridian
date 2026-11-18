# Authenticated style decoding and budget rejection context

The latest executed geometry source is Root 09842c9762eb36cc06dc01df55db2a432cb5423b with tree 914216fe229eecf224483f21691ba353bd70fe48. Its 47 Rust tests, nine host tests, check, strict Clippy and dev build passed. Its 25-body original batch decoded five inputs and refused five constructors. SH010 and SH011 requested 100013 and 100028 storage memberships. SH016, SH017 and SH166 attempted 2000003, 2000001 and 2000001 charged operations. Fifteen dependent bodies stayed UNRUN and zero candidates returned. The broader 202-case matrix has its own source and execution requirements.

## Repeated adapter work

The authenticated decoder first compares every input byte against its compiled immutable asset. It then decodes hexadecimal style text on every facet before checking the six-entry shared text pool. All repeated encodings therefore spend their full per-byte decode charge before reusing the same immutable style allocation.

This revision compares the original canonical encoded token against previously decoded tokens first. A matching token reuses the existing Arc text. A new token goes through the complete original hexadecimal and UTF-8 checks and is added to the pool. Canonical lowercase hex equality and decoded byte-string equality select the same entry, in the same insertion order. The full-byte source binding still executes first. The five compiled assets are the only accepted inputs.

The comparison retains the existing one-step pool-lookup price. Each actual hexadecimal decode retains one step per decoded byte. The new pool records a borrowed two-field string slice, one immutable handle and one record membership, and guards four memberships per pool entry. Encoded bytes remain borrowed from the authenticated input for the duration of decoding. The returned original Facets own their immutable decoded style handles. All normal geometry admission, rational checks, exact corner UVs, identity and style data continue through the original constructor.

| Input | Facets | Unique decoded style bytes | Removed repeated decode charges |
| --- | --- | --- | --- |
| SH010 | 1012 | 228 | 38228 |
| SH011 | 1012 | 228 | 38228 |
| SH016 | 1100 | 228 | 41572 |
| SH017 | 1100 | 228 | 41572 |
| SH166 | 988 | 228 | 37316 |

These are exact source-derived reductions before every construction cutoff. They describe the removed decode operations. Full reconstruction and certification still require changed owner execution. The larger temporary pool adds at most 18 memberships across its six entries. Every actual live pool record passes its guard.

## Rejection snapshot

Budget keeps bounded control state alongside its existing limit, source and counter fields. The state contains an active phase, eight numeric checkpoint values, their charged-operation timestamp, 18 phase counters and the first resource rejection. It owns no geometry, strings, collections or growing event trace. Budget-control counters and fixed call-location metadata have the same control-state accounting treatment as the existing Budget object. Geometry reservations, temporary data and all original caps retain their prices. Capturing control metadata adds no charged geometry operation.

The original saturating operation update, limit comparison, checked storage addition, attempted/admitted peak rules and reserve/release order remain. The first rejected step or guard copies its context immediately before returning the existing typed Failure. Track-caller propagation through retain and with_retained identifies the invoking source location. Scoped phase and reservation cleanup then run normally. The snapshot persists after that cleanup.

The actual adapter copies the snapshot into the case report. Its optional budgetRejection object contains the Failure kind and latest source, the active phase, caller file/line/column, operation and storage bases for a dependent-check budget, local operations before/after the attempted charge, attempted operation increment, retained memberships before cleanup, attempted scratch, optional required memberships, active limits, phase work totals and eight checkpoint counters with their recording time. The old Failure shape and five declared bodies remain available to existing callers. A semantic or rational failure outside a Budget step/guard can have a null budgetRejection.

Phase counters include the attempted rejected charge. Their saturated sum matches the charged total for that Budget. Both bases are zero for the main decode/construction budget. A dependent-check budget records the already-spent case work and the retained source/candidate storage that tightened its available limits. This preserves the total-case interpretation when checks use tightened remaining limits.

Checkpoint values are a recorded stage inventory. Their timestamp allows the reader to determine work charged after that checkpoint. The active source location and sealed implementation define each layout. The main recorded layouts are listed below.

| Phase | Checkpoint values in order |
| --- | --- |
| Decode | Decoded facets, style entries, style bytes, fields, line bytes, scalar index, current component, current facet |
| Index | Retained original input, owner-bound reservation, original count, then five zeros |
| Sections | Frame memberships, selected owners, physical barriers, coplanar references, original count, current ordinal, joint scratch, zero |
| Arrangement setup | Base, segments or ordered segments, physical segments, protected points, events, then three zeros |
| Arrangement pairs | Base, ordered segments, physical segments, protected points, events, processed pairs, then two zeros |
| Arrangement slabs, occupancy, fields | Base, ordered segments, physical segments, protected points, events, accumulated cell members, levels, active/following runs |
| Global points | Retained point pool, packed regions, point count, raw reservation, then four zeros during insertion |
| Packing | Initial cell/physical/protected counts, then zeros at entry. During interning, points, lookup size, lower bound, upper bound or shift count, then zeros |
| Noding | Cell corners, global points, accumulated result, cuts, edge ordinal, then three zeros |
| Triangulation | Remaining corners, completed triangles, then six zeros |
| Contact stars | Triangles, protected points, then six zeros |
| Witness | Expanded/planar/star/cell/physical/protected counts at publication. A nested witness subtotal update records prior subtotal, count and width |

A phase-entry checkpoint begins with zero values when no finer checkpoint has been recorded. Caller locations and timestamps distinguish that entry from an explicit inventory update. Guard state records the exact retained amount and requested scratch independently of the checkpoint inventory.

## Review and independent checks

Before source authoring, the adapter, Budget and geometry/host implementations were frozen. The compiled independent adapter rewrites its compile-time fixture path to the current crate root. Its frozen generated bytes remain in a lossless gzip file. The compiled view removes the final blank line, and the source verifier proves that exact whitespace-only derivation. The raw parent adapter remains pinned alongside it. One new regression compares all original Facets and input ownership to that frozen decoder on all five assets, requires the exact removed work and preserves full-byte authentication refusal. A second tests first-rejection capture across reservation and phase cleanup, exact stage sums and caller propagation. A third exercises a real small closed input through reconstruction and the adapter's refusal JSON without returning a candidate.

The source validator removes only explicit context annotations and restores the complete parent geometry source. It also restores the exact primitive Budget arithmetic and limit-constructor code and checks all prior test bodies. The new 50 Rust tests and nine host tests are UNRUN. Source formatting, metadata, immutable bindings and comparison checks are author activities. Root owns the serial joined source and G2 review, and G1 owns compilation and any changed original execution. An original run requires the changed implementation to pass those focused gates.

Every original 2M operation, 100k membership, 4096-bit and 512-face limit continues to apply. The next refusal receipt will identify its callsite and active budget before cleanup. Any further reconstruction optimization must use that resulting causal evidence.
