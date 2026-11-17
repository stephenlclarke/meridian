# Release completed original identity maps

## Actual terminal evidence

Root `ddf06586c81c2ffe3c63357bb417fcd439870791`, tree `82a20cbc47f91433360e0e5072ab1bbb6e350d9c`, passed all 44 Rust tests, nine host tests, check, strict Clippy and the dev build. One changed whole-five batch decoded every input, then refused SH010/SH011 on Storage and SH016/SH017/SH166 on Operations. All 15 dependent bodies stayed UNRUN; zero complete candidates or new admissions resulted. The owner reported no input/binary change or postflight error. The native batch took 129.567703709 seconds. Raw per-case timings and their earlier cutoff comparisons remain preserved; they are not completed-pipeline timings or proof of full fleet performance.

| Input | Guard | Latest source | Operations | Attempted peak | Admitted peak | Terminal retained |
| --- | --- | --- | --- | --- | --- | --- |
| SH010 | Storage | 6,8 | 1761362 | 100023 | 99991 | 76942 |
| SH011 | Storage | 6,8 | 1754311 | 100006 | 99974 | 76503 |
| SH016 | Operations | 11,15 | 2000001 | 94181 | 94181 | 90548 |
| SH017 | Operations | 11,10 | 2000001 | 94066 | 94066 | 90256 |
| SH166 | Operations | 10,3 | 2000007 | 89417 | 89417 | 83662 |

Attempted reservations include refused guards; neither peak is an allocator measurement or a minimum-memory result. The exact resource limits remain 2,000,000 total operations including decode, 100,000 logical memberships, 4,096 coordinate bits and 512 complete output triangles.

## Bounded ownership correction

`Facet::input_memberships` explicitly includes original geometry, identity maps and actual style ownership in the unchanged `40N + 64 + styles` initial bound. Reconstruction creates a BTreeSet of original SourceIds and a BTreeSet of owner IDs, validates uniqueness, audits every owner's topology, audits original sheet crossings and reads the owner count for the bounds reservation. Neither identity set is read after that point. In the parent they remain owning locals until reconstruction exits, so their original reservation also stays retained throughout classification, packing, global noding and publication.

The correction consumes both sets in a private owner object at their last use. SourceId records have two scalar identity fields plus one set membership; owner-ID records have one scalar and one membership. After actually dropping both owned sets, release exactly `3 * unique_facets + 2 * unique_owners` memberships from the already-admitted input reservation. Reduce the later input-release balance by the same amount so final ownership is unchanged. The original 64 header/padding slots remain reserved, as do every original facet, normal, point, UV, style and authenticated identity stored on those facets. No geometry or field record is removed.

The initial 40N admission and every primitive price are unchanged. As M <= N, at most 5N record memberships release. The remaining base is at least 35N + 64 plus styles, still above the existing conservative 32N geometry representation used for the same Facet type. This is lifetime release of actual dead sets, not a smaller price assigned to live original geometry. No charged operation, geometry traversal, ray/section value, parity rule, physical constraint, witness or audit changes. The sets deallocate before their budget reservation releases. The helper consumes ownership, and the initial admission still refuses at its old boundary.

Immutable source metadata gives these record counts without running geometry:

| Input | Original facets N | Owners M | Released records | Unchanged initial input reservation |
| --- | --- | --- | --- | --- |
| SH010 | 1012 | 31 | 3098 | 40784 |
| SH011 | 1012 | 31 | 3098 | 40784 |
| SH016 | 1100 | 33 | 3366 | 44304 |
| SH017 | 1100 | 33 | 3366 | 44304 |
| SH166 | 988 | 29 | 3022 | 39824 |

## Connection to the observed failures

The input IDs are sorted and their final markers are 30,11; 30,11; 32,43; 32,43; and 28,43. Input accounting/identity validation and the completed per-owner audit leave that final marker. The sheet audit, bounds/index/plane construction and their arithmetic helpers do not change it. The later original-facet collection and output assembly set the current source again. Each actual refusal's marker differs from the pre-loop marker, so it occurred after the dead identity sets' last use. This is a source-supported phase bound, not identification of the exact failing inner body or callsite.

Every post-release storage reservation therefore loses the corresponding constant identity-record component. With unchanged algorithmic decisions up to the old refusal, the old SH010 failed requirement of 100023 would be 96925 at that same endpoint; SH011's 100006 would be 96908. These are source-derived reservations at the previous endpoints, not measured new peaks or proof that a later guard fits. Complete construction can still refuse on storage, operations, rational growth, output count or a semantic audit.

The three Operations failures remain an explicit work blocker. The terminal Failure contains only kind, latest source, total charged operations and generic text. The case report contains whole-body decode/construction deltas and global storage peaks/terminal retention, but no inner body, endpoint, active-record inventory or work-by-stage attribution. Source IDs are reused in classification and assembly, and guard scopes unwind before terminal retention is read. The 32-member storage peak increments and terminal retained counts cannot identify a particular arrangement, callback, packing, noding or witness call. No algorithmic impossibility, measured hotspot or proposed work cure is inferred. No unchanged diagnostic or instrumentation-only batch is requested.

## Preservation and validation

Before this change, the complete parent certificate, source accounting and independent membership-boundary contract were frozen. One new regression uses three unique SourceIds and two owners, including duplicate set insertions. It independently expects an initial 184 memberships, release of 13 and remaining 171. Nineteen later scratch memberships must pass at limit 190 and fail at 189; the same initial admission must still fail below 184. It verifies unchanged operation count, retained final zero and empty-map neutrality. The consuming release is required for these boundaries; no fake final candidate or geometry bypass is involved.

All 44 existing test bodies, all strict work assertions, all prior frozen references and every geometry implementation outside the identity ownership block stay byte exact. The source verifier removes only the new ownership helper, balance adjustment and test module, then requires complete equality with the exact parent certificate. One new test brings the current count to 45 Rust plus nine unchanged host cases. Current checks and original execution are UNRUN. Author activity was limited to source/ID/style metadata, hashes, formatting, Markdown and Git/sealing. Root owns serial integration, G2 bounded review, and G1 compilation/execution. There is no author compiler, native job, geometry probe, cap waiver, contact shortcut, field averaging, global CSG, cross-source output or unchanged retry.
