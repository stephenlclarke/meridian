# Banteng source envelope

The SH052 Banteng package preserves the original navy-copper detailed and native artwork in relative coordinates. The frozen engineering calibration is 20,000 / 1,661 metres per source unit, or approximately 12.040939193 metres per unit. It gives detailed bounds of 32.246 m wide, 10.301 m high, and 40 m long.

The frozen runtime catalogue disables player flight and normal purchase. The original R3 design contract specifies player flight after blueprint fabrication. This delivery covers catalogue inspection and geometry queries.

| Artwork | Vertices | Faces | Parts |
| --- | ---: | ---: | ---: |
| Archived detailed mesh | 3,192 | 5,324 | 265 |
| Archived native mesh | 736 | 1,332 | 35 |
| Project-authored native candidate | 288 | 484 | 23 |

The archived native mesh exceeds the 512-face budget. The separately identified project-authored candidate fits with 28 faces to spare. It retains all ten lofts and all original longitudinal stations, including the same coordinate extrema at each station. Five fixed box parts and eight blind service wells retain exact original vertices, local faces, materials, and UVs. Twelve shallow wells are omitted from the candidate artwork and remain present in both originals. The maximum verified directed distances are 1,481 mm from candidate to archive and 1,587 mm from archive to candidate. The central body and an omitted blind well set those respective bounds.

The 85 local convex supports use nominal 20 mm rounding. Recess rims share the support of their own blind well. This grouping stays local to each well. Seven shape sweeps have 472 to 1,905 mm of guarded clearance in both arrangements. The original stacked-pod openings retain 475 mm on port and 472 mm on starboard.

| Reference surface | Upper bound from the candidate arrangement |
| --- | ---: |
| Original detailed mesh | 1,723 mm |
| Archived native mesh | 1,966 mm |
| Project-authored native candidate | 2,545 mm |

The fit replay preserves the complete independently captured reference triangulation as a derived proof input. Identical source coordinates can produce different Qhull facet diagonals across platforms, changing conservative per-piece bounds. The shared reference validator binds every support to its original scaled points and indices, proves the closed outward fan with the unchanged exact radial tolerance, and recomputes all adaptive bounds. The original artwork, report bytes and acceptance thresholds remain unchanged.

The installed signed Hull Studio exported the exact artwork twice offline with the original detailed mesh and 484-face candidate. Both exports and both raw responses matched byte for byte. The package retains the relative calibration and the frozen 70,185 kg finite empty-dry record. The signed receipt covers artwork export. The geometry results come from the independent source replay.

[Source pins and replay commands](../integration/sh052-banteng/CRITERIA.json)
