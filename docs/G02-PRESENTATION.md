# G02 mission dialogue and objectives

Historical feature contract and author-stage notes. For current delivered behavior, exact validation and open limits, read [implementation status](IMPLEMENTATION-STATUS.md). Earlier UNRUN statements below apply to the original handoff, not the latest delivered source.

## Scope and source

The first playable requires the original consented-freight story and terms alongside its actual custody/settlement mechanics. At Meridian base `df2b1301369b90272df1c58917b6303df8efeefd`, the World already supports G02 offer, one/all receiving, abandonment and durable continuation. Desktop review text is generic and does not present the original full offer or crew consent. It also distinguishes offer/receiving using next-unit presence. Receiving and abandonment reviews both retain the next held original unit, so that heuristic cannot identify an abandonment action. This slice supplies concrete read-only G02 presentation; Root owns the desktop integration.

The source is immutable original L337 revision `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`, not the human working checkout. [The per-file provenance](G02-PRESENTATION-SOURCES.json) pins paths, sizes, hashes, transformations and the retained exact MIT notice. The source pack declares MIT; `assets/legacy/Elite-L337-MIT.txt` remains byte-identical to its source LICENSE. New projection code follows Meridian's repository license. No Elite/Oolite mission, Lua implementation, new framework, renderer asset or external content loader is added.

The original offer is reproduced with the supported one-leg frozen reward substituted as 730 CR / L1. The accepted callback journal and both receiving/abandonment dialogues are exact original strings. The original one/all choice labels and same-original-unit warning are preserved. New short objective descriptions and the first-clue-only reminder are editorial presentation of the existing rules, identified separately in the provenance.

## Read-only API

The API is `meridian_core::freight::g02::presentation`.

| API/value | Purpose |
| --- | --- |
| `review(review, selected_choice)` | Returns a descriptive `Option<ReviewPresentation>` by matching the actual private review purpose and explicit selected `Choice` |
| `ReviewKind` | Distinguishes `Acceptance`, `ReceiveOne`, `ReceiveAll` and `Abandonment` |
| `ReviewPresentation` | Static speaker/title/body/notice, optional original-tonne transfer counts and at most three labelled choices |
| `DialogueAction` | Describes `Confirm(existing Choice)` or `Back`; it never executes a command |
| `TransferProgress` | Original tonnes remaining and original tonnes selected for this confirmation; no invented delivered count or completion prediction |
| `objective(&World)` | Describes the actual owning World's active job, finite source, completed jobs and retained trusted clue |
| `ObjectiveKind` | Collect originals, deliver originals, completed family objective or exhausted supplier |
| Original text constants | Offer, accepted journal, receiving consent, abandonment warning, identity notice and destination |

`Some` from `review` means that the requested presentation matches the typed review purpose. It is not an authorisation result. Rendering an old or foreign review cannot restore pending consent, a dock or contact. World must still receive the existing command and validate the live review, owner, account, selected NPC and actual admission. A mismatched choice returns no presentation. Receiving one requires that review's next original unit; receiving all is selected explicitly, even though the review also has a next unit. An abandonment review retains the next held original unit, just as a receiving review does. Its typed purpose and explicit choice distinguish it from both receiving and acceptance.

Back is always presented as Back, separately from confirm. Its existing command revokes the review and transfers nothing. The projection cannot allocate units, transfer cargo, abandon a job, credit cash or award a clue. No public review/receipt/identity constructor is added.

## Original rules visible to the player

- ARLEN at Avenor reserves eight original Machinery utility-parts tonnes from one finite sixteen-unit batch for the consenting Renas main-orbital utility crew.
- The supported direct route has one native leg; its frozen H0 reward is 730 CR. There is no deposit, berth requirement or deadline and no new special paid kit.
- Ordinary Machinery cannot replace the original identified units. Loss, abandonment, update and reinstall do not replenish allocations.
- The receiver offers one original tonne or all remaining originals. Seven partial transfers pay nothing; the final original transfer commits the existing once-only job payment and first family clue.
- The family clue is first-completion-only. Later job payments do not replace or duplicate its first receipt.
- Abandonment is explicit, pays zero and never returns the allocated units to the supplier. Back changes no cargo or payment.

The objective reads actual World state, not a UI counter. An active later allocation remains a delivery even when the earlier family clue is already recorded. Completion is displayed from the actual World's retained clue record; seven transfers cannot trigger it. If both allocations are abandoned, the objective states that the finite batch is exhausted instead of inventing replacement stock or claiming success. This is truthful presentation of the current supported rules, not a newly designed loss-recovery system.

## Verification and integration

Four authored tests use actual public World commands. They check genuine offer/Back and stale-review refusal; two real abandonments with no restock, cash or clue; the ordinary launch/jump/flown physical dock and seven/eighth receiving sequence; and absence of a G02 objective in prototype worlds. The receiving test explicitly distinguishes one versus all for the same real review, checks presentation leaves encoded World state unchanged, continues the seven-unit save with the same active job, rejects the old review, settles only the eighth through a new current crew review, and retains the completed objective on another memory round trip. No pose/dock/receipt or permission is injected. This is not an actual process quit/Continue or a GPU result.

The first joined candidate `37e21708097a57126a9ce1c10b96571313ddaaf2` failed [CI run 43767000952](https://github.com/stephenlclarke/meridian/actions/runs/43767000952): the abandonment test wrongly expected no next unit. The unchanged domain issues the next original unit for both receiving and abandonment. The corrected test checks the exact first GoodsId from its actual Accepted event for each allocation, rejects acceptance and both receiving choices for the abandonment purpose, and preserves all no-mutation/no-restock/zero-payment checks. The original failed run remains evidence; the corrected joined candidate must be tested.

Rustfmt parsing, source fidelity/hash comparisons and scoped documentation checks are source-only evidence. Corrected-candidate compilation, tests, strict Clippy, joined desktop readability/input and native game acceptance are pending. Root joins the presentation with its Session/Profile/renderer work, including explicit one/all/abandon actions, then routes one complete candidate through the existing Dell owner. There is no separate leaf-run gate or competing Mac compiler/GPU job.

Root's UI must retain refusal/status and the explicit pending choice plus confirm/cancel controls while wrapping the original body and warning. Do not truncate the terms into a generic review-ready notice or interpret presented choices as new authority. See [current implementation status](IMPLEMENTATION-STATUS.md).
