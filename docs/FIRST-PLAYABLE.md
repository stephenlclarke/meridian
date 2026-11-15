# First playable milestone

## Outcome

Deliver a standalone Meridian Mac app in which a new commander can launch, fly, navigate, dock, trade, complete the original G02 consented-freight journey used for M01 acceptance: Avenor to Renas with eight identified one-tonne handovers, receive its once-only 730 CR and trusted clue, save, quit the process and continue correctly. The journey is the acceptance target; a moving ship demo or headless simulation alone does not complete it.

This is a new implementation milestone, not a claim that the current Swift app or new repository already passes. Original physical actors, objectives, source identities, receiving actions, required equipment, cargo and payment semantics remain. The [source-grounded breakdown](LEGACY-MILESTONES.md) selects SH025, Avenor G01-S007 and Renas G01-S185. It identifies a finite supplier of 16 units, eight unique one-tonne units allocated to the job, seven partial handovers without reward, then the eighth handover with one 730 CR payment and trusted clue `consented-freight-family-trace`. Full callback and dialogue extraction remains part of implementation; unexamined details stay unknown. Do not substitute a simpler courier mission to meet the milestone label.

## Implementation sequence

| Slice | Concrete result | Acceptance before expanding |
| --- | --- | --- |
| P0: reference closure | Small manifest of journey assets, mechanics, actors, route, equipment and tests from one immutable legacy revision | Every required item has a source and reuse disposition; no invented asset, approval or acceptance |
| P1: executable shell | Rust app, winit lifecycle, wgpu scene, actual reused ship/station assets, HUD, input and normal quit | Actual Mac rendering, resize/minimize/fullscreen/input checks, no GPU validation errors; core builds without graphics |
| P2: local flight and docking | Authoritative movement, fuel, collisions, station approach/admission, launch and paid services | Positive and negative flight/docking fixtures; actual player-controlled launch-flight-dock loop |
| P3: connected journey | Required systems/travel, real market transaction, held cargo/equipment and all M01 delivery/receiving transitions | Eight deliveries preserve identity and physical requirements; invalid, duplicate and unpaid actions refuse atomically |
| P4: durable continuation | Meridian save/load, title/continue flow, atomic save failure handling | Quit and restart the executable at pre-completion and post-payment checkpoints; state remains correct and payment cannot repeat |
| P5: packaged playable | Versioned Mac bundle and reproducible release evidence | Complete journey in the packaged app, save/quit/continue, timings/memory recorded and clean normal exit |

P0-P5 are slices within one first-playable milestone, not separate substitute products. No ETA is inferred from the number of slices. Ship/station selection is the smallest dependency closure that faithfully supports the journey, not all 171 fleet entries. Background traffic/hazards required by the mission remain part of that closure; broader combat and unrelated missions follow later milestones.

## Required evidence

- Flight and travel: player controls work; force, acceleration, rotation, fuel and arrival follow explicit reference rules; collision geometry is not inferred from a display-only mesh.
- Economy: one ordinary buy/sell transaction changes credits, stock and cargo consistently. Required equipment and fuel are paid for; unavailable stock, insufficient funds and capacity failures leave state unchanged.
- Mission: each original objective is completed through its actual actor/location/action, with the correct held item and equipment. Save/reload, duplicate input, wrong receiver and missing item cannot mint progress, consent, replacements or payment.
- Completion: eight identified one-tonne handovers within one freight job, exactly 730 CR under the original reward rule, trusted-clue result and no repeat reward after a second receiving action or cold continuation. Record balance deltas separately from market, fuel and equipment expenses.
- Save: restart at multiple checkpoints; compare mission/item IDs, cargo, credits, equipment, location, fuel, time and RNG state. Truncated, unsupported and semantically invalid files preserve the prior valid save and current World.
- Presentation: actual wgpu frames show navigation, docking cues and current mission/HUD values. Verify help, text readability, pause, resize, fullscreen and normal quit. Render-only screenshots do not prove mission acceptance.
- Delivery: package fingerprint, source revision, imported-content manifest, machine/OS/backend, exact test results and recorded limitations accompany the app. Source authored, unit tested, actual-game tested and released are distinct stages.

## Initial performance targets

Proposed first-playable targets on a named reference Apple Silicon Mac: 60 fps at 1920x1080, p95 frame interval at most 16.7 ms during a documented steady flight sample, first usable window within 2 seconds and ready-to-fly within 5 seconds on a warm launch. Record cold launch separately. Proposed total process resident-memory target is at most 1 GiB for that scene, with CPU/GPU allocation categories and OS accounting recorded. These are design targets, not enforced machine resource limits or measured results.

Run a documented repeated launch/travel/dock/save/load scenario and record baseline, peak and post-cycle memory, object counts, asset-cache sizes and frame timings. Explain any retained allocation; do not infer a leak from one RSS sample or hide growth behind queue drops/cache clearing. Formal tolerances and the reference hardware are selected before qualification once the first scene exists.

## Scope after the first playable

Expand only after the whole loop works: combat/equipment breadth; physical work and non-C01 mission families; full fleet/station parity; robust content tooling and performance; Windows qualification; commercial distribution. The delegated legacy roadmap will give source-specific dependencies and acceptance for these expansions. All 64 original mission and 171 fleet acceptances remain separately tracked; a first playable does not close them.
