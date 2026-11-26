# Caracal Scout source envelope

The SH053 Caracal Scout package preserves the original navy-copper detailed and native meshes in relative coordinates. Its frozen calibration is 1,500,000 / 171,707 metres per source unit, or approximately 8.735811586 metres per unit. The original detail spans 23.881 m wide, 8.905 m high, and 30 m long.

The frozen runtime catalogue disables player flight and normal purchase. The original R3 design contract specifies player flight after blueprint fabrication. This delivery covers catalogue inspection and geometry queries.

| Artwork | Vertices | Faces | Parts |
| --- | ---: | ---: | ---: |
| Archived detailed mesh | 3,224 | 5,372 | 269 |
| Archived native mesh | 768 | 1,380 | 39 |
| Project-authored native candidate | 296 | 492 | 25 |

The archived native mesh exceeds the 512-face budget. The separate project-authored candidate fits with 20 faces to spare. It retains all ten lofts and every original longitudinal station and coordinate extremum. Nine fixed box parts and six shallow wells retain exact original vertices, local faces, materials, and UVs. Fourteen blind wells remain in both original meshes and are omitted from the candidate artwork. The central body sets the verified candidate-to-archive upper bound at 1,233 mm. An omitted well sets the archive-to-candidate upper bound at 1,761 mm.

The 89 local convex supports use nominal 20 mm rounding. Rims share the support of their own blind well. The seven shape sweeps preserve 317 to 1,376 mm of guarded clearance in both arrangements. The stacked-pod openings retain 319 mm on port and 317 mm on starboard.

| Reference surface | Upper bound from the candidate arrangement |
| --- | ---: |
| Original detailed mesh | 1,768 mm |
| Archived native mesh | 1,708 mm |
| Project-authored native candidate | 2,299 mm |

The fit replay preserves the complete original reference triangulation as a derived proof input. The same support points produced different Qhull facet diagonals on x86 in 25 archived and 28 candidate supports, changing individual conservative bounds while preserving the reported maxima. The shared validator binds both full point streams and every support identity, validates the closed outward reference fan with the unchanged exact radial tolerance, and recomputes every adaptive bound. A macOS arm64 capture reproduced all three original numerical reports byte for byte. The four imported Banteng measurement functions retain their exact original sources. Artwork, numerical receipts and acceptance thresholds remain unchanged.

The installed signed Hull Studio exported the exact original detail and 492-face candidate twice offline. Both exports and both raw responses matched byte for byte. The package retains the original relative scale and the frozen 40,097 kg finite empty-dry engineering record. The signed receipt covers artwork export. The geometry results come from the independent source replay.

[Source pins and replay commands](../integration/sh053-caracal-scout/CRITERIA.json)
