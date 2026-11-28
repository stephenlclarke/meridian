# Harpy Dropship source envelope

This SH046 Harpy Dropship inspection package preserves the original L337 metre coordinates, sand-maroon palette, triangle winding, normals, per-corner UVs, and material appearance. Its three shared texture atlases match the existing Meridian atlases byte for byte.

The frozen runtime catalogue disables player flight and normal purchase. The original R3 design contract specifies player flight after blueprint fabrication. This delivery covers catalogue inspection and geometry queries.

| Original mesh | Vertices | Faces | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 1,628 | 3,132 | 31 |
| Native | 144 | 240 | 12 |

The approved native mesh fits the 512-face limit. Twenty-seven local convex supports use nominal 20 mm rounding. The belly module, forward ramp, skids, canopy, shoulders, and keels retain separate supports. Each drive's three detailed source components share one support.

Seven sweeps have 279 to 1,233 mm of guarded clearance. Seven original joints overlap in both meshes, including the forward ramp-to-belly and belly-to-central-keel joints.

| Original surface | Verified full upper bound | Strict lower bound at an opposite-LOD source vertex |
| --- | ---: | ---: |
| Detailed | 847 mm | 88 mm |
| Native | 1,379 mm | 1,191 mm |

Original detailed vertex 96 lies 1.192290507 m from native faces 41 and 54. Both frozen engineering records give a dry mass of 51,822 kg. The Studio package preserves the original finite empty-dry centre and symmetric inertia as source data.

The installed Developer ID signed Hull Studio accepted the exact definition offline. Both exports and both diagnostic responses matched byte for byte. All seven catalogue probes passed with 27 groups. The exported package preserved the definition except for its final newline.

[Source pins and replay commands](../integration/sh046-harpy-dropship/CRITERIA.json)
