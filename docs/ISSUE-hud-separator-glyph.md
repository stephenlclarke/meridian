# HUD separators render as question marks

The actual signed Mac app at main 3ca1403 renders the literal vertical bars in HUD lines using the fallback question-mark glyph. The native Metal capture meridian-g02-accepted.png shows the defect. The text is correct; the original code-drawn font lacks that character.

## Required behavior

Draw each vertical separator as a narrow vertical stroke, while preserving text layout, vertex bounds and all gameplay behavior. Use original code, with no font asset import.

## Validation

Run formatting, existing HUD geometry tests, strict Clippy and documentation checks. Recheck the actual Mac HUD after a native build. Preserve the current app's separate ongoing gameplay evidence; this fix does not change simulation, saves, input or mission authority.
