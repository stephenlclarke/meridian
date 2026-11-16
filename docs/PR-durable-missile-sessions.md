# feat(core): persist complete finite missile sessions

## Motivation and result

A finite missile launch previously made the session unsaveable. The version 8 core writes consumed ownership, complete live and ended ordnance and linked practice/combat evidence, then validates the entire candidate before Continue or in-place replacement. Old supported v4-v7 readers remain explicit; no used-v7 migration or ammo reconstruction is introduced.

## Implementation

The bounded controller retains last-motion scene evidence, accepted ordinary incidents and beam interceptions. The account writes actual optional item slots, and its revision accounting includes actual missile consumption. Practice receipts retain missile origins and pending/end quiet references. Explicit weapon/target tags separate delayed missile impacts from Pulse discharge rules. Decoding publishes no gameplay events.

## Validation and compatibility

Nine new tests are authored and unrun. Existing paid-gear/old-stock regressions are adapted to the supported v8 boundary. Formatting and source checks are recorded in the handoff packet. Root owns the desktop warning/status and host cold Save/Quit/Continue join; G1 remains the sole Mac compiler/native executor. Preserve prior files/previews and bind review/execution to the joined head/tree.

## References and risks

See [issue scope](ISSUE-durable-missile-sessions.md) and [format, invariants and execution criteria](DURABLE-MISSILE-SESSIONS.md). There is no PR number at author handoff. This change does not qualify full T02 or an actual player hit; native and cross-platform acceptance remain pending.
