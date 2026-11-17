# Cockpit instruments

The cockpit adds separate shield, fuel, laser heat, speed and four-bank energy instruments, a player-relative scanner, finite missile icons and selected-view station direction. The renderer receives one immutable presentation value captured from the simulation owner. It cannot issue commands or change a save. Existing station and confirmation panels remain available above the instruments, and the title screen keeps its existing presentation.

The layout uses the frozen L337 cockpit as a style reference: a yellow separator, left resource gauges, central green scanner and right flight/energy instruments. Labels use logical points; Retina backing resolution does not change cockpit proportions. The original outline geometry is retained for the missile icons. Existing independently drawn Meridian glyphs remain; this change imports no L337 bitmap-font table or Elite/Oolite art. [The reference manifest](../assets/legacy/hud-reference-manifest.json) records the immutable source hashes and provenance limits. Source presence does not establish authorship or permission for unrelated historical content.

## State and controls

Shields, energy, laser heat, fuel, missiles, lock state, speed, station and beacon/projectile contacts come from current World snapshots. Gauge capacities come from the same read-only constants used by the simulation. Laser heat follows the source's heat/255 display scale. Scanner contacts use front ship coordinates, independent of the selected camera; station bearing and elevation use the selected camera. Destroyed and docked states take precedence over other flight labels. The HUD does not claim Safe or Hostile without an active hostile-body model.

The existing Meridian keyboard controls, click-to-focus behavior, station reviews and save/quit/refusal rules remain. The original flight cockpit has no pointer hit regions for its passive gauges; this implementation adds none. Docked-screen interaction and the final coherent controls/help map belong to the shared desktop join. There is no requirement for old L337 keyboard, menu or save-file compatibility.

Cabin temperature, altitude and roll/pitch deflection currently show N/A. Their missing dependencies are thermal simulation; planet position/radius and an altitude definition; and authoritative control deflection rather than Euler attitude. Further core parity needs active hostile-body/EMP status, targeting identification and fitted autopilot state. These are implementation gaps, not permanent feature exclusions. The current public capacity values describe the existing SH025 fit; future modular equipment must supply its actual current capacities through the same authority boundary.

## Validation and limits

Focused HUD tests verify actual launch/thrust/fire changes, full/unknown resources, save invariance during drawing, rear/side station bearings, finite/clamped geometry and bounded vertex reuse with all fourteen existing control rows at compact and large sizes. These tests establish presentation behavior and resource binding, not a completed game or original pixel identity. Ten focused tests and strict desktop Clippy passed on the author branch before this documentation update; the raw Mac evidence is retained separately by G1.

Native current-source visual and input comparison remains pending. No authenticated original cockpit image has been located, so direct original screenshot comparison remains unobserved. Station compass panel placement, close docking roll/spin cues, original target panels, flight projection/clip and the existing verbose overlay still need integration work. Existing published previews and player profiles are unchanged by source authoring.

## Joined equipment feed

The next-preview candidate now captures all nine fitted missile positions and every retained projectile through the complete World APIs. The renderer reuses a bounded projectile buffer; the scanner has fixed space for every checked-u8 launch identity. The previous three-entry compatibility snapshot is used only for arm/lock controls. A new real paid-item test checks the ninth pylon without mutating the save. These joined Rust changes and the three new desktop tests are authored and unrun; the earlier ten focused HUD passes apply only to the original HUD source. Docked screens and the rest of modular equipment remain in progress.

## Flight overlay and controls

The next-preview candidate uses a compact default flight overlay for current status, notices, missile safety, practice state and jump countdown. Instrument gauges carry the numeric ship readings. `/` opens the keyboard help; toggling help preserves pending reviews and does not issue a gameplay command. Confirmation panels retain priority. Two new regression tests cover warning/control visibility and unchanged World/review state. These composed changes remain uncompiled and unqualified until the complete menu/equipment join is reviewed.

## NPC scanner source awaiting integration

The source based on `ccf0aa948a3e7de67ddefe47c9f47c3150a78741` adds actual SH025 traffic from `World::traffic_snapshot()` to cockpit capture. It reserves four additional slots from `MAX_TRAFFIC_ACTORS`, after the unchanged station, three practice and 256 possible retained-projectile slots. No original contact is displaced to make room for an NPC. Four contacts add at most 48 scanner vertices to the existing fixed buffer allowance. Rendering reuses that buffer; this is not a measured memory or FPS result.

NPC positions use the same player-relative quaternion transform as the original scanner contacts. The scanner stays in front ship coordinates for all four selected flight views; it does not rotate with a rear or side camera. Station direction continues to use the selected view. Existing scanner range/elevation projection and edge clamping remain presentation conventions, not a new sensor-range or targeting authorization.

Every non-Complete phase retains its contact, including a Destroyed solid body. Only the core's actual Complete/berthed state is omitted. Yellow means an ordinary live admitted SH025 contact with wanted level zero; red means its actual wanted level is positive; blue means a retained destroyed body. These colors describe current core facts. They do not assert hostility to the player, authorize fire, grant a lock or create a police response. The existing flight/status, missile arm/lock and resource gauges retain their own World authorities. Unsupported roles or hulls never receive an SH025 substitute contact.

`Cockpit::capture` now returns `Result<Cockpit, String>`. Unsupported hull/role/frame, non-finite contact or excess actor count refuses the complete capture without truncation or partial contact publication. Root owns the required App result handling before presentation, the same-frame renderer snapshot join and all Session/target controls. The source packet alone is not runnable integration; a fallback that silently omits active NPC contacts is not accepted.

Eight focused tests are authored **UNRUN**. Public-World positives cover actual trader/pirate poses, current gauges/security facts, four view selections with the original scanner basis, and preservation of real station/practice/launched-projectile slots. Explicit descriptive fixtures cover translated and rotated player coordinates, all ten phases, colors, four-contact capacity, preservation of all 260 original slots, atomic rejection and finite/clamped geometry with buffer reuse. These fixtures do not create World actor, damage, targeting or sensor authority. Existing HUD assertions remain unchanged, with only successful Result unwrapping added to their capture calls.

Whole-composition source review, focused test execution, required CI/coverage and actual rendered scanner/native control/save/quit/cold validation remain pending the owning executor assignment. No compiler, tests, Clippy, native app or release ran in this authoring pass. The published v0.5 build and all prior frozen renderer packets remain unchanged.

The normal HUD keeps status, current warnings and a small H/? help hint. The window title omits the control list. H or ? opens host modal help for the current flight view and fitted weapon. Underlying flight, firing, saves, transactions and quit commands wait until Escape/H/? dismisses it. The game clock and held input reset on opening/closing so a held thrust/fire key cannot replay on dismissal. Help itself has no saved state.

The Help candidate now lists mission details and flight-lesson review/cancel/details in both the station HUD and flight popups. Its retained minimum-window regression checks the concise HUD hint, all those popup controls, complete card text, bounded rendering and unchanged saved state through docked, launch and all four flight views. This changed regression and help content are authored; qualification remains pending.
