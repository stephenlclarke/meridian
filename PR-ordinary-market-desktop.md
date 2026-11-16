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

## Immediate launch/save chronology failure

Changed candidate `2b03a4f` compiled on both Dell targets and passed 89 core tests plus 33 of 34 desktop tests on each. The failing test starts a valid commander and calls the public launch command at tick zero, then immediately saves. The save validator rejects that state because a successful launch records one launch before any simulation tick. This exposes a real producer/validator chronology inconsistency. The temporary `d644d38` test delay is withdrawn and superseded; the original immediate-save, trade-refusal, missing-berth and exact byte-preservation assertions are restored. The core owner is correcting the transition-derived chronology rule with public immediate-save/cold-Continue and invalid-count regression coverage. No new qualification is authorized until the complete production correction is reviewed. Strict Clippy and native build were not reached by either failed run.

## Production chronology correction candidate

Core component `14283f65` replaces the inconsistent tick-only launch bound with the actual producer invariant: launches plus one unconsumed admission when docked equals dock sequence; later dock admissions cannot exceed elapsed simulation steps. Launch itself consumes the current admission without advancing time. Counter arithmetic uses widened sums and a nonzero sequence check before subtraction. Existing identity, visit, jump, fuel, pending-travel and account validations remain. This accepts genuine immediate launch saves while rejecting fabricated extra launches or admissions.

The original desktop test is byte-identical to `2b03a4f`, with no delay or weakened assertions. Three new core tests cover public tick-zero buy/launch/save/fresh Continue and a genuine round trip, resealed impossible chronology and identity fields, and overflow rollback/max-tick boundaries. Author formatting, docs and a finite transition-model check pass; the model is not Rust execution. This production successor requires independent delta review and fresh full Dell qualification before merge or native Metal acceptance. Earlier failed candidates remain separate evidence.

## Immutable provenance checkout correction

Owned Dell qualification of `2dbc7ae` passes 126 top-level Rust tests per platform, strict Clippy, the native Windows build and 37 importer tests against each checkout. Hosted Linux/macOS Rust checks and coverage also pass. Hosted Windows Rust checks pass, but its native Python importer suite fails one receipt-hash assertion: automatic LF-to-CRLF checkout conversion changes the archived provenance JSON from `a3a82919...` to `afdba455...`. Reproducing that conversion gives the exact failed hash.

The checkout correction marks immutable `assets/legacy/**` bytes as `-text` in `.gitattributes`. It changes no asset bytes, expected hashes, importer assertions, Rust code or save fingerprint. A fresh checkout with `core.autocrlf=true` must retain every original asset byte; current-head hosted checks and native Windows Python validation must pass before merge. Reuse unaffected exact Rust proof explicitly rather than calling the failed hosted job a pass.
