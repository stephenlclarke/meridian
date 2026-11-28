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

## Stock Showroom inspection

The original catalogue lists SH027 for blueprint fabrication. The stock descriptor binds that row to the original Ibex mesh when the player searches for **SH027** or **Ibex**. The station-screen regression checks the selected row before and after supplying the Ibex geometry. The desktop regression compares all 9,324 detailed corners and ties its collision pieces to the source. The Studio definition retains the original detailed and native material indices. Active engineering remains unassigned.

Four 100 mm radius probes pass through the length and width of the trench and the two pipe channels. A single convex support closes all four paths. Probes touching detailed vertices 32 and 440 hit the two original keels. The portable certificate reproduces the fit against both meshes. The native lower witness falls on a pipe mount omitted from the original low-detail mesh.

[Stock definition](../integration/sh027-ibex/IBEX-STUDIO.json) · [Portable fit](../integration/sh027-ibex/CERTIFIED-FIT.json) · [Original-mesh differences](../integration/sh027-ibex/CERTIFIED-DIFFERENCE.json)
