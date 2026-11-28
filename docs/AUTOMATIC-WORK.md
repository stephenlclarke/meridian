# Automatic mining and salvage

The Dell headless test on `200c0289` passed the paid mining journey, including collection and cold Continue. The corrected salvage journey on Dell at `63fb125c` recovered nine units and passed cold Continue. Mac preview verification is pending.

## Controls and equipment

Press **Tab** to select the nearest scanner contact, including contacts behind you. Each press selects the nearest remaining contact. After the farthest, the cycle returns to the nearest. **Delete** releases the target. **F** switches between assisted and manual control while keeping the contact marked.

| Equipment or mode | What the pilot controls |
| --- | --- |
| Hand flight, with any fit | Steer, set speed, line up with the docking entrance or target, and operate the fitted tool. A scanner mark works without a computer. |
| Basic targeting | The computer turns to face the contact and tracks ships in the crosshairs. The pilot controls range and presses **Z** to fire. |
| Advanced targeting | The computer can also approach and hold position relative to a rock or salvage target. The pilot can operate the tool. |
| Docking targeting and navigation computer | Selecting a station starts the docking approach. **N** also starts or cancels automatic docking. |
| Mining or Salvage targeting module and work equipment | Selecting the corresponding contact starts the work sequence. Occupied cutter contact still requires confirmation. |

Steering by hand takes over the current approach and keeps the scanner mark. Press **F** to resume assistance on that contact. **Delete** clears it. With Basic alignment, the pilot can still use the throttle. Identification adds legal status to the readout. Pirate targeting adds available cargo and route information. Military targeting adds shield and laser heat readings.

In manual flight, point the fitted mining laser at the selected rock and press **Z**. To collect released ore, select the fragment, match speed and bring it into the scoop below the nose. Press **Z** to collect. At an eligible wreck, match its motion at cutter range and press **Z** with the cutter and scoops fitted. A disabled occupied ship opens a review before contact. Press **T** to request a missile lock on the selected ship. Laser fire against a ship always needs pilot input.

For mining, fit the targeting core, Mining Module and Fuel Scoops. Carry a Mining Laser in storage or fit it in front. The computer fits the owned laser, holds the ship clear of the rock and fires. Keep the same rock selected. When ore breaks free, the computer follows the moving fragment and guides it into the scoop below the nose. Once collection finishes, the previous front laser returns. The six vertices that define the visible Meridian ore fragment also determine whether it fits through the scoop.

For salvage, fit the targeting core, Salvage Module and Fuel Scoops, and carry a Salvage Cutter. The computer selects the owned cutter. An eligible wreck has a finite amount of cargo. The cutter releases one unit at a time and the scoops move it into a hold with free space. A disabled ship with people aboard requires a separate decision before cutter contact.

## Occupied cutter contact

When an actual disabled occupied ship reaches the accepted work hold and World exposes the current confirmation authority, a distinct warning opens for that exact target. The review identifies the selected ship. It does not open for ordinary healthy tracking or lawful external wreck recovery.

The modal stops accepted simulation ticks and clears held flight inputs. It shows the original target identity, occupied/claimed warning, attributed contact and the absence of verified detachable material. This does not grant permission from the crew. Enter confirms one reviewed contact through the normal atomic commander writer. Escape, target loss or loss of focus cancels the review and releases the original selection. A refused write or completed contact also releases the selection. A new warning needs a new deliberate selection; neither held Enter nor the next simulation tick repeats the contact.

The host consumes ordinary steering, fire, EMP, route, pause, help, Save and Quit inputs while the warning is open. The validated receipt is separate from any cargo or ship acquisition. An unlawful contact is explicitly described as such; no reward, people, intact cargo or hull entitlement is inferred. The authored native acceptance must verify the actual warning, frozen tick and unchanged pre-confirmation receipt, Escape with no new receipt, stale/focus cancellation, explicit saved confirmation, failed-write rollback, cold continuation and no repeat. These observations remain unrun.

For an eligible stationary wreck, the approach predicts the remaining ordinary braking path. If that path would enter the hull-clearance boundary, it brakes and turns clear before resuming the hold approach. The turn is retained until the ship stops; manual controls or a new selection cancel it. This does not strengthen braking or bypass any swept collision checks. At an already unavoidable immediate contact, the normal collision refusal remains authoritative. This correction is authored against the earned recovery trace; a new physical replay remains required.

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

Launch from the Avenor berth. Press **Tab** until the first one-unit rock is marked. The original headless acceptance used the belt's real rock ID. The scanner integration tests exercise the distance-ordered selection command.

1. Selection alone gives no cargo. Turn by hand to take control. The rock stays marked. Press **F** to resume assistance on the same rock.
2. Thereafter supply only ordinary unheld flight steps. The same original rock identity must remain selected through actual depletion and automatic ore approach. Do not reselect, fire manually or call a collection command.
3. The original rock loses its one finite unit. Observe the actual matching pending ore, source and fragment identity, release clock, velocity and the approved asymmetric Meridian shard with six vertices and an eight-face closed mesh (12 × 16 × 14 units). The target's collection projection and the ordinary renderer use this same body; this is not a rebuilt legacy cargo hull. One matching native commodity unit moves into the hold only after physical collection; the matching pending ore is removed. Wallet and paid fitting identities stay unchanged.
4. Save through the ordinary compatible writer, then quit and Continue. The one unit, depleted original body and consumed ore remain consistent. No active guidance resumes on Continue and later ordinary steps cannot receive a second copy.

The executable headless acceptance is `crates/meridian-core/tests/automatic_work_public.rs`. It exercises public World commands and the decoder. Actual native input, target readout, render visibility, Save/Quit and process restart still require the owning platform run. The earned mining case passed on Dell at `200c0289`.

## Required refusal and salvage acceptance

Before qualifying native work, repeat the actual earned setup in separate scratch profiles. The public acceptance also creates a separately continued copy of the earned fitted commander, stores the paid scoop without a refund, selects the real rock and requires the actual `Fit fuel scoops` refusal. It checks that the rock is still intact, no pending ore or cargo exists for that work, wallet remains unchanged and Continue preserves those facts. For a full hold, use real public market transactions to occupy the native hold. Record the reason published by World; neither case may add an uncollected commodity or credit. Continue must keep the actual finite uncollected source, not replenish it.

For external salvage, fit the paid Basic core, Salvage Module (4,000 CR), compatible cutter (2,500 CR) and scoops. The total is 9,200 CR, before fuel or other required equipment; the mining funding route alone is insufficient. The public salvage case uses the same actual market journey, then buys 16 legal Gold units at Avenor and sells them at Renas. It checks the live ask, bid, stock, cargo and exact cash debit and return. It also buys four finite ordinary missile rounds (180 CR each) through the actual ammunition offers. The ordinary practice consent protects the finite local traffic actors during funding; the same actual practice closure used by the original wreck regression happens at Avenor before the encounter. No traffic reset or synthetic source is used.

Observe that an ordinary healthy ship is not eligible for automatic recovery. A live disabled hull may be approached; any separate occupied-ship contact needs its own explicit warning, confirmation and physical custody authority. The external-cargo case uses an actual eligible destroyed ordinary trader with its legal retained manifest, as described in the [wreck source record](WRECK-MANIFEST.md). This checks recovery of legal cargo, not whether the prior attack was lawful. It preserves the actual law consequences of the encounter. It uses real public flight, lock, finite ordnance and terminal contact, then selects the eligible wreck. In the cockpit, press **Tab** until its scanner marker is selected. Confirm normal approach and cutter/scoop state in the target panel. The executable requires one finite cargo unit per accepted recovery interval, no cash or law mutation, exact original manifest identity, and a cold decoder check after each unit. The final Continue retains all and only the source quantity, and repeating selection cannot recover it again. External cargo recovery does not acquire the ship or unlock its purchase.

Record separate native refusal cases for missing cutter, missing scoops, occupied or protected target, full hold, wrong/expired source and manual cancellation. A correct refusal preserves the held cargo and ownership and grants no credit. An eligible destroyed trader's descriptive manifest alone is not a recovered payload.

The current external transfer uses a direct cutter-to-scoop feed. The terminal ordinary manifest is finite; it has no approved independent debris body or native cut-section asset. The admitted SH025 clearance geometry supplies the near external face, and the existing lower-forward scoop aperture supplies the receiving point. Both endpoints must be inside the supported scoop reach, and the transfer must leave the source clearance shell towards the actual fitted scoop. Live and cold admission use the same stationary speed, radial hold and facing checks. Each receipt retains the actual player position, normalised quaternion, speed, source identity and both fitted item IDs. The receipt has an explicit direct-feed format; older radial-only development receipts are refused because they cannot prove the missing pose. Ordinary saves without salvage history retain their encoding. Tests alter the actual first receipt's speed and facing with a repaired checksum, both immediately and after a second receipt, and require semantic refusal. No independent moving cargo body or native hull cutting is claimed.

Mining observes the real public `mining_collection_target` and `pending_ore` projections before and after intake and cold decoding. The external salvage case in `crates/meridian-core/tests/automatic_work_public/salvage.rs` uses the public `auto_salvage_eligible` and original `wreck_manifest` alongside actual native commodity quantities and wallet, not the manifest alone. A corrected Dell run on `63fb125c` recovered all nine units and confirmed that selecting the wreck again after Continue could not duplicate them. The desktop tests on `d0f7daba` loaded a genuine held encounter. They verified confirmation, cancellation, failed-write rollback and cold Continue on Mac, Linux and Windows. Interactive Mac verification remains open. The ordinary player save and original clue journal remain separate from these scratch acceptance journeys.
