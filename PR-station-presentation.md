# feat: present the original station launch and docking loop

The flight preview had no station or launch control. This slice connects the World station scenario to the desktop shell, renders the original SH028 display mesh and adds L to request a tick-bound launch. Docking remains automatic after physical entry validation in the core.

## Implementation

A docked exterior camera faces the original port. Flight uses a pilot view near the entrance and the existing chase view farther away. Station presentation replaces the three fixture rings. CPU and GPU vertex capacity includes the original station mesh and is reused. Conservative contact status appears in the title without exiting. World advances the tick, retains the last clear pose and stops speed so the pilot can recover; contact never grants docking. Invalid command errors retain the complete state. This does not implement original contact damage or momentum parity.

## Validation

Authored a catch-up-frame regression that drives an actual overspeed station crossing followed by a clear tick and checks the refusal is still presented. Authored three independent presentation tests for transform order, port-facing camera coordinates and near-port station rendering with reused buffers. Existing mesh and flight tests remain. Rust formatting completed. Compilation, tests, strict Clippy and native graphics/controls are UNRUN for this slice until the core/content packet is joined and validated on the Dell.

## Compatibility and limits

The original free-flight World and mesh test mode remain available. The app starts in the new station scenario. Missing or invalid station data fails startup instead of substituting fixture geometry. The station native collision LOD is declared separately from the complete display. Full first-playable mission, economy and saves remain open.

Linked issue: [Station presentation](ISSUE-station-presentation.md).

## PR 1 quality-check correction

GitHub [PR 1](https://github.com/stephenlclarke/meridian/pull/1) reported a command-argument security finding in the station importer. The importer now selects only two fixed Git blob specifications, validates a real source directory, uses it as the subprocess working directory and disables external diff/text conversion. Regression tests reject unknown or option-like identifiers and non-directory inputs before Git runs; the SHA check still rejects foreign bytes. All nine Python import tests and exact original-source re-import pass. Rust source/assets remain byte-identical to the candidate that passed all 46 tests on both targets. The assertion-order annotation is addressed with explicit actual/expected variable names.
