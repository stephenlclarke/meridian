# feat(desktop): expose the original seventeen-commodity market

The desktop adds a read-only five-row market viewport in original display order. M toggles the panel, brackets select a commodity, and G/H trade one native unit through World. The panel shows actual current quotes/stock, wallet, used hold, legal value, and the selected cargo/reservations. Selecting or trading cancels any hidden freight review. Opening a market panel outside a berth grants no trade admission.

The UI keeps Food as the initial selection, consumes typed core commodity identities, and does not own mutable cargo or prices. A separate market presentation module keeps the screen and transaction notices outside mission handling. The existing profile test uses the same selected-trade commands as the keyboard; Food aliases remain in the core. Bracket and selection glyphs are original code-drawn additions.

## Verification

Authored behavioral checks cover all seventeen source-ordered selections, read-only rendering and navigation, viewport bounds, actual Gold kilogram purchase/sale with exact cash and stock deltas and zero tonne usage, cancelled hidden mission confirmations, airborne trade refusal, and glyph coverage for every displayed original name/control. These tests are **UNRUN** until the complete G3 core and MC1 content are joined. Standalone Rust formatting and `git diff --check` pass. No compiler, GPU or installed-app acceptance is claimed for this component.

## Joined acceptance

Join exact frozen core/content components into one coherent artifact. Run the existing Dell owner compilation, tests, strict Clippy and asset/docs checks; get one independent whole-artifact review. Then qualify native Metal controls and representative tonne/kg/g trades, finite-stock/capacity/insufficient-funds refusals, mission custody and cold v2 continuation with the G1 owner. Do not silently load or reset a v1 profile, replace the installed preview before qualification, or infer graphics acceptance from headless tests.
