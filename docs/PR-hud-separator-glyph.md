# Draw the HUD vertical separator

Add the missing vertical-bar glyph to the existing original code-drawn character table. The actual Mac scene previously displayed question marks between HUD values. A seven-pixel vertical stroke fixes that character without changing text, geometry allocation, layout or application behavior.

## Validation

This is a one-entry rendering correction. Existing HUD geometry and bounds tests provide regression coverage; the decisive check is the actual rendered Mac HUD. No implementation-mirroring glyph test is added. Formatting, compiler/tests, strict Clippy, independent delta review and native visual verification are recorded separately as they complete.

## Scope

Simulation, World commands, mission state, source assets, save format, custody and payment code are unchanged. The ongoing main 3ca1403 journey remains evidence for that exact app; this rendering delta does not silently relabel it as a new full-game acceptance.
