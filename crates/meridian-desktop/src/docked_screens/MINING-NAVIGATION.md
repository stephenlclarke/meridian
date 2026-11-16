# Mining navigation projection

Subsequent mining work only. This increment is based on Root's frozen `9129c6f65ffea64a099bc1022c4e24de40487b13` (the `e30fb82` preview candidate plus MC1's core composition). It does not change the current preview candidate. The seven new tests and compilation of this increment are **UNRUN**; G1 owns compiler and native acceptance. Prior tests of the five menus are separate evidence.

## Provider and displayed facts

`mining_navigation_bodies(&World, selected_system)` reads `World::mining_destination` and `World::mining_belt_snapshot`. No provider means an empty vector, including explicit legacy profiles whose core mining state is absent. No fallback catalogue marker, seeded asteroid or guessed location exists in this adapter.

In the current Avenor system, the provider yields 13 lightweight descriptors: its belt centre at `[4000, 0, 8000]` and all 12 real bodies, beginning at `[4000, 0, 6000]`. Positions come directly from World in the main station's local frame. Asteroid `(system, ordinal)` identities are copied unchanged. The centre uses `MINING_BELT_CUE_ORDINAL = u64::MAX` solely as a navigation descriptor; it is never an asteroid ID, physical hull or extraction target. No resource quantities, vertices, depletion state or work capability are copied into UI ownership. Exhausted bodies remain present because the adapter never filters on ore quantity.

Current-system distances are measured from the actual player position in the same coordinate frame, in world units. Browsing Avenor from another system displays only the genuine centre descriptor; its range is unavailable and local selection is refused. The remote origin cannot be subtracted from Avenor coordinates. System travel remains the ordinary admitted hyperspace route.

The Galactic Chart's scrollable facts identify supplied local destinations and their coordinate frame. Its existing Details control opens System Data. This does not place station-local metres/world units on a galactic light-year chart. System Data pages through the centre and rocks, marks the selected row, and displays its station-local XYZ cue. Pointer selection and semantic `NextBody`, `PreviousBody` and `BodyRoute` act on the same identity. Selection emits the existing `ReviewBodyRoute` intent only. It never moves the ship, engages autopilot, fires a tool, extracts cargo or changes a save.

The painted frame retains lightweight navigation descriptors for the selected system. Map input refuses if that list, identity, location, measured range or availability changed, even when the host mistakenly reused a stamp. Ordinary host session/revision checks remain required. The descriptors are presentation snapshots, not a second provider or persistent ledger.

## Proposed Root host join

The author changed only `docked_screens/**`. The packet contains an exact proposed `host-join.patch` against base `9129c6f` for Root to inspect and compose into its evolving `docked.rs`; it is not applied here. Its changes are:

1. Add `navigation_bodies: &'a [ui::NavigationBody]` to the private `context` helper and use that slice in `ui::Context`.
2. Immediately before each context construction in `build_frame` and `input`, compute `ui::mining_navigation_bodies(world, self.screens.selected_system())` from the same actual World and retain the vector for the context borrow. Do not cache provider facts across input, arrival, Continue or profile replacement.
3. Handle `ReviewBodyRoute(id)` by requerying this helper, finding the exact ID, requiring the actual current system and `can_target`, and recording only an ephemeral local position-cue selection. Resolve the selection again when rendering a flight cue; if it vanished, is foreign or unavailable, display none. Do not turn a descriptor into a mining/tool authority or call a flight-position setter.
4. The proposed `selected_body_cue(&World)` method returns a fresh descriptor for Root/G1's actual position cue. It contains no ray, mining result, ore count or fabricated autopilot completion. Root/G1 still own rendering and actual flight controls. Manual flight reaches the position through ordinary simulation.
5. Root owns actual context-sensitive key bindings and visible hints for the three body actions. Pointer buttons already work through the existing semantic dispatch. Clear held flight input on menu entry and retain normal no-click-through rules.
6. Clear the painted frame, pending review and selected cue on commander replacement/Continue. Give a new host session identity to stale-session validation. The cue may be discarded on launch, arrival or reopening without changing gameplay state; selection is not a new persisted capability.

The patch retains Root's existing prepared/durable actions for purchases and equipment. Its body selection path has no payment or World mutation. It must be independently reviewed and compiled with Root's actual joined state, MC1's provider and G1's cue renderer before acceptance. An external `legacy-provider-test.rs` snippet accompanies it for the core owner to extend the real explicit-v11 loader test: the test-only legacy encoder is private to core, so this leaf author does not expose it or manufacture save bytes to claim a desktop legacy test.

## Validation to execute

The seven new tests exercise the actual provider: centre and all 12 identities/positions/ranges; missing provider and unsupported Renas; real hyperspace travel followed by remote inspection with no false local distance; pointer/keyboard selection and unchanged World bytes; changed provider location/availability/identity/removal with an unchanged stamp; newly available bodies against an older empty frame; and actual save/Continue preserving navigation descriptors. Negative provider fixtures in the stale-frame test are explicitly UI fixtures and do not mutate core.

Root/G1 must run the focused tests and strict Clippy after the serial join, then inspect actual pixels at supported sizes and high DPI. Qualify real body selection, the position/range cue during normal flight, absence after legacy Continue, no foreign-system cue, selection invalidation, and approach to the actual renderer/collision geometry. Mining equipment, work admission, resource costs, cargo conservation, depletion and durable continuation remain MC1/G3/Root's separate mechanics and qualification. No new preview or release is authorized by the existence of these source files.
