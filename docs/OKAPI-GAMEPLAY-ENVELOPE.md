# Okapi source shield

SH029 is the original Okapi. Meridian lists it for blueprint fabrication. The legacy activation, physical contract and approved native archive bind both source meshes to authored metres. Meridian uses their original axes, origin and 34.09 m span at 1 m per source unit.

| Original mesh | Vertices | Triangles | Index-connected components |
| --- | ---: | ---: | ---: |
| Detailed | 1,248 | 2,376 | 30 |
| Native | 76 | 120 | 8 |

The native mesh matches the approved archive and fits the original 512-face limit. The source slice covers the bare hull. Optional equipment, runtime mass and ordinary admission remain open.

## Pallet lanes and rails

The candidate covers all 2,496 original triangles with 26 local convex supports and 20 mm spherical rounding. The deck, each of the three pallets and top rails, cabin, glazing and pipe parts retain separate supports. Each drive support groups the casing, recess and emitter with its closed native assembly.

| Swept control | Travel | Guarded clearance |
| --- | ---: | ---: |
| Each inter-pallet lane | 16.6 m | 628 mm |
| Each pipe lane | 3.9 m | 208 mm |
| Each of three top-rail gaps | 10 m | 29 mm |
| Aligned motion at original SH028 | 2 m | 45.018 m |
| Offset approach before lip contact | Static start | 0.979 m |

Each detailed top rail sits 100 mm above its pallet. The solid controls use original detailed rail vertices 210, 226 and 242. Exact interior rays verify that the deck overlaps all three pallets and the opaque cabin, and that the cabin overlaps the glazing in both source meshes. Original native cabin vertex 22 crosses the strict interior of station lip face 482 halfway through the offset sweep.

## Fit and original visuals

The certified directed exposed-surface fit is 373 to 386 mm against detailed and 1.600 to 1.611 m against native. The detailed peak lies at a cabin inset. The native peak lies at a pipe mount that the lower-detail mesh omits. One mandatory original vertex of that mount already lies more than 1.489 m from native. Individual inset traversal remains unmeasured.

Both slate-and-orange `OKAPVIZ1` streams preserve original face order, normals derived from winding, tint, emissive flags and UVs. The detailed and native streams contain 7,128 and 360 packed corners. The original 1,216 detailed and 16 native unmapped faces retain their UVs and the `0xffffffff` selector.

SteveMBA verified the source geometry, fit, contact and original-to-original difference on 19 November 2026. The pinned drivers replayed all three reports byte-for-byte.

```bash
python3 -B tools/prepare_okapi_geometry.py --check
python3 -B tools/verify_okapi_geometry.py
cargo test --locked -p meridian-core okapi::tests:: -- --nocapture
```

Rust and renderer qualification await owned execution.
