# Harpy Gunship source envelope

This SH047 inspection package preserves the original L337 Harpy Gunship hull in metre coordinates, with the original sand-maroon palette, triangle winding, normals, per-corner UVs, and material appearance. The three shared texture atlases match Meridian's existing copies byte for byte.

The frozen runtime catalogue disables player flight and normal purchase. The original R3 design contract specifies player flight after blueprint fabrication. This delivery covers catalogue inspection and geometric queries.

| Original mesh | Vertices | Faces | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 2,584 | 5,040 | 32 |
| Native | 148 | 244 | 13 |

The original native mesh fits the 512-face limit. Twenty-eight local convex supports use nominal 20 mm rounding. The two lower housings, forward rails, belly module, canopy, and three keels retain separate supports. Each drive's three detailed source components share one support.

Seven hull inspection sweeps have 621 to 1,233 mm of guarded clearance. Eight original joints overlap in both meshes, including the belly-to-central-keel and both rail-to-lower-housing joints.

| Original surface | Verified full upper bound | Strict lower bound at an opposite-LOD source vertex |
| --- | ---: | ---: |
| Detailed | 847 mm | 88 mm |
| Native | 1,448 mm | 1,191 mm |

Original detailed vertex 96 lies 1.192290507 m from native faces 41 and 54. Both frozen engineering records give a dry mass of 47,068 kg. The Studio package preserves the original finite empty-dry centre and symmetric inertia as source data.

The installed Developer ID signed Hull Studio exported the exact visual artwork twice offline. Both exported packages and both raw responses matched byte for byte. That receipt covers the artwork export. The geometric bounds and joint results above come from the separate source replay.

[Source pins and replay commands](../integration/sh047-harpy-gunship/CRITERIA.json)
