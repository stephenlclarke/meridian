# Eagle Cruiser source envelope

SH043 Eagle Cruiser is a nonflyable military catalogue hull. This package supports catalogue inspection and explicit geometry queries. It preserves the original authored metres, navy-copper palette, face normals and winding, colours, alpha, emissive flags, and per-corner UVs. The three atlas images match the existing Meridian atlas files byte for byte.

| Original mesh | Vertices | Faces | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 2,000 | 3,876 | 31 |
| Native | 148 | 248 | 12 |

The approved native mesh fits the source contract's 512-face limit. The envelope uses 27 local convex supports with 20 mm nominal rounding. Each drive's casing, throat, and luminous recess share one support. The hangar walls, shoulders, cross supports, sensor ramparts, and bridge retain local supports so the original trenches remain open.

## Fit and contact

Seven swept routes have 984 to 3,349 mm of guarded clearance. The two original shoulder-to-wall joints, the two shoulder-to-cross-support joints, and the bridge-canopy joint overlap in both meshes.

| Original surface | Verified full upper bound | Strict lower bound at an opposite-LOD source vertex |
| --- | ---: | ---: |
| Detailed | 1,772 mm | 89 mm |
| Native | 5,377 mm | 5,088 mm |

The original native mesh omits the detailed dorsal louvres. Detailed vertex 1943 is already 5.089400567 m from original native faces 109 and 116. The left dorsal louvre support has the largest conservative native fit bound. Its detailed fit upper bound is 250 mm.

## Engineering and Hull Studio

The frozen stock engineering and finite-intrinsic empty-dry records are preserved byte for byte. Both give a dry mass of 295,038 kg. The Studio definition uses the named finite empty-dry centre and exact symmetric inertia. It preserves the detailed face appearance and includes seven 100 mm probes. The source package also retains the native appearance in a separate visual stream.

The installed Developer ID signed Hull Studio accepted the definition offline with 27 support groups and seven passing probes. Two exports produced the same bytes and preserved the definition except for its final newline.

The source defines a catalogue inspection package. Operating, station, crew, fuel, cargo, and gameplay admission remain outside this milestone.

[Source pins and replay commands](../integration/sh043-eagle-cruiser/CRITERIA.json)
