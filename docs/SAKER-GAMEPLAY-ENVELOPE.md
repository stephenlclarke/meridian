# Saker source envelope

SH042 Saker is a military catalogue hull with blueprint fabrication as its recorded acquisition method. The frozen L337 tree at `a11fe16924d87d06021816f9037952afcbac08db` supplies the metre-authored forest-ivory design, approved native geometry and physical records. The encodings retain source order, face normals, material colour, alpha, emissive flags, per-corner UVs and original atlas selectors for both meshes. All three texture atlases match the existing Meridian atlas files byte for byte.

| Source mesh | Vertices | Faces | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 1,996 | 3,864 | 32 |
| Native | 132 | 220 | 11 |

The original 220-face native mesh meets the source contract's 512-face limit. The envelope uses 26 local convex supports with 20 mm nominal rounding. Each engine casing, throat and luminous recess share their drive support. The barrel, braces, shrouds, glazing and service details retain local supports, keeping the spaces around the three nacelles open.

## Fit and contact

| Original surface | Verified full upper bound | Strict lower bound at an opposite-LOD source vertex |
| --- | ---: | ---: |
| Detailed | 448 mm | 88 mm |
| Native | 2,106 mm | 2,084 mm |

The left instrument housing has a 344 mm upper bound against the detailed mesh and a 2,106 mm upper bound against the native mesh. It has eight original detailed vertices and no native vertices assigned to its support. Its original vertex 1899 at `(-2.37, 3.7, -3.21)` m lies 2.0855163309712954 m from the nearest native relief, face 14 of the forward barrel. That exact nearest-face calculation supplies the native lower bound before rounding. The measured separation is already present in the original LOD geometry. The conservative housing bound includes the rounding and certificate margin and sits about 20.5 mm above that point measurement.

All seven swept routes through the gaps around the shrouds and drives have at least 479 mm of guarded clearance. Strict source-volume checks confirm the three barrel-to-brace and three brace-to-shroud overlaps, plus the barrel-to-canopy overlap, in both LODs. The aligned SH028 station control has 41,772 mm of guarded clearance. The offset control starts 979 mm clear and native vertex 2008 crosses strictly inside original SH028 face 482 halfway through the sweep.

## Engineering and Studio

The frozen stock engineering record and the separate finite-intrinsic empty-dry reference each give a dry mass of 26,588 kg. Their original centres and inertia tensors are preserved. The Studio definition uses the finite reference centre `(0, 0.12022601970437655, -6.249922187870627)` m and its exact symmetric inertia tensor. The source flight speed is 51.0. The definition includes seven 100 mm probes and explicitly labels the engineering reference. The frozen record limits this reference to the empty-dry configuration.

The signed installed Hull Studio exported the definition with 26 support groups and all seven probes passing. The export matches the definition byte for byte after its final newline is removed.

The Rust interface exposes explicit geometry-only sweeps. Per-hull Rust execution and Showroom qualification are pending. The catalogue implementation state remains pending.

[Qualification, source hashes and replay commands](../integration/sh042-saker/CRITERIA.json)
