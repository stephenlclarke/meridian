# Erne Corvette source envelope

This SH045 Erne Corvette inspection package preserves the original L337 metre coordinates, ice-cobalt palette, winding, normals, per-corner UVs, and material appearance. The shared texture atlases match Meridian's existing atlases byte for byte.

The frozen runtime catalogue disables player flight and normal purchase. The original R3 design contract specifies player flight after blueprint fabrication. This delivery covers catalogue inspection and geometry queries.

| Original mesh | Vertices | Faces | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 1,224 | 2,340 | 27 |
| Native | 76 | 120 | 8 |

The approved native mesh fits the original 512-face limit. Twenty-three local convex supports use nominal 20 mm rounding. Each drive's three detailed source components share one support. The lower rails, shoulders, canopy, and smaller surface details retain separate supports.

Seven swept routes have 1,743 to 4,144 mm of guarded clearance. Five original joints overlap in both meshes. These include both shoulders' joints with the central hull and lower rails, plus the canopy-to-hull joint.

| Original surface | Verified full upper bound | Strict lower bound at an opposite-LOD source vertex |
| --- | ---: | ---: |
| Detailed | 891 mm | 88 mm |
| Native | 2,333 mm | 2,103 mm |

Original detailed vertex 1127 lies 2.104644621 m from native face 14. Both original engineering records give a dry mass of 84,357 kg. The Studio definition uses the original finite empty-dry centre and symmetric inertia. It retains both meshes and seven 100 mm inspection probes.

The installed Developer ID signed Hull Studio accepted the definition offline. Both exports and both diagnostic responses matched byte for byte, with 23 support groups and seven passing probes. The exported package preserved the definition except for its final newline.

[Source pins and replay commands](../integration/sh045-erne-corvette/CRITERIA.json)
