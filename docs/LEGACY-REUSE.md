# Legacy reuse and parity

## Reference

The initial immutable legacy reference is Elite/L337 commit `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. It is a source reference, not a claim of full native qualification. Current Swift qualification and MBA fleet authoring continue separately; later source changes require a deliberate manifest update.

The local immutable reference inspected for planning is `/Volumes/SSD/offload/l337-g1-pr335-equipment-compose.UtqLRJ/required-cold-runtime.a11fe169/elite`. This machine-specific path is provenance context, not a build dependency. Never read/import the human WIP checkout as an implicit latest source.

## Source map

| Reference path | Reuse intent |
| --- | --- |
| Sources/L337Core/Game.swift, including GameRuntime | Identify intentional simulation, actor ownership and transition behaviour; reimplement behind smaller boundaries |
| Sources/L337Core/FlightSimulationState.swift and FlightControlInput.swift | Flight state, control semantics and numerical fixtures |
| Sources/L337Core/MarketInteraction.swift and OriginalLiveMarkets.swift | Stock, price, cargo and money transaction rules |
| Sources/L337Core/OriginalSealedDispatch.swift, ScriptCourier.swift and ScriptCourierState.swift | Distinguish sealed-dispatch/courier rules from original M01; do not substitute their missions for M01 |
| Sources/L337Core/CommanderSnapshot.swift and CommanderSaveInput.swift | Reference persisted state and semantic validation; not a promise of binary format compatibility |
| Sources/L337Core/Resources/Ships and ShipCatalogue.swift | Candidate source ship definitions and validation rules |
| Sources/L337Core/OriginalFleetCurrentHullSource.swift | Original hull constraints and display/collision source acceptance |
| Sources/L337Mac/L337App.swift | Player input/window/UI behaviours to preserve where intentional |
| Tests/L337CoreTests | Behavioural fixtures and negative cases to port after checking their source and scope |

[G3's detailed milestone map](LEGACY-MILESTONES.md) gives the mission/fleet/feature dependency breakdown. This table is a targeted starting map, not an inventory of all legacy systems.

## Asset import contract

Use an explicit per-file manifest with source repository/revision/path, SHA-256 and size, origin/author, exact notice/license reference, import transformation/version, output path/hash, units/axes/winding/material mapping and consumer milestone. Record reuse status separately from implementation and game acceptance. The first implementation imports one SH025 display mesh and its MIT notice, with the exact source and output hashes in assets/legacy/manifest.json. This expands all 3,484 source triangles, preserves positions/order and converts the source palette to linear RGB. UV textures and physical/collision admission remain unimplemented.

Import only the first journey's closure. Runtime consumes validated packaged data and never scans or decodes the entire legacy catalogue at startup. Conversion should be reproducible offline, with independent bounds, topology/UV/material and identity checks where relevant. Preserve original collision/contact/physical constraints; new renderer tessellation does not grant permission to relax source requirements. Future MBA fleet results enter through a new immutable source reference, not a live authoring directory.

The top-level legacy LICENSE is MIT with Stephen Clarke's copyright. Resource-specific notices also exist. For example, Sources/L337Core/Resources/Oolite-LICENSE.txt contains GPL and alternative noncommercial/share-alike artwork text. The exact origin and applicable terms of each selected asset are not yet resolved. Record those files and notices explicitly; do not infer a blanket commercial-use permission from the repository's top-level LICENSE. No commercial asset pack is approved by this planning document.

## Mechanics parity

Capture inputs, seed, initial state, commands, expected transitions and rejection invariants from source and meaningful tests. Where available, retain already recorded reference outcomes with their exact build provenance. Do not execute the current reference app or compile extra tools outside G1's execution lane to manufacture a baseline.

Port intentional rules, not Swift implementation structure, temporary workarounds or accidental bugs. Any deliberate gameplay change gets its own documented decision and test. Exact integer economic/item identity results should match. Numerical physics comparisons need stated tolerances and trajectory/contact invariants; cross-platform floating-point bit identity is not assumed. Images need an actual wgpu visual comparison, not only matching asset hashes.

Use Meridian-owned save fixtures and data directories. Importing user profiles, credentials or unrelated machine files is outside scope. The new product name does not establish trademark availability or final distribution clearance.
