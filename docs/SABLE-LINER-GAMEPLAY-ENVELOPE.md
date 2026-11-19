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

## Station Showroom

The packed visual keeps all 13,488 detailed corners in original triangle
order, with flat normals derived from the original winding and the
original plum-gold face colours, emissivity and UVs. The established
original patrol, strike and service-detail textures retain their exact
identities. The 1,240 deliberately untextured faces use the unsigned
`0xffffffff` marker. The production adapter uses exact face colour for that
marker with the same colour-space conversion as the textured faces. The three
original maps still use repeating UVs. SH001 and all three Goshawk hulls retain
the strict textured-only decoder, with unknown layers and sentinel mutation
refused.

Open the station Showroom with 3 and search for Sable Liner. The selected hull
uses the original opaque pass and subtle shimmer from the same 47 source
supports. Compare keeps the current ship in its own pane. Selection and rotation
do not alter ownership, inventory or paid fitting. SH012 uses its own immutable
pass and visual key; the shared cache now supports up to two passes for each of
the four original hulls without reusing another hull's pass.

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

The three Rust regressions exercise seam coverage, the opening and station
contact. Stock mass and inertia remain unassigned. Ordinary insertion and native
drawing require their respective qualifications.

The Showroom qualification covers the 640×360 solo and Compare frames, actual
nonblack RGB with at least two logical pixels of own-pane clearance, and the
exact isolated full-target union, including black gutters. It also verifies all
13,488 positions and winding, independently pinned colour and UV samples
including untextured source face 2156, all 47 supports and the WGSL layout. The
shared shader's sentinel branch is byte-identical to G4's Air-checked source.
Native Metal checks for SH012 and the existing SH008–SH010 regressions remain
owned executions on this combined head.

```bash
cargo test --locked -p meridian-desktop sable_liner
cargo test --locked -p meridian-desktop metal_sable_liner_untextured_face_uses_exact_source_colour -- --ignored --nocapture
MERIDIAN_SH012_EVIDENCE_DIR=/absolute/fresh/sh012-shimmer cargo test --locked -p meridian-desktop metal_sable_liner_same_pose_shimmer -- --ignored --nocapture
MERIDIAN_SH012_MINIMUM_DIR=/absolute/fresh/sh012-minimum cargo test --locked -p meridian-desktop metal_sable_liner_minimum_showroom_model_pass -- --ignored --nocapture
python3 -B tools/verify_goshawk_showroom_evidence.py /absolute/fresh/sh012-shimmer
```

The minimum capture writes `rgb-qualification.tsv` as well as the original
PPM files and pane metadata. All original ship cache orders, including SH012
and each Goshawk, and duplicate-hull panes are covered by the portable cache
allocation test; the SH009 and SH010 native mixed-cache tests are retained.

## Reproducing the surface measurements

With the packages in `tools/requirements-goshawk-surface-fit.txt` installed,
run the two original Air adapters from the repository root. The criteria
record binds both adapters and their original evidence archive.

```bash
audit_dir=$(mktemp -d)
python3 -B tools/measure_sable_liner_air.py > "$audit_dir/SURFACE-FIT.json"
python3 -B tools/compare_sable_originals.py > "$audit_dir/ORIGINAL-DIFFERENCE.json"
cmp "$audit_dir/SURFACE-FIT.json" integration/sh012-sable-liner/SURFACE-FIT.json
cmp "$audit_dir/ORIGINAL-DIFFERENCE.json" integration/sh012-sable-liner/ORIGINAL-DIFFERENCE.json
```
