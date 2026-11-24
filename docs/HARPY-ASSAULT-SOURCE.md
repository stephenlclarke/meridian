# Harpy Assault source envelope

This SH044 Harpy Assault inspection package preserves the original L337 metre coordinates, sand-maroon palette, winding, normals, per-corner UVs, and material appearance. The three texture atlas files match the shared Meridian atlas files byte for byte.

The frozen runtime catalogue disables player flight and normal purchase. The separate original R3 design contract specifies player flight after blueprint fabrication. This delivery covers catalogue inspection and geometry queries.

| Original mesh | Vertices | Faces | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 1,822 | 3,516 | 32 |
| Native | 146 | 248 | 11 |

The approved native mesh fits the original 512-face limit. Twenty-eight local convex supports use nominal 20 mm rounding. Each drive's three detailed source components share a support. The shoulders, outer keels, central keel, ventral keel, canopy, and smaller surface details retain separate supports.

Seven swept routes have 229 to 1,250 mm of guarded clearance. Both shoulder-to-central-keel joints, both shoulder-to-outer-keel joints, and the canopy-to-central-keel joint overlap in both original meshes.

| Original surface | Verified full upper bound | Strict lower bound at an opposite-LOD source vertex |
| --- | ---: | ---: |
| Detailed | 847 mm | 88 mm |
| Native | 1,642 mm | 1,499 mm |

Original detailed vertex 1497 lies 1.5 m from native face 194. The stock and finite empty-dry engineering records both give a dry mass of 47,408 kg. The Studio definition uses the original finite empty-dry centre and symmetric inertia. It retains both meshes and seven 100 mm inspection probes.

The installed Developer ID signed Hull Studio accepted the definition offline. All seven probes passed across 28 support groups. Both exports and both diagnostic responses matched byte for byte. The exported package preserved the definition except for its final newline.

[Source pins and replay commands](../integration/sh044-harpy-assault/CRITERIA.json)
