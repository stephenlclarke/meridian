# Harrier source shield

SH026 is the original Harrier. Meridian lists it for blueprint fabrication. The active bare resource at legacy revision `a11fe169` already uses metres. Its authoring scale was 30/4.192 metres per source unit. Both active meshes preserve their source axes, origin and 30 m nose-to-tail span.

| Active original mesh | Vertices | Triangles | Index-connected components |
| --- | ---: | ---: | ---: |
| Detailed | 5,764 | 8,884 | 661 |
| Native | 306 | 512 | 25 |

The native mesh meets the 512-face limit. The frozen legacy fleet and C01 consumers both select the native mesh from the active ship resource. The older 2,852-face export is retained as provenance. The active meshes preserve the four empty rails and remove the carried preview hardware. The original 25,545 kg dry profile remains source evidence. Meridian assigns no runtime mass in this candidate.

## Pod bridges and rail channels

The candidate covers all 9,396 source triangles with 180 local convex supports and 20 mm spherical rounding. Each major body assembly, four pod bridges, pylons, guide rails and canopy components keep separate supports. The 139 vent-panel supports group each original mount only with its adjoining baffles. Sixteen small central service panels retain their own supports.

| Swept control | Travel | Guarded clearance |
| --- | ---: | ---: |
| Left pod channel | 5.35 m | 1.036 m |
| Right pod channel | 5.35 m | 0.851 m |
| Each empty rail channel | 4.65 m | 99 mm |
| Aligned motion at the original SH028 port | 2 m | 46.493 m |
| Offset approach before lip contact | Static start | 0.979 m |

Exact interior rays verify that each pod bridge overlaps the original pressure body and its pod. They also verify the housing's overlap with the body, hammerhead and all four glazing panels. Every joint is checked against both source meshes. These are geometric intersections. Structural strength and all-view rendering remain open.

The solid rail controls use original detailed vertices 5588 and 5620. Native hammerhead vertex 48 crosses the strict interior of original station lip face 482 halfway through the offset sweep.

## Fit and original visuals

The certified directed exposed-surface fit is 154 to 195 mm against detailed and 1.168 to 1.170 m against native. The detailed peak is on the central body and the native peak is at a detailed central panel. One required detailed vertex already lies more than 1.148 m from native. The measurements cover the stated sweeps and the shield's surface bounds. Individual vent interiors remain unmeasured.

The orange-and-black detailed and native `HARRVIZ1` streams preserve original face order, normals derived from winding, tint and texture assignments across 26,652 and 1,536 packed corners. All three source maps retain their original bytes. The 200 detailed and 88 native unmapped faces keep the `0xffffffff` selector. Their source UV arrays are empty and their packed records carry zero placeholders.

SteveMBA verified input topology, both visual encodings, fit, source difference and contact on 19 November 2026. The pinned drivers replayed all three reports byte-for-byte.

```bash
python3 -B tools/prepare_harrier_geometry.py --check
python3 -B tools/verify_harrier_geometry.py
cargo test --locked -p meridian-core harrier::tests:: -- --nocapture
```

The stock Showroom package uses the active bare source and the 180 original
groups. Schema 1 stores three zero UV pairs for an unmapped triangle. Both
original packed visual streams retain the same zero values. The desktop
corner test checks the detailed mesh against its packed stream.

The portable fit verifier bounds the rounded support surface at 154–195 mm
from the detailed mesh and 1,168–1,170 mm from the native mesh. Detailed
source vertex 4848 alone lies more than 1,148 mm from the native mesh.
The core and Studio checks include both open pod channels, both empty rails
and solid rail contacts at source vertices 5588 and 5620.
