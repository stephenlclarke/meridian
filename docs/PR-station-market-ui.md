# feat(desktop): connect paid station Food trade

Join MC1's frozen World/account service implementation `5531b8407fd690ca5a5f4155014574a6838c4de1` to the integrated World/HUD desktop. G buys one Food tonne and H sells one; F uses the same shared fuel service rule. The HUD reads actual Food/hold totals and current supported station prices and stock. A copied quote cannot choose a site or set a price: World derives service facts from its current genuine dock and commits the account and stock together after validation.

The core retains owned standard equipment and all eight identified G02 units in the shared 92-tonne hold. Full-tank refuelling returns a no-charge result under the shared original rule. This removes the earlier duplicate formula without adding another wallet or cargo owner. The market data remains exact original L337-authored data with retained provenance; no art/font assets or third-party Elite/Oolite content are added.

The desktop now keeps one active freight selection until that job completes. It refuses another source review while the existing job is active, then clears the selection after genuine final receiving. This avoids orphaning an earlier held job by replacing the UI's only opaque job handle; it does not change the core's finite 16-unit source or add replacement cargo.

Conflict reconciliation: the three shared core files selected verbatim from MC1 are flight/mission.rs, freight/g02.rs and lib.rs. All core Rust implementation files match the frozen MC1 source after formatting; the Cargo proprietary license remains current. Root changes only desktop caller, HUD and validation documentation beyond that core.

Validation before review: 25 Python importer tests, source formatting, whitespace and documentation lint pass. MC1's standalone eight service tests passed earlier on the Dell; this is not proof for the new World join. Five new MC1 core tests and the desktop shared-wallet/hold/stock regression are authored but Rust/compiler/Clippy/GPU execution is UNRUN for this candidate. Existing 73-test World/HUD qualification remains tied to `218267a`; no test, save, trading or playable result is inherited by this changed candidate.

Next: one complete G2 source review of the actual core and caller, final hosted checks, then the existing Dell owner qualifies the same exact changed native executable. Preserve the previous qualified preview and the separate earlier candidate. Save/quit/continue and actual GPU/held-input remain open while Windows is locked.
