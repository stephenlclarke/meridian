# Automatic mining and salvage

Development integration candidate. The public acceptance below is staged, not a report of a passing build or a native game run. The current World owns target authority, work, tool operation, cargo custody and refusals. The desktop reads that state; it does not grant work, alter the target or create cargo.

## Controls and equipment

Use F to select Mining or Salvage, then Tab to select an eligible actual target. A healthy ship is a normal tracking target, not a salvage grant. A disabled ship can be tracked and approached, but that does not authorise taking its intact hull, crew or held cargo. The current recovery scope is the finite detached cargo of an eligible destroyed ordinary source. Selection of a work target starts automatic work without pressing Z. Delete or manual flight input ends guidance and work. F changes the filter and ends the current selection. Z remains manual use of the fitted front laser. T remains a missile lock; the missile and automatic-docking controls are unchanged.

The core and the separate matching targeting module must be fitted. Mining needs the fitted Mining Laser. Salvage needs an eligible source and the fitted Salvage Cutter for cutting. Complete physical recovery also needs fitted Fuel Scoops, a reachable actual payload and free cargo capacity. Owning equipment in storage alone grants no work capability. The automatic target readout reports actual approach, hold, tool and custody status; its text does not mean a pickup has already occurred. A missing fitting or capacity refusal must be visible and must not pay, create cargo or claim a completed recovery.

The original rock lock remains through depletion and its resulting ore acquisition. No second Tab, extra Z press or caller-supplied cargo award is part of that operation. Automatic intake must use the real released ore and scoop aperture. The intended local aperture is (0, -45, 55), the manual centre distance is strictly below 85 units and automatic relative speed must not exceed 15 units. These are acceptance constraints, not observations from a running build.

## Occupied cutter contact

When an actual disabled occupied ship reaches the accepted work hold and World exposes the current confirmation authority, a distinct warning opens for that exact target. This applies to both Ships/All selection and the Salvage filter. It does not open for ordinary healthy tracking or lawful external wreck recovery.

The modal stops accepted simulation ticks and clears held flight inputs. It shows the original target identity, occupied/claimed warning, attributed contact and the absence of verified detachable material. This does not grant permission from the crew. Enter confirms one reviewed contact through the normal atomic commander writer. Escape, target loss or loss of focus cancels the review and releases the original selection. A refused write or completed contact also releases the selection. A new warning needs a new deliberate selection; neither held Enter nor the next simulation tick repeats the contact.

The host consumes ordinary steering, fire, EMP, route, pause, help, Save and Quit inputs while the warning is open. The validated receipt is separate from any cargo or ship acquisition. An unlawful contact is explicitly described as such; no reward, people, intact cargo or hull entitlement is inferred. The authored native acceptance must verify the actual warning, frozen tick and unchanged pre-confirmation receipt, Escape with no new receipt, stale/focus cancellation, explicit saved confirmation, failed-write rollback, cold continuation and no repeat. These observations remain unrun.

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
3. The original rock loses its one finite unit. Observe the actual matching pending ore, source and fragment identity, release clock, velocity and the approved asymmetric Meridian shard with six vertices and an eight-face closed mesh (12 × 16 × 14 units). The target's collection projection and the ordinary renderer use this same body; this is not a rebuilt legacy cargo hull. One matching native commodity unit moves into the hold only after physical collection; the matching pending ore is removed. Wallet and paid fitting identities stay unchanged.
4. Save through the ordinary compatible writer, then quit and Continue. The one unit, depleted original body and consumed ore remain consistent. No active guidance resumes on Continue and later ordinary steps cannot receive a second copy.

The executable headless acceptance is `crates/meridian-core/tests/automatic_work_public.rs`. It exercises public World commands and the decoder. Actual native input, target readout, render visibility, Save/Quit and process restart still require the owning platform run. The test is authored, UNRUN against the joined core.

## Required refusal and salvage acceptance

Before qualifying native work, repeat the actual earned setup in separate scratch profiles. The public acceptance also creates a separately continued copy of the earned fitted commander, stores the paid scoop without a refund, selects the real rock and requires the actual `Fit fuel scoops` refusal. It checks that the rock is still intact, no pending ore or cargo exists for that work, wallet remains unchanged and Continue preserves those facts. For a full hold, use real public market transactions to occupy the native hold. Record the reason published by World; neither case may add an uncollected commodity or credit. Continue must keep the actual finite uncollected source, not replenish it.

For external salvage, fit the paid Basic core, Salvage Module (4,000 CR), compatible cutter (2,500 CR) and scoops. The total is 9,200 CR, before fuel or other required equipment; the mining funding route alone is insufficient. The public salvage case uses the same actual market journey, then buys 16 legal Gold units at Avenor and sells them at Renas. It checks the live ask, bid, stock, cargo and exact cash debit and return. It also buys four finite ordinary missile rounds (180 CR each) through the actual ammunition offers. The ordinary practice consent protects the finite local traffic actors during funding; the same actual practice closure used by the original wreck regression happens at Avenor before the encounter. No traffic reset or synthetic source is used.

Observe that an ordinary healthy ship is not eligible for automatic recovery. A live disabled hull may be approached; any separate occupied-ship contact needs its own explicit warning, confirmation and physical custody authority. The external-cargo case uses an actual eligible destroyed ordinary trader with its legal retained manifest, as described in the [wreck source record](WRECK-MANIFEST.md). This checks recovery of legal cargo, not whether the prior attack was lawful. It preserves the actual law consequences of the encounter. It uses real public flight, lock, finite ordnance and terminal contact, then selects the eligible source once through the Salvage filter. Confirm normal approach and cutter/scoop state in the target panel. The executable requires one finite cargo unit per accepted recovery interval, no cash or law mutation, exact original manifest identity, and a cold decoder check after each unit. The final Continue retains all and only the source quantity, and repeating selection cannot recover it again. External cargo recovery does not acquire the ship or unlock its purchase.

Record separate native refusal cases for missing cutter, missing scoops, occupied or protected target, full hold, wrong/expired source and manual cancellation. A correct refusal preserves the held cargo and ownership and grants no credit. An eligible destroyed trader's descriptive manifest alone is not a recovered payload.

Mining observes the real public `mining_collection_target` and `pending_ore` projections before and after intake and cold decoding. The external salvage case in `crates/meridian-core/tests/automatic_work_public/salvage.rs` uses the public `auto_salvage_eligible` and original `wreck_manifest` alongside actual native commodity quantities and wallet, not the manifest alone. It is authored and UNRUN against the owning joined source. The separate occupied-ship confirmation acceptance awaits its actual API and receipt. No positive native salvage result is claimed here. The ordinary player save and original clue journal remain separate from these scratch acceptance journeys.
