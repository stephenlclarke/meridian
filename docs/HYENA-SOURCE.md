# Hyena source envelope

The SH049 Hyena package preserves the original sand-graphite detailed and archived native artwork. The originals use relative coordinates. The frozen calibration is 20,000 / 1,737 metres per source unit, or approximately 11.514104778 metres per unit. The calibrated detail spans 27.549 m in width, 12.851 m in height and 30 m in length.

The frozen runtime catalogue disables player flight and normal purchase. The original R3 design contract specifies player flight after blueprint fabrication. This delivery covers catalogue inspection and geometry queries.

| Artwork | Vertices | Faces | Creator parts |
| --- | ---: | ---: | ---: |
| Archived detailed mesh | 2,064 | 3,472 | 164 |
| Archived native mesh | 528 | 952 | 26 |
| Creator-pinned native candidate | 304 | 504 | 26 |

The archived native mesh exceeds the original 512-face budget. The separate candidate fits with eight faces to spare. All four frozen recipe tests passed with the original SH020 comparison fixture present. Eighteen fixed parts retain exact original vertices, local faces, materials, and UVs. Eight curved parts use lower angular sampling. The maximum verified directed distances are 1,113 mm from candidate to archive and 1,442 mm from archive to candidate. The two forward pods set both bounds.

The 164 local convex supports match the creator part boundaries and use nominal 20 mm rounding. Seven source sweeps retain 834 to 2,771 mm of guarded clearance in both the archived and candidate arrangements.

| Reference surface | Upper bound from the candidate arrangement |
| --- | ---: |
| Original detailed mesh | 1,740 mm |
| Archived native mesh | 2,184 mm |
| Creator-pinned native candidate | 2,559 mm |

The fit replay preserves the complete independently captured reference triangulation as a derived proof input. Identical source coordinates can yield different Qhull facet diagonals across platforms, changing conservative per-piece bounds. The shared reference validator binds every support to its original scaled points and indices, proves the closed outward fan with the unchanged exact radial tolerance, and recomputes all adaptive bounds. The original artwork, report bytes and acceptance thresholds remain unchanged.

The signed installed Hull Studio exported the exact detailed artwork and 504-face candidate twice offline. Both exports and both raw responses matched byte for byte. The package retains the original relative coordinate scale and the frozen 58,298 kg finite empty-dry engineering data. The signed receipt covers the artwork export. The geometry results above come from the independent source replay.

[Source pins and replay commands](../integration/sh049-hyena/CRITERIA.json)
