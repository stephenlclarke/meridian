# feat(missions): present original G02 dialogue and objectives

## Problem and behavior

G02 gameplay exists, but generic review notices omit the original story and important finite-cargo terms. A review with no next unit could be either acceptance or abandonment. Add source-grounded, typed presentation so players can understand exactly which action they are reviewing and which original units remain. See [the issue](ISSUE-g02-mission-presentation.md).

## Changes

Add `freight::g02::presentation` and a single module declaration beside the existing persistence module. The projection matches the private review purpose with the explicit selected choice, distinguishing acceptance, one-unit receiving, all-remaining receiving and abandonment. It returns original speaker/body text, warnings, exact one/all labels, Back and descriptive transfer counts. A mismatch returns no presentation and does not issue a command. Objectives read the actual World and retained clue, with active delivery taking precedence over an earlier completion and no fabricated stock after abandonment.

[The specification](G02-PRESENTATION.md) documents the API and integration. [Source provenance](G02-PRESENTATION-SOURCES.json) records the immutable original inputs and exact text transformations. The existing MIT notice remains intact. New short descriptions are distinguished from verbatim original dialogue. No borrowed third-party missions, Lua runtime, generic mission framework or extra graphics are introduced.

No World transition, G3665 save/accounting rule, receipt, identity, cargo, reward, source, equipment, flight or station code changes. Session, Profile, HUD and renderer files remain Root-owned and untouched by this branch. Root's separate desktop work will display wrapped terms and wire explicit receiving-all and abandonment controls while retaining normal confirm/Back and actual World validation.

## Authored verification

Four new tests exercise genuine public commands: offer/Back with no allocation or permission resurrection; real abandonment and finite exhaustion without success/restock; the actual flight/jump/physical dock and seven/eighth handovers with one/all choice distinction and cold memory continuation; and no objective for prototype worlds. Presentation is checked against unchanged encoded World bytes. The cold receiving test rejects an earlier review, uses a fresh actual crew review for the eighth, verifies one 730 CR payment/first clue and refuses repeat confirmation.

Exact source-fidelity checks reproduce the original Lua offer text through only the supported 730/L1 substitutions, compare the accepted journal and receiving/abandonment text, original choice labels, identity warning and destination, and verify the existing notice against the immutable source LICENSE. Rustfmt parsing, Markdown and diff checks are source-only. All new Rust compilation/tests/Clippy and joined desktop/native acceptance remain unrun in the author lane. Existing source tests and source-review evidence are not relabelled as actual-game completion.

## Required joined acceptance

Root composes this exact delta with its desktop controls/word wrapping and the current save/accounting source, then requests independent whole-candidate review and existing Dell Linux/Windows validation. Run workspace/all-target checks, all tests and strict Clippy on the actual joined revision. Actual UI must show original terms and warnings readably, distinguish one/all/abandon, retain refusal/status and explicit confirm/cancel, and route every action through the existing World commands. The first-playable packaged quit/Continue and full native game journey remain separate acceptance requirements. No separate leaf-run gate, competing Mac compiler/GPU work or automatic mission-authority grant is added.

## Limits

The core projector is not an authority check, renderer or persistence subsystem. A stale review can still be described but cannot be confirmed without normal World validation. The exhaustion objective does not implement loss recovery. The existing ship/station collision-source and native visual qualification limits remain. This branch is authored source pending review and joined execution, not a claim that the whole first playable is complete.
