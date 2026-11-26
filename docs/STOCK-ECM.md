# Stock E.C.M. System — manual field and equipment

Source candidate only. The incoming missile encounter and automatic interception are **open**: the current game creates player-owned projectiles and NPC Pulse beams, but no NPC-owned missile launch. This candidate does not treat a beam, an outgoing player round, a notification or a test fixture as a hostile missile.

## Original evidence

Immutable original revision `a11fe16924d87d06021816f9037952afcbac08db`:

| Original file | Blob SHA-256 | Contract |
| --- | --- | --- |
| `Sources/L337Core/ECMControl.swift` | `e399ff153ae54f82aceb57148a5e05e946674b6494a313915441f6f81accdc59` | 22 energy; 32 game frames; independent activation identity and termination tally |
| `Sources/L337Core/Game.swift` | `e5b4e30c835d460aa7ca6f77364db0aaeaf877bb9d73bacc162e9111b26e2e17` | Fitted stock system; flight-only; energy strictly greater than 22; price 21000 tenths; technology index 4 (displayed level 5); manual and guarded automatic controls |
| `Sources/L337Core/EquipmentRuntimeCatalog.swift` | `ccf1ee2a940175f0d1a26a65aaa7b62ca7cf61d0eb33906f8b8feb6664210e27` | Unique `utility-ecm` inventory family |
| `Tests/L337CoreTests/OriginalCombatCostsUnitTests.swift` | `c1e024b22b885ee6395b188fbcf3add2b5310bdb88f9041d859759f918cd3487` | Exact energy debit and strict refusal at insufficient energy |

The Meridian SH025 uses a shared ordinary three-mount budget. Its stock ECM uses one mount and one equipment unit. The existing explicit four-tank test exception does not add another ordinary utility mount. The current maximum outfit retains its existing utility choices; fitting ECM separately requires a free ordinary mount. Fitted ECM must be stored before invoking TEST MAX if the latter would add the three other reserved utilities.

## Current candidate

- Buy one actual item in Equipment → E.C.M. utility at an admitted berth with displayed TL 5 or greater and 2,100.0 CR. Store and reinstall the same identity; no duplicate purchase or automatic grant.
- Press **K** in flight for one manual field. The original E key is not reused because Meridian already uses E for roll. The command spends 22 energy once, does not change heat or laser cadence, and does not advance the physical clock. The field lasts 32 accepted physical ticks in the same visit. A repeat during the field is refused. The HUD shows ECM FITTED or ECM ACTIVE.
- ECM and EMP remain separate. An EMP-disabled ship cannot activate ECM. An accepted ECM cancels an unfinished EMP charge and refuses a new EMP trigger while the field remains active.
- The activation ledger binds the fitted item, owner, hull, system, visit and accepted clocks. The shared resource replay checks the cost. The independent EMP meter replay checks every activation after that physical step's release or recovery, including the final clock of an electrical disable interval.
- Actual launch steps retain the initial native station geometry, controls and opening traffic scenes. The resource journal independently marks the accepted transition to local Flight. The jump owner records the actual departure, including whether ECM was already accepted at that command clock; completed and pending travel are independently matched to the navigation journal, and a physical berth records a separate cancellation marker. A retained activation cannot use a Launching or travelling clock.
- An additive tagged v17 extension preserves the local admission records and remaining field duration on cold load; an independent content marker prevents silently removing that extension. Genuine non-ECM saves retain their prior encoding; launching with fitted ECM creates its own admission records before a field is used.
- XYZZY initial activation enables automatic intent, as in the original. The separate guarded menu entries Fit E.C.M. only and Toggle automatic E.C.M. retain provenance and do not grant cash, fuel or unrelated items. The latter is a setting only. No automatic emission receipt is accepted until a real incoming ordnance owner exists.

## Acceptance and limits

The candidate authors paid acquisition/store/refit/rollback, shared mount and secret guard, strict energy edge, same-clock repeat refusal, 32-tick expiry, paused/stale refusal, retained history corruption and cold continuation checks. A separate real NPC Pulse encounter exercises cancelling an unfinished EMP charge without releasing it, followed by cold validation. The retained electrical replay has a labelled component fixture derived from a real pulse to test the exact disabling and recovery clocks independently of hull energy. Public launch and jump tests cover cold pending launch, completed and cancelled jumps, valid activation before same-clock departure and balanced-energy forged activations during travel. A labelled native-scene component tests an actual refused-motion geometry decision without claiming a produced encounter. Desktop cases cover actual menu and K dispatch. Rust and native preview remain unqualified until the owning platforms run the exact correction.

For native preview use a disposable commander only. Activate XYZZY if convenient, store one ordinary utility, fit E.C.M., then launch. Verify the ordinary item and price readout, toggle setting, press K, observe the actual energy bar decrease and ECM ACTIVE, wait for FITTED, then save and cold Continue. Do not claim destruction, warning or interception of an incoming missile during this preview; no such game object is currently produced. The complete playable ECM acceptance needs a real NPC-produced incoming projectile, real termination receipt, incoming danger presentation and automatic policy acting on that actual event. Audio/announcement and original broader ordnance variants remain outside this candidate.
