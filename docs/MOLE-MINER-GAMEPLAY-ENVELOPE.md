# Mole Miner source shield

SH016 is the original Mole Miner. Meridian lists it for blueprint fabrication. The legacy revision `a11fe169` supplies its relative-unit source meshes and an engineering scale of 11500/687 metres per source unit. The candidate applies that scale once to both meshes and preserves their axes, origin and 46 m length.

| Original mesh | Vertices | Triangles | Index-connected components |
| --- | ---: | ---: | ---: |
| Detailed | 6,760 | 12,836 | 179 |
| Native | 616 | 1,100 | 33 |

The independently archived native export matches the resource's native geometry and appearance. That mesh exceeds the 512-face design limit by 588 triangles. The controlled collision and presentation data preserve both originals. Native face-budget compliance and ordinary admission remain open. The retained legacy 119,801 kg mass model is an engineering budget. The candidate assigns no runtime mass.

## Local supports

The candidate uses 203 convex supports rounded by 20 mm. Each service well groups its rim and recessed struts. The pressure hull, two pods, individual docking frame members, boring heads and engines keep their own supports. Sixteen 22.5-degree sectors cover each of the eight boring collars. Neighbouring sectors share only their boundary vertex rings. Every source triangle belongs in full to at least one support, including each collar seam across the final angular index.

| Control | Guarded clearance |
| --- | ---: |
| 8 m sweep through the original tooling aperture | 1.909 m |
| 60 mm collar-to-head sweep on each side | 2 mm |
| 300 mm sweep between the first two collars on each side | 173 mm |
| Aligned crossing against the original SH028 station | 39.731 m |
| Offset approach before lip contact | 0.979 m |

The solid controls use an original vertex on each boring head. In the offset crossing, native vertex 396 crosses the strict interior of original SH028 face 482 halfway through the sweep. Exact interior rays through both source meshes verify two glazing overlaps, one with the opaque pressure body and one with the opaque cockpit crossmember.

## Fit and appearance

The directed exposed-surface fit is 1.102 to 1.460 m against detailed and 2.102 to 2.175 m against native. The largest certified sample for each comparison is at a service well. Required original vertices alone lie more than 0.746 m from detailed and more than 2.084 m from native.

The original slate-and-orange detailed and native visuals supply 38,508 and 3,300 packed corners. Both `MOLEVIS1` streams preserve triangle order, normals derived from winding, tint, emissive flags and repeating UVs. Their faces use original shared maps 0 and 2.

SteveMBA verified the source topology, generated both metric encodings and measured fit and contact controls on 19 November 2026. The replay manifest pins the drivers, shared numerical helper, exact inputs and reports. The older general exact importer stopped on the SH016 storage budget before returning a candidate. The rounded supports use a separate controlled geometry path.

```bash
python3 -B tools/prepare_mole_miner_geometry.py --check
python3 -B tools/verify_mole_miner_geometry.py
cargo test --locked -p meridian-core mole_miner::tests:: -- --nocapture
```

The three Rust collision tests and native visual qualification await owned execution.
