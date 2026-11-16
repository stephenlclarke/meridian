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
