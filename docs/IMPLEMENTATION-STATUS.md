# Implementation and validation status

Reviewed against actual delivery evidence on 16 November 2026. This document distinguishes installed gameplay from source awaiting execution. Update it with every delivery or material acceptance change; test counts belong to their exact source.

## Delivered checkpoint

Gameplay main `d5be89b9b1a515e9185d450687a558b20d9e1e7d`, tree `e13b6447396ff2e2578f0e51ba7c712282935aaa`, is the protected merge of [PR 17](https://github.com/stephenlclarke/meridian/pull/17). The signed local `Meridian Finite Missiles Preview.app` and adjacent launcher/Read Me use a separate save directory. The installed signed executable SHA-256 is `002c208a4dc9d3a3fb07265a2b12fae8dbe7a9beb7e0063d3e78d1726d594e24`. All 24 delivered files/modes/signature and 171 unchanged prior artifact files/modes were independently verified. Normal installed title/render/close exited zero without creating a commander save.

Mac, Linux and native Windows validation passed 136 core plus 43 desktop tests: 179 top-level Rust tests. Each applicable Python importer suite passed 48 tests. Nested subprocess summaries are not additional cases. Native Windows Python execution was verified separately from WSL. [Main CI 43834783303](https://github.com/stephenlclarke/meridian/actions/runs/43834783303) and exact-main SonarQube passed. Headless Windows validation does not establish Windows GPU/input acceptance.

## Actual gameplay evidence

The delivered sequence has qualified actual Metal flight, physical docking, market transactions/native kg/g goods, funded capacity refusals, eight identified original G02 freight deliveries, once-only 730 CR/trusted clue, seven-delivery and completed cold Save/Quit/Continue, unsupported-format refusal and explicit new-game safeguards. Later scoped previews added paid equipment, fitted Pulse fire, harmless practice beacons/reception and ordinary station impacts with permanent attempt failure.

Finite missile qualification observed F1-F4 controls, actual three-to-zero owned-round consumption, original missile mesh rendering, pending-projectile pause/quiet behavior, a first missile practice hit, real station obstruction with legal 64 and permanent failure, and dock/relaunch without ammunition replenishment. At 640 by 360, critical missile HUD rows were visibly checked. Opaque item/projectile IDs are tested in core/desktop, not exposed as native UI evidence. Live player-hit and full T02 acceptance remain open.

## Save compatibility and active work

The delivered finite launcher writes version 7 and reads exact supported versions 4-7 for unused missile stock. Armed/locked state must be cleared before saving. Any missile use, including ended history, refuses save/restore; existing saved files remain byte-identical. Q explicitly quits without saving after a save failure. Earlier previews keep their dedicated versions/profiles; never silently migrate or replace them.

Durable missile version 8 at reviewed source `1fea0fa2892e4fda58bbf5c3b94184026690e997` is authored and independently source-cleared. It retains actual ammunition ownership, live/ended projectile origin/history, arm/lock/cooldown and quiet state. Compiler, tests, strict Clippy and actual native cold continuation are pending. Source clearance is not playable delivery.

## Open acceptance and distribution limits

- Memory qualification is unresolved. The original in-flight scan reported 416 objects/19,936 bytes in three XPC root cycles. Later cold and instrumented zero scans did not reproduce the original fresh setup/age trigger. They establish neither a cure nor OS-only responsibility; no evidenced source fix exists.
- Sustained minimize, held-key focus-loss, measured FPS/p95 and race evidence remain open. The minimum-size missile HUD check does not qualify all window behavior.
- Apple notarization is blocked by the required-agreement HTTP403. A dependency notice also limits commercial distribution. No legal acceptance or unchanged notarization retry is authorized.
- Full first-playable/release acceptance, all 171 expanded fleet and all 64 expanded mission acceptances remain separate and open. Legacy baseline is 23 of 162 ships plus four of six stations. Source catalogue entries are not accepted playable hulls.

Earlier exact-source tests and failures remain in Git history, merged PRs and retained delivery evidence. They must not be transferred to a newer source or counted as new acceptance. Historical feature contracts describe their original boundaries; this page records the current delivered checkpoint.
