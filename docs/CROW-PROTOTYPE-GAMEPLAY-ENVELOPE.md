# Crow Prototype source shield

SH020 is the original Crow Prototype. Meridian lists it for blueprint fabrication. Both original meshes use the legacy engineering scale of 20000/2533 metres per source unit. The candidate applies it once to both meshes and preserves their source origin, axes and 30 m nose-to-tail span.

| Original mesh | Vertices | Triangles | Index-connected components |
| --- | ---: | ---: | ---: |
| Detailed | 2,064 | 3,472 | 164 |
| Native | 528 | 952 | 26 |

The archived native export matches the resource's native mesh and appearance. It exceeds the original 512-face limit by 440 triangles. The legacy lower-face proposal records an unqualified generator. The controlled candidate preserves the two active originals. Native face-budget compliance, ordinary admission and runtime mass remain open.

## Fork geometry

The candidate uses 68 local convex supports with 20 mm rounded padding. Each forward arm and its longitudinal shoulder keep separate supports. The aft cross bridge and central body keep their own supports and leave the forward fork open. The 36 detailed well assemblies keep separate supports with their local recessed struts. The 12 native wells retain their distinct source positions. The glazing, opaque cockpit crossmember and engine components keep their own supports.

| Swept control | Travel | Guarded clearance |
| --- | ---: | ---: |
| Axial fork opening | 9.6 m | 1.793 m |
| Lateral fork opening | 2.34 m | 1.206 m |
| Along either inner arm wall | 9.1 m | 1.018 m each |
| Aligned motion at the original SH028 port | 2 m | 45.437 m |
| Offset approach before lip contact | Static start | 0.978 m |

The two solid arm controls use original detailed vertices 112 and 680. Native vertex 54 crosses strict interior of original station face 482 halfway through the offset sweep. Exact interior rays check the original glazing's overlap with the opaque pressure body and the cockpit crossmember in both meshes.

## Fit and original visuals

The certified directed exposed-surface fit is 626 to 645 mm against detailed and 914 to 920 mm against native. The peaks occur at different source well positions. Mandatory source vertices already require distances greater than 607 mm from detailed and 898 mm from native.

The navy-and-copper detailed and native `CROWVIZ1` streams contain 10,416 and 2,856 packed corners. Both preserve the original face order, normals derived from winding, tint, emissive flags and UVs. The original faces use repeating texture maps 0 and 2.

SteveMBA verified topology, metric encodings and the fit, source-difference and contact reports on 19 November 2026. The pinned replay drivers reproduced all three reports byte-for-byte.

```bash
python3 -B tools/prepare_crow_prototype_geometry.py --check
python3 -B tools/verify_crow_prototype_geometry.py
cargo test --locked -p meridian-core crow_prototype::tests:: -- --nocapture
```

Rust and renderer qualification await owned execution.
