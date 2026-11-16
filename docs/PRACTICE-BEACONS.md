# Avenor practice beacons

At the Avenor main berth, press F10 to review one safe gunnery practice sortie. Enter confirms and saves the choice. B cancels. After launch, continue into clear flight to open the range. Hold Z to fire the fitted front Pulse Laser. The HUD counts each distinct beacon hit and the beacon flashes white.

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

This slice grants no payout or lesson completion. The full T02 ceasefire,
physical reception and trusted trace, missile hits, ordinary damage and law,
and traffic parity remain open. The beacons participate in the Pulse Laser
ray. Ship-to-beacon physical collision remains open.

## Qualification

- At the actual Avenor berth, review and cancel, then review again, allow
  normal dock ticks, confirm, and cold Continue the persisted choice.
- Launch without consent and verify that no practice range opens. On the
  accepted departure, fly clear, acquire the distinct cyan targets and use
  the real Z trigger. Confirm a first hit, a repeat that leaves the count
  unchanged, and an obstructed shot that cannot credit a beacon.
- Verify active hit state across cold Continue and retirement after a real
  physical dock or jump. Verify a previously qualified v4 paid-bay and hot
  laser save loads in an isolated copy under the v5 executable.
- Run the joined workspace tests and strict Clippy. The desktop regressions
  cover delayed consent publication, cancellation, a real save replacement
  refusal, and cold reopen.
