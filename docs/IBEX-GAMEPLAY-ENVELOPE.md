# Ibex source shield

SH027 is the original Ibex. Meridian lists it for blueprint fabrication. The legacy activation and approved native archive bind the mesh to authored metres with a scale of 1 m per source unit. Both original meshes preserve the source axes, origin and 34.59 m nose-to-tail span.

| Original mesh | Vertices | Triangles | Index-connected components |
| --- | ---: | ---: | ---: |
| Detailed | 1,612 | 3,108 | 29 |
| Native | 112 | 184 | 10 |

The native mesh matches the approved archive and fits the original 512-face limit. The candidate leaves ordinary admission and stock mass open.

## Trench and cross bridges

The candidate uses 25 convex supports with 20 mm spherical rounding. Each cargo keel, upper frame and half of the cross bridge retains its own support. Each drive groups its source casing, recess and emitter with the corresponding closed native assembly. The opaque cabin, glazing, canopy crest, pipe runs, mounts and individual clamps keep separate supports.

| Swept control | Travel | Guarded clearance |
| --- | ---: | ---: |
| Longitudinal cargo trench | 33 m | 1.997 m |
| Lateral cargo trench | 2.7 m | 1.010 m |
| Each pipe channel | 4 m | 225 mm |
| Aligned motion at original SH028 | 2 m | 42.658 m |
| Offset approach before lip contact | Static start | 0.979 m |

Exact interior rays verify that each cross-bridge half overlaps its keel and that the opaque cabin overlaps the glazing. In each source mesh the two bridge halves meet at the same 3.24 m² rectangular face with opposite normals. The original keel controls use detailed vertices 32 and 440. Original native keel vertex 14 crosses the strict interior of station lip face 482 halfway through the offset sweep.

## Fit and original visuals

The certified directed exposed-surface fit is 287 to 300 mm against detailed and 3.201 to 3.206 m against native. The detailed peak lies at a keel inset. The native peak lies at a detailed pipe mount that the lower-detail mesh omits. A mandatory original vertex of that mount already lies more than 3.184 m from native. Individual inset traversal remains unmeasured.

Both forest-and-ivory `IBEXVIZ1` streams preserve original face order, normals derived from winding, tint, emissive flags and UVs. The detailed and native streams contain 9,324 and 552 packed corners. All three original texture maps are used. The 1,216 unmapped detailed faces and 16 unmapped native faces retain the `0xffffffff` selector and their original UVs.

SteveMBA verified the source geometry, fit, contact and original-to-original difference on 19 November 2026. The pinned drivers replayed all three reports byte-for-byte.

```bash
python3 -B tools/prepare_ibex_geometry.py --check
python3 -B tools/verify_ibex_geometry.py
cargo test --locked -p meridian-core ibex::tests:: -- --nocapture
```

Rust and renderer qualification await owned execution.
