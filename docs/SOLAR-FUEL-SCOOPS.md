# Solar fuel collection

This source candidate gives the existing paid Fuel Scoops utility its original solar-fuel effect in Meridian's real local flight scene. It does not create a cargo body, ore, mission receipt or purchase. The separate physical cargo intake paths remain open. Native acceptance is pending.

## Source rules and controls

| Condition | Behaviour |
| --- | --- |
| Outside three solar radii | No fuel collection |
| Inside three radii, outside the physical surface | A fitted scoop, forward speed greater than zero and nose dot product at least 0.8 are required |
| At forward speed 8 or above | Seven whole fuel units per second, with fractional carry |
| Below forward speed 8 | Collection rate scales with speed divided by 8 |
| At the actual tank limit | Stop at capacity, discard unused fractional carry |
| Inside four solar radii | Independent thermal radiation, with cooling proportional to temperature |

Buy Fuel Scoops in the Equipment utility slot for the original 1,900 CR at displayed TL4. It is one owned item; storage and refit retain the same identity. The existing XYZZY maximum can grant the real item. After activating the test commander, choose **Fit Fuel Scoops only** in Ctrl+Shift+X to test this utility without changing cash, fuel, or other equipment. A free ordinary utility mount is required for a new fit; an already-fitted item is a no-op.

Undock and fly toward the actual sun. The nose, not the selected camera, determines collection. Cruise toward it inside the collection radius. The cockpit shows actual fuel and cabin temperature, including no-scoops, heading, collecting and tank-full states. Danger begins at 0.75 and critical at 0.9; at 1.0 the actual hull energy falls to zero, regardless of shield fit or unlimited-fuel mode. Leave the corona to cool. Thermal exposure is independent of collection eligibility. Destruction freezes the shell on the terminal cockpit, including during catch-up; it is not a process error. Ctrl+R restores the last durable commander, Ctrl+S explicitly saves the terminal state, and N uses the existing confirmation before starting a replacement commander. Normal Quit preserves the previous durable save and asks for explicit Q to exit without saving. An existing accepted physical surface-contact refusal is left unchanged; fatal original surface collision and celestial mass-lock parity are separate source work.

## Account and cold continuation

Every whole fuel increment is a distinct no-cash account revision, tied to an actual accepted flight tick and the fitted scoop identity, not a market refill or a secret grant. The original station payment history is unchanged. Extended tanks supply only the real fitted capacity. The existing T04 trade trace can retain solar fuel continuity as a separate source, but solar fuel is not a paid refill and cannot replace its paid acceptance receipt.

The independent solar extension is absent from saves with no exposure. A current-format content fingerprint distinguishes the empty history, retained history and exact pre-solar v17 format, so removing a marked extension does not turn a current save into a legacy save. Compact runs retain actual accepted controls, opening poses, exact owner item, system/visit, physical clock, fuel/account prefix and whole-unit receipts. After the first exposure in a visit they retain exterior steps too, so a re-entry cannot skip a heat-producing excursion. Recovery replays accepted motion, verifies swept surface clearance and physically reachable origin, recalculates heat and fractional gain, and reconciles fuel and terminal damage with the existing account and resource replay. A dock or new visit resets local exposure, while historical receipts remain.

The v17 ordinary save base retains its independent 65,536-byte limit. The new `MSOL0003` trailer supports up to 8 MiB of additional evidence within the same atomically published commander file. It stores independently bounded chunks of up to 64 motion runs, 256 fuel receipts or 16 NPC refusal scenes, with a checked base boundary and exact chunk lengths. Completed chunks are shared in memory; normal steps only replace the current tail. Recovery replays all retained motion across chunk boundaries, with no discarded heat or receipt. Crossing a chunk or the former 64 KiB whole-file size has no effect on pilot control, repeated scooping or Continue. A safe stop retains its actual attempted control and refusal kind. Planet and sun use the same swept geometry as live flight; station, aperture, asteroid and the clue-bound fixed Mule refusal use their historical geometry. A traffic stop retains the existing authoritative historical NPC scene and verifies that the attempted sweep intersects a retained collider; NPC scene positions use the existing birth/clock/resource/reach validation and are not a new independent replay of every NPC movement.

At the real outer file limit, the live pilot can still steer, leave, return, scoop, jump and dock. A save or other durable transaction that needs a larger file is explicitly refused and the previous file remains intact; the cockpit warns when the solar history exceeds the file limit. Normal Quit fails safely and leaves the application open: P resumes with the unsaved live history; Q explicitly discards unsaved progress and quits. The previous durable commander can still be reopened. Unsaved live history remains in process memory, and there is no forced steering, teleport, invisible barrier or proof-erasing reset. This finite storage refusal is distinct from flight physics.

An earlier airborne save without this additive extension can establish its existing validated near-sun position as a legacy opening anchor; it receives no retroactive temperature, fuel or acquisition receipt. Genuine old saves without exposure retain their exact pre-solar encoding until solar history starts. Earlier saves lacking the shared resource birth-history owner do not gain a new solar producer. The immutable scene is Meridian's existing station-local solar placement, not an assertion of the original source's placement or physical astronomy.

## Immutable source

Revision `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`.

| Source path | SHA-256 |
| --- | --- |
| Sources/L337Core/SolarFuelCollection.swift | `f880a95ee99ecc1bd79e2a61c973ea8d5a2b1b57f6af60f4e0768b72d97e21d1` |
| Sources/L337Core/CelestialFlight.swift | `ded59f40174a34e1920b8ea47a3a27a2d0f1d920b402cf0a63fd48f1241dfc65` |
| Sources/L337Core/ScriptSolarSave.swift | `87b74b21e34c6d5993c0464bb6685532a44ae32f38288829dcca2e69ceead8c1` |
| Sources/L337Core/CargoScooping.swift | `6dae2421062621fee0f992d2648f501403c87cb87f77138127175eab6390ced7` |
| Tests/L337CoreTests/SolarFlightTests.swift | `b7e33d695ec90d954e048391f527822a7fc290f057a470d8455b06d871fda4a4` |
| Tests/L337CoreTests/FlightWorkGuidanceRuntimeTests.swift | `86f72c76fdf4d0e9441013a3623e6908a71585e52a6d5a4b506bc1d3061cb311` |

Authored regressions exercise the source thresholds, fractions and thermal safety; real earned paid fitting and extended fuel capacity; public flight into the actual local solar scene, real unbought fuel, pause and cold fractional continuation; independent test grant; unfitted fatal thermal exposure; dedicated refit identity and mount refusal; missing or altered history, malformed/truncated chunks and receipt rejection; repeated same-commander scooping and cold restoration beyond the former file bound; forced finite-limit and failed atomic publication rollback; and HUD/help presentation. The [exact protected-source inverse](../integration/sh001-mule/SOLAR-SCOOPS-INVERSE.json) preserves all earlier independent source pins.
