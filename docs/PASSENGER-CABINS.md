# Passenger cabins and G04 cabin journey

Development source candidate for the ordinary SH025 preview. Compiler, hosted checks, independent review and native gameplay are pending.

## Paid accommodation

At an admitted main orbital station, open Equipment and select **Passenger cabins**. Select a grade, review the purchase and confirm. Each purchase fits a distinct five-tonne, one-equipment-unit cabin. It is independent of all other cabin items. The remaining hold is available to freight, ordinary market cargo and existing reservations. A stored cabin retains its identity and original paid receipt; fitting it again does not charge the wallet. Only an empty cabin can be stored. Refitting requires the actual free space and equipment unit again.

| Grade | Price (CR) | Places | Modes |
| --- | ---: | ---: | --- |
| Packed Stasis | 1,000 | 20 | Stasis only |
| Economy | 2,500 | 8 | Awake and stasis |
| Standard | 5,000 | 4 | Awake and stasis |
| Luxury | 10,000 | 2 | Awake and stasis |
| Single Stateroom | 20,000 | 1 | Awake and stasis |
| Double Stateroom | 20,000 | 2 | Awake and stasis |

The original shared-project layout balance is retained. All six have working life support and stasis hardware at purchase, as in the original healthy condition. A packed cabin offers no awake places. A private booking reserves the complete physical cabin even if other places exist. The ordinary prices apply at currently admitted Avenor, Renas and Lareria main orbitals; PLUGH special pricing is not in this scope. There is no free initial cabin, no passenger grant and no change to the XYZZY maximum outfit. The original 720 CR starting cash cannot buy any cabin.

## G04: Consented cabin journey

After earning enough through ordinary transactions, return to Avenor and fit two different awake-compatible cabins. Two Economy cabins cost 5,000 CR and use 10 tonnes and two equipment units. A single Double Stateroom or Economy cabin does not satisfy the two private reservations. The distinct Packed Stasis grade does not qualify for this awake passage.

Open the separate **G04 passenger service** slot in Equipment. Select **Request replies**, then review and confirm. The actual finite original waiting people, Mira Talen and Oren Vey, reply to the current Avenor-to-Renas offer. With insufficient fuel, fewer than two eligible fitted private cabins or a stale fitting, request fresh replies. Continue invalidates unconfirmed offers; a fresh request is necessary before boarding. Only then can the separate **Board both people** action commit their private allocations. The two passengers have their original owner-bound batch and identities. Purchase, a route display or arrival alone does not transfer anyone.

Leave the actual Avenor berth, make the 3.365 LY jump using the normal flight controls and dock at the Renas main orbital. Confirm **Renas reception / receive both**. This releases the two private reservations, retains the original people and their reception history, credits 610 CR once and records the trusted `l337.unwritten-route/clue/cabin-journey-family-trace` clue in an owner-bound terminal receipt. The separate Unwritten Route journal candidate consumes this receipt; Shift+J is not part of this standalone PR. Arrival by itself credits nothing. Before reception, the same two living people can instead be released to their actual Avenor custodian using **Avenor custodian / release both**, without payment or clue. Neither terminal path can be repeated or produce new source people. Boarding uses one of the three shared Original active service slots.

Save publication is prospective and transactional. A failed purchase, boarding or reception write leaves the current account, cabins, occupancy and clue unchanged. Continue reconstructs ownership, offers, the fitted private allocations and both owner-bound terminal receipts from the bounded save event ledger. New v17 module tags 40–45 identify the six physical cabin grades; event tags 4–7 identify the four G04 service operations. In this fuel-compatible build, bit 30 of the module event count declares accepted-berth and capacity history; bit 31 remains the existing fuel-storage cargo history. Existing no-cabin/no-passage fuel-compatible v17 records retain their earlier encoding; older executables do not accept the new marker or tags. Consent binds historical 16-bit fuel, epoch and fit. The actual four-tank 410-unit explicit test fit may request a refusal when fewer than two awake cabins are fitted, but may not board people while outside ordinary capacity. Active G04 custody also prevents the explicit four-tank MAX. Boarding, reception and cancellation must reference the exact accepted berth retained by the World; a completed jump alone never creates such a receipt.

From the first cabin or passage operation, retained cargo-bay purchase/refit/store and freight custody receipts join the existing commodity and module histories by account revision. Replay checks cabin mass, real cargo, the T04 navigation-trade one-tonne reservation and fitted equipment units at every combined transition, including those followed by later storage or sale. The T04 reservation begins at the retained plan revision, becomes actual cargo at its designated purchase (with no double count), or ends after the exact account revision of a new pre-purchase cancellation. New cancellations retain that revision; an unbound legacy cancellation must demonstrably precede cabin activation. The first activation also requires an evidenced legacy boundary: no held original freight, a closed freight prefix with exact final custody closure if it is non-empty, and an unowned, once-purchased or exactly evidenced cargo bay (from the existing test-tank cargo history). An old save with unrecorded bay refits or an unproven prior G02 custody closure remains loadable, but cabin/passage activation refuses rather than fabricating historical receipts. Complete an active old freight job before first activation. New bay refits and freight transactions are recorded after activation. Historical accepted berths must have monotone world-minus-physical tick offsets within the current saved clock bound when physical-clock evidence is available.

The [exact passenger-cabin inverse](../integration/sh001-mule/PASSENGER-CABIN-INVERSE.json) reverses only the 19 affected historic protected files to the immutable pre-cabin source. All 115 original core files remain checked through the existing inverse chain; unrelated edits or a modified manifest refuse.

Limits: the preview has no independent cabin component deterioration, life-support repair or hull-loss passenger receipt system. The healthy working hardware and private awake occupancy are implemented; the source's fatal-loss settlement and general scripted stasis/crew/passenger contracts remain separate. This slice does not claim an implementation of original contract-board presentation or full original G04 acceptance. A real four-tank 410-unit cold refusal regression is retained from the combined source. The cold-profile Shift+J regression belongs to the separately sealed combined candidate. Exact standalone qualification remains pending.

## Immutable original provenance

Original revision `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`, verified from the isolated historical object store. No human working checkout, player profile or authored artwork was copied.

| Original reference | SHA-256 |
| --- | --- |
| `Sources/L337Core/PassengerCabinGrades.swift` | `12fac9619f733666db4cba1c9f83732ade9f1b29ff19318b17d9b04813a6334c` |
| `Sources/L337Core/PassengerFitting.swift` | `760c8ddad3e4e01d222b6c3b8bb7e025dd6b1994641d530121a325e85b2b2524` |
| `Sources/L337Core/NativeStationPassengerJourney.swift` | `033849260f41968d1a06005ceb2325295cf469ee75031922b4f63bc8d7d1d0ac` |
| `Sources/L337Core/NativeStationPassengerJourneyBoard.swift` | `197ee1cafde2b7c5119339cc79392c3a23f78bc16c53394f50228dbdb58a7cf4` |
| `Sources/L337Core/NativeStationPassengerJourneySave.swift` | `2a1ca3571c9b5ba514d80330f32513ebc85234f2e9ce0904823199d742222713` |
| `docs/meridian/missions/gameplay/G04-CABIN-JOURNEY.md` | `a7072ebde0e41573285edddf79d0c6ead6ebf7f3695bdbd807b656da15a94861` |

Authored regressions cover all source grades, unpaid refusal, genuine paid fitting and storage, hold capacity shared with ordinary trade, freight and T04 reservation, fresh and stale replies, packed-stasis and one-cabin refusal, exact private occupancy, real travel and Renas reception, failed writer rollback, forged historical fuel and arrival-only docking, monotone physical/world clock bounds, stripped/truncated history marker, missing bay or freight receipts, transient capacity overflow with a plausible final hold, T04 reservation-aware purchase and cancellation across cold Continue, 410-unit receipt codec, safe cancellation, single payment, namespaced clue and cold Continue. The desktop regression drives the separate request review with ordinary controls and reopens the persisted refusal. Owned execution remains pending.
