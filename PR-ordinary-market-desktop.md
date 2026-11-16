# feat(economy): trade seventeen native commodities with durable accounting

The desktop adds a read-only five-row market viewport in original display order. M toggles the panel, brackets select a commodity, and G/H trade one native unit through World. The panel shows actual current quotes/stock, wallet, used hold, legal value, and the selected cargo/reservations. Selecting or trading cancels any hidden freight review. Opening a market panel outside a berth grants no trade admission.

The UI keeps Food as the initial selection, consumes typed core commodity identities, and does not own mutable cargo or prices. A separate market presentation module keeps the screen and transaction notices outside mission handling. The existing profile test uses the same selected-trade commands as the keyboard; Food aliases remain in the core. Bracket and selection glyphs are original code-drawn additions.

## Joined core and original content

The complete slice joins immutable original definitions and Avenor/Renas quotes from content `4f4e3b6`, core transactions/save validation from `3abb761`, and desktop controls from `20d060e`, all on main `54b6920`. The content importer preserves all original definition fields, native display order, actual finite stock and old Food bytes. The core keeps one Food authority, separates ordinary cargo from identified G02 units and excludes actual reservations from sale. Native kg/g quantities consume zero tonne capacity. Actual successful launch records original cargo legality; market browsing grants no authority.

The same World transaction publishes stock, cargo and wallet together. The explicit v2 content/rules fingerprint and semantic ledger validation cover all commodity quantities, stock, money and legal history. A v1 save is refused before replacing the current World or file; no reset, migration, restocking or fallback is added. The installed v1 preview is retained. No dependencies, native station assets or mission receipts change.

## Verification

Authored behavioral checks cover all seventeen source-ordered selections, read-only rendering and navigation, viewport bounds, actual Gold kilogram purchase/sale with exact cash and stock deltas and zero tonne usage, cancelled hidden mission confirmations, airborne trade refusal, and glyph coverage for every displayed original name/control. These desktop tests and the ten new core behavioral tests are **UNRUN** on the joined source. The content owner passed 37 importer tests (12 new), exact-source extraction, and focused importer coverage 211/218 lines; Root verified all seven content and fourteen core component file pins. Those Python/source results do not prove Rust execution. Standalone Rust formatting and `git diff --check` pass. No compiler, GPU or installed-app acceptance is claimed for this component.

## Joined acceptance

Join exact frozen core/content components into one coherent artifact. Run the existing Dell owner compilation, tests, strict Clippy and asset/docs checks; get one independent whole-artifact review. Then qualify native Metal controls and representative tonne/kg/g trades, finite-stock/capacity/insufficient-funds refusals, mission custody and cold v2 continuation with the G1 owner. Do not silently load or reset a v1 profile, replace the installed preview before qualification, or infer graphics acceptance from headless tests.

## Compiler correction

The first exact joined candidate `bcddc62` failed compilation on both Dell targets because desktop catalogue callers assumed `station_services::Error` implements `Display`. Documentation, formatting and 37 importer tests passed per checkout; Rust tests, strict Clippy and binary production were not reached. Both lanes stopped without a retry. The correction maps typed catalogue errors to an explicit desktop content-read notice and uses Debug formatting in the affected test failure message. It changes no core API, accounting, content, save rules or behavioral assertion. The changed candidate requires delta review and a fresh owned qualification; no prior test pass is reused as compiler acceptance.

## Runtime fixture correction

Changed candidate `2b03a4f` passed native Windows compilation and 89 core tests, then failed one of 34 desktop tests. The airborne trade-refusal fixture tried to serialize immediately after launching at tick zero, before the first real simulation step. The unchanged navigation chronology guard correctly rejects one recorded launch with zero elapsed ticks. The fixture now advances one public simulation tick and explicitly verifies no dock context and a non-docked flight mode before taking its save baseline. Trade refusal, missing-berth HUD and byte-for-byte state preservation assertions remain exact. No chronology rule, production transition or acceptance assertion is weakened. The same owned Linux run is allowed to finish; results remain separate until its final receipt. The correction requires delta review and a fresh changed-head qualification.
