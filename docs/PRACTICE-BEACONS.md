# Avenor practice beacons

These controls apply to the current Fleet build. Shift+5 reviews closure of an attempt. Shift+3 reviews a fresh attempt after closure. [Practice history and retry](SAFE-GUNNERY-ATTEMPTS.md)

At the Avenor main berth, press Shift+3 to review one safe gunnery practice sortie. Enter confirms and saves the choice. B cancels. After launch, continue into clear flight to open the range. Hold Z to fire the fitted front Pulse Laser. A newly hit beacon flashes white. Press H, then I, to check the distinct hit count in Flight Status. After the third hit, release Z for three continuous, unpaused flight seconds. Return to the actual Avenor main berth and press Shift+4 to review reception. Enter confirms reception and saves the Safe Gunnery Family Trace once.

The practice provider follows original L337 revision
`a11fe16924d87d06021816f9037952afcbac08db` and its
`NativePracticeSource.swift`, `NativePracticeRuntime.swift`,
`NativePracticeState.swift`, `NativeSafeGunneryRuntime.swift`, station board,
and safe-gunnery manifest. The three 6 m cubes are generated in code from the
original dimensions. Their immutable poses come from World. Production
geometry follows the accepted flight pose, and beacon contact competes with
the bound SH028 station hull for the nearest finite ray hit.

The profile publishes confirmed bytes through its exclusive save store.
Failure before replacement leaves live consent unchanged. If replacement
succeeds and directory synchronization fails, the game reports that the
new consent was written and the directory sync failed. Cold Continue retains
the practice range and distinct hit receipts. The first physical dock or
completed jump retires this attempt.

Reception has zero cash payout. The fitted front Pulse Laser route covers
the original three beacons, ceasefire and a separate physical reception. The
original missile route, ordinary actor damage and law, practice traffic parity,
and ship-to-beacon physical collision remain open.

## Qualification

- At the actual Avenor berth, review and cancel, then review again, allow
  normal dock ticks, confirm, and cold Continue the persisted choice.
- Launch without consent and verify that no practice range opens. On the
  accepted departure, fly clear, acquire the distinct cyan targets and use
  the real Z trigger. Confirm a first hit, a repeat that leaves the count
  unchanged, and an obstructed shot that cannot credit a beacon.
- Verify active hit state across cold Continue and retirement after a real
  physical dock or jump. Verify a previously qualified v4 paid-bay and hot
  laser save loads in an isolated copy under the v6 executable.
- Run the joined workspace tests and strict Clippy. The desktop regressions
  cover delayed consent publication, cancellation, a real save replacement
  refusal, and cold reopen.
- After all three distinct hits, verify a further discharge resets the
  interval, including a shot stopped by the station. Paused time adds
  nothing. A premature dock and Renas reception refuse. Complete the quiet
  interval in flight, physically dock at Avenor, review with Shift+4 and confirm
  after ordinary dock ticks. Cold Continue retains the same receipt and the
  cash balance. A second review cannot create another trace.
- Load a pre-reception v5 practice save in isolation. An active save must
  establish a new full quiet interval. A retired save cannot infer readiness.
