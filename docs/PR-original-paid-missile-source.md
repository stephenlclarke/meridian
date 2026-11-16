# feat: add paid finite missile fitting and retained launches

## Result

A reviewed paid main-berth action prepares a new owned missile and the 180 CR debit in one complete save candidate. Store/refit retains identity and provenance. Nine SH025 positions are usable without granting six free rounds. Later launches retain every old origin and contact record.

## Implementation and compatibility

See the [complete core boundary, source references and acceptance](PAID-MISSILES.md). Current saves use explicit v12. Existing readers remain explicit, and unsupported/corrupt bytes refuse without replacement. Existing published previews, installed profiles, assets, Cargo dependencies, workflow definitions and platform code are unchanged.

## Validation

Fifteen new Rust cases are authored and **UNRUN**. Direct rustfmt parsing/formatting, Markdown lint and static immutable-source/assertion checks are the only author-lane checks. No compiler, Cargo, Clippy or native execution has run for this change. No parent pass is rebound to this source.

## Remaining work and risks

Root must join the actual docked screen, all-round HUD, all-projectile rendering and atomic profile writer. G2 whole-source review and exact-commit execution remain required. This first increment does not complete all modular equipment or all-hull compatibility. General equipment resources/effects must be real before additional modules are saleable. Native purchase/fire/dock/quit/Continue and writer failure remain unverified.

## Linked issue

[Problem and acceptance](ISSUE-original-paid-missile-source.md).
