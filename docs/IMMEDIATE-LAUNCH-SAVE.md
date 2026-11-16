# Immediate launch save correction

Historical feature contract and author-stage notes. For current delivered behavior, exact validation and open limits, read [implementation status](IMPLEMENTATION-STATUS.md). Earlier UNRUN statements below apply to the original handoff, not the latest delivered source.

This bounded core correction starts at exact joined source
`2b03a4f4a440ba0dd5ff13dcbb45f553fdca070a`. The execution owner reported both
Dell platforms compiled that source and its 89 core tests passed, but the
desktop test that launches and immediately saves at tick zero failed. Those
results belong to that source, not this correction. Root restores the original
desktop test separately; this component changes only core and this document.

## Cause and derived invariant

`World::request_launch` requires Docked mode, checks the supplied current tick,
then calls the checked navigation launch transition. It does not advance the
simulation tick. That transition increments both visit and launches and consumes
the current dock admission. A first legal launch therefore has tick zero,
launches one, dock sequence one and no dock. The old save validator's
`launches <= tick` rejected this actual public producer.

The navigation producers establish a stronger relation:

- Initial state has dock sequence one, launches zero and one current admission.
- Launch consumes exactly one current admission and increments launches once.
  The World mode guard prevents consuming it twice.
- A physical entry is published only by a step changing airborne to Docked;
  it increments dock sequence once and creates one current admission.
- Docked steps, trade, contact, review and pause commands do not add admissions.
  Jump requests and jump completions do not create a dock admission.

Consequently `launches + (mode == Docked ? 1 : 0) == dock_sequence`.
Each admission after the initial one requires a distinct successful simulation
step, so `dock_sequence - 1 <= tick`. Together these imply
`launches <= tick + 1` while airborne and `launches <= tick` while docked.
The implementation checks the stronger admission relations directly. It never
adds one to a u64 tick, so maximum tick cannot overflow this check. The existing
nonzero dock sequence guard precedes subtraction, and the exact launch sum uses
u128. Existing wide visit equality, jump, fuel, identity, physical dock,
pending-travel and World accounting checks remain.

No producer, clock, frame callback, command order, save format, compatibility
version, legality rule or stock changes. No forced step, fake tick, validation
waiver, repair or reset is introduced. Valid version-two saves retain their
existing representation; malformed chronology receives stricter checks.

## Authored verification and acceptance

Three new tests are authored:

- Public tick-zero purchases of original Narcotics and Gem-Stones, immediate
  launch, save without a step, and fresh World Continue retain Launching mode,
  tick zero, exact launch pose/speed, owner/hull, legal record two, cash 645,
  native cargo, tonne occupancy and no dock or minted mission freight.
  Duplicate launch and airborne trade refuse with exact saved bytes unchanged.
  Subsequent genuine public flight, two jumps and physical docking return to
  the original Avenor stocks minus the two paid native units; cash and legality
  persist and fuel is charged once per jump.
- Resealed malformed saves test double consumption of one admission, balanced
  but impossible extra admission at tick zero, zero sequence, wrong visit and
  maximum-u64 launch/jump values. Refusals preserve both live observables and
  complete encoded bytes.
- Internal arithmetic boundary tests verify launch visit/count exhaustion and
  physical admission exhaustion return SequenceExhausted before mutation; valid
  docked/launching chronology also validates at maximum u64 tick without adding
  one to that tick.
  These are arithmetic fixtures, not public navigation authority or flight proof.

Rustfmt, scoped Markdown and whitespace checks pass. Compilation, execution of
the three new Rust tests, strict Clippy, delta review and actual desktop test
re-execution are **UNRUN** for this correction. The execution owner must join the
core correction with Root's restored original desktop immediate-save test, run
the exact resulting source on both Dell platforms, preserve all original
immediate-save/no-dock/refusal/byte assertions, and report its exact commit and
results. No native Mac build, retry, deployment or preview replacement is
performed here.
