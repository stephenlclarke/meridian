# Sable Liner source shield

SH012 is the original Sable Liner. Meridian already carries its R3 catalogue
and specifications. The legacy revision `a11fe169` binds the approved
plum-gold SH012 design and metre coordinates. The detailed 2,308 vertices
and 4,496 triangles and native 132 vertices and 228 triangles retain the
original positions and winding. The native export matches the staged
resource and is within the 512-face limit. Controlled Meridian collision,
rendering and ordinary admission have their own qualification.

The shield contains every original triangle using 47 convex supports and
20 mm of spherical rounding. Corresponding pressure hull, drive and bridge
assemblies share local supports. The crescent uses ten detailed and eight
native supports along existing ring seams. Seam vertices are shared by
neighbouring supports. A single support across the crescent would close the
opening. The independent 12 m sweep from the open side retains at least
0.829 m of guarded clearance. A probe at original detailed rim vertex 194
provides the collision control.

The aligned SH028 sweep certifies at least 39.518 m of separation from the
original station triangles. The offset lip sweep begins with 0.979 m of
certified clearance, then the original native front vertex reaches the
strict interior of station face 482 halfway through the transition.

The fit audit brackets the directed exposed shield surface maximum within
1.340–1.592 m of detailed and 2.719–2.727 m of native. The first exposed
witness lies on the coarse native crescent, and the second is a detailed
pipe fitting omitted from native. The source comparison separately certifies
that detailed vertex 2251 is over 2.705 m from any native triangle. An
interior point of original native face 22 is over 2.022 m from the detailed
surface. Direct original-to-original distances include buried points, so
they measure a different quantity than exposed shield distance. Further
splitting cannot eliminate the need to contain those source points.

## Visual preparation

The packed visual keeps all 13,488 detailed corners in original triangle
order, with flat normals derived from the original winding and the
original plum-gold face colours, emissivity and UVs. The established
original patrol, strike and service-detail textures retain their exact
identities. The 1,240 deliberately untextured faces use the unsigned
`0xffffffff` marker. A display adapter needs to use the original face
colour for that marker.

## Qualification

SteveMBA regenerated the geometry and visual and measured both surfaces
from 07:19:54 to 07:20:12 UTC on 19 November 2026. The same Air checked the
exact contact and original-to-original witnesses from 07:22:52 to 07:22:54.
The immutable results are under `integration/sh012-sable-liner`.

```bash
python3 -B tools/prepare_sable_liner_geometry.py --check
python3 -B tools/verify_sable_liner_geometry.py
cargo test --locked -p meridian-core sable_liner::tests:: -- --nocapture
```

The three Rust regressions exercise seam coverage, the opening and native
station contact. The source work leaves the stock mass and inertia decision,
ordinary insertion and native drawing to their respective owners.
