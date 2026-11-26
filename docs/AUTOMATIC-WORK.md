# Automatic mining and salvage

Development integration candidate. The public acceptance below is staged, not a report of a passing build or a native game run. The current World owns target authority, work, tool operation, cargo custody and refusals. The desktop reads that state; it does not grant work, alter the target or create cargo.

## Controls and equipment

Use F to select Mining or Salvage, then Tab to select an eligible actual target. A healthy ship is a normal tracking target, not a salvage grant. It needs a different, eligible disabled or wreck state before automatic salvage can begin. Selection of a work target starts automatic work without pressing Z. Delete or manual flight input ends guidance and work. F changes the filter and ends the current selection. Z remains manual use of the fitted front laser. T remains a missile lock; the missile and automatic-docking controls are unchanged.

The core and the separate matching targeting module must be fitted. Mining needs the fitted Mining Laser. Salvage needs an eligible source and the fitted Salvage Cutter for cutting. Complete physical recovery also needs fitted Fuel Scoops, a reachable actual payload and free cargo capacity. Owning equipment in storage alone grants no work capability. The automatic target readout reports actual approach, hold, tool and custody status; its text does not mean a pickup has already occurred. A missing fitting or capacity refusal must be visible and must not pay, create cargo or claim a completed recovery.

The original rock lock remains through depletion and its resulting ore acquisition. No second Tab, extra Z press or caller-supplied cargo award is part of that operation. Automatic intake must use the real released ore and scoop aperture. The intended local aperture is (0, -45, 55), the manual centre distance is strictly below 85 units and automatic relative speed must not exceed 15 units. These are acceptance constraints, not observations from a running build.

## Earned public acceptance: mining

Use a fresh isolated standard commander. Do not use XYZZY, injected balances, generated test cargo, a private state setter or prepositioned ships. The existing finite public market sequence funds the normal equipment purchase:

| Sale destination | Trade | Expected cash after sale |
| --- | --- | ---: |
| Renas | 29 Minerals from Avenor | 1,143.4 CR |
| Renas | 47 Minerals from Avenor | 1,829.6 CR |
| Renas | 75 Minerals from Avenor | 2,904.2 CR |
| Avenor | 48 Alloys from Renas | 3,365.0 CR |
| Renas | 37 GemStones from Avenor | 4,954.1 CR |
| Avenor | 83 Alloys from Renas | 5,750.9 CR |
| Renas | 64 GemStones from Avenor | 8,534.9 CR |

Return to Avenor through an actual jump and berth. Refuel at the real market when required; record all fuel debits and the resulting cash. Buy the Basic targeting core (800 CR), Mining Module (2,000 CR), Mining Laser (3,300 CR) and Fuel Scoops (1,900 CR), a total of 8,000 CR in the current Meridian shop. Verify the four paid item identities, fitted slots, wallet and available hold. These are the admitted Meridian prices, not a substitution of the historical game's cheaper scoop or laser.

Launch from the real Avenor berth, select Mining with F and use Tab on the first one-unit rock. In the headless public test, the viewport hint is the real rock ID from the current belt; this does not qualify actual renderer clipping or keyboard delivery. The acceptance requires the following:

1. Selection alone does not deplete the rock or grant cargo. A manual turn ends the selected operation without a work stroke or cargo transfer. Select the rock once again.
2. Thereafter supply only ordinary unheld flight steps. The same original rock identity must remain selected through actual depletion and automatic ore approach. Do not reselect, fire manually or call a collection command.
3. The original rock loses its one finite unit. Observe the actual matching pending ore, source and fragment identity, release clock, velocity and the approved Meridian six-vertex fragment with radius 8. The target's collection projection and the ordinary renderer use this same body; this is not a rebuilt legacy cargo hull. One matching native commodity unit moves into the hold only after physical collection; the matching pending ore is removed. Wallet and paid fitting identities stay unchanged.
4. Save through the ordinary compatible writer, then quit and Continue. The one unit, depleted original body and consumed ore remain consistent. No active guidance resumes on Continue and later ordinary steps cannot receive a second copy.

The executable headless acceptance is `crates/meridian-core/tests/automatic_work_public.rs`. It exercises public World commands and the decoder. Actual native input, target readout, render visibility, Save/Quit and process restart still require the owning platform run. The test is authored, UNRUN against the joined core.

## Required refusal and salvage acceptance

Before qualifying native work, repeat the actual earned setup in separate scratch profiles. The public acceptance also creates a separately continued copy of the earned fitted commander, stores the paid scoop without a refund, selects the real rock and requires the actual `Fit fuel scoops` refusal. It checks that the rock is still intact, no pending ore or cargo exists for that work, wallet remains unchanged and Continue preserves those facts. For a full hold, use real public market transactions to occupy the native hold. Record the reason published by World; neither case may add an uncollected commodity or credit. Continue must keep the actual finite uncollected source, not replenish it.

For salvage, fit the paid Basic core, Salvage Module, compatible cutter and scoops using admitted offers and a sufficient normally earned balance. Record the exact property IDs, actual remaining capacity and source custody. Observe that an ordinary healthy ship is not eligible for automatic salvage. Use an actual eligible disabled target or an eligible destroyed source with its original retained manifest; do not inject a wreck or assume that every destruction grants salvage. Select it once through the Salvage filter. Confirm normal approach and cutter/scoop state in the target panel. Require actual finite payload transfer, provenance and consumed source evidence from World. Record separate refusal cases for missing cutter, missing scoops, occupied or protected target, full hold, wrong/expired source, manual cancellation and repeat collection. A correct refusal preserves the held cargo and ownership and grants no credit. An eligible destroyed trader's descriptive manifest alone is not a recovered payload.

Mining observes the real public `mining_collection_target` and `pending_ore` projections before and after intake and cold decoding. The exact salvage custody receipt API and downstream executable salvage cases are pending the owning core implementation. No positive salvage result is claimed here. The ordinary player save and original clue journal remain separate from these scratch acceptance journeys.
