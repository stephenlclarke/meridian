# Goshawk Mk II Custom (SH010)

SH010 is a separate original hull in the station Ship Showroom. In a development build, open Showroom with 3, search for Goshawk Mk II Custom, and press Enter. C switches between Compare and the individual preview. The original forest-silver model retains the four lower mounting structures, its glass crossbridge and the open wishbone. Exchange remains unavailable until actual acquisition and ordinary hull admission exist.

## Source and engineering

The [SH010 manifest](../assets/fleet/sh010/MANIFEST.json) binds the immutable original resources at revision `a11fe16924d87d06021816f9037952afcbac08db`. Its own detailed and native-candidate resources retain their original face order, winding, materials, texture identities and UVs. Positions are converted once using the adopted engineering ratio of 15000000/1644589 metres per source unit. The original origin, +Y up and +Z forward are preserved.

| Source | Vertices | Faces | Original components |
| --- | ---: | ---: | ---: |
| Detailed visible model | 2,104 | 3,532 | 169 |
| Original candidate | 568 | 1,012 | 31 |

Both source sets span 21.734 × 10.411 × 30.000 m. The visible asset packs the original detailed triangles in their original material/UV order. The existing original texture atlas is validated against its independent source manifest. The original native candidate remains above the 512-face exact qualification ceiling.

The source engineering record `30aba17c31f65a5d1bcb9a29455f45bb876628ce5906d4af5cf16f249d2a79a6` supplies a project-estimated dry mass of 32,411 kg and a centre of mass offset of (-0.259589, 0.092566, -2.197835) m. The full inertia tensor and retained hardware/stock accounting are preserved with the original source. These values are accessible from the SH010 core geometry source.

## Collision and original station fit

The SH010 union uses 73 source-local convex supports with a 20 mm spherical offset. Each triangle is contained within one piece by convexity. The partition follows the local wishbone rails, curved arms, pressure vessels, wells and bracing. It includes four separate lower mount supports found in both own source meshes. Small recessed service wells are bridged locally, while the main forebody channel remains a separate opening. The visual never displays a filled hull in place of the original mesh.

The [independent fit witness](../integration/sh010-goshawk-custom/FIT-WITNESS.json) checks all local triangle assignments and all 73 pieces against selected SH028 station sweeps. It uses 21 mm padding, including a 1 mm arithmetic guard.

| Probe | Certified separation or result |
| --- | ---: |
| Original port, aligned crossing | 44.608 m minimum aperture and station gap |
| Off-aperture crossing | 10.821 m station gap, aperture refused |
| Physical triangle through the forward channel | 1.457 m minimum separating-plane gap |
| Actual SH028 lip at initial approach pose | 0.979 m gap |

The lip witness uses SH010 detailed rail vertex 6 and original SH028 face 482. During a two-metre inward sweep the rail passes through the strict interior of that triangle while the ship origin remains outside the opening. The independent launch bound identifies source vertex 1920, 20 mm rounded padding, and the exact least-outward f32 launch Z coordinate, 1153.363037109375 m.

## Whole-surface clearance trade-off

The [full-surface audit](../integration/sh010-goshawk-custom/SURFACE-FIT.json) independently bounds the directed distance from every rounded support surface to both original meshes. Exact point-to-triangle witnesses certify the reported exposed lower bounds. Fixed-point patch bounds cover unsampled positions and the 73 individual surfaces, including surfaces hidden inside the union.

| Original target | Certified exposed lower bound | Certified whole-surface upper bound |
| --- | ---: | ---: |
| Detailed | 603 mm | 800 mm |
| Native candidate | 1,014 mm | 1,197 mm |

The measured maximum samples exceed the 20 mm nominal spherical padding because the convex supports bridge some service wells and the original source meshes differ. The two sources place a mandatory detailed well vertex at least 998 mm from the native surface and a native-candidate well vertex at least 297 mm from the detailed surface. The 30,228 guarded exposed samples cover 72 pieces. Exposure of the cockpit crossmember remains unresolved, while its independent whole-surface upper bound is included. These data describe the approximation and are separate from native acceptance.

Repeat the surface audit with the pinned dependencies in `tools/requirements-goshawk-surface-fit.txt`:

```bash
python3 -B tools/audit_goshawk_custom_surface.py
```

## Acceptance

Source preparation and the independent Python clearance calculation have been executed. The authored Rust tests cover the exact original triangle domains, channel clearance, original lower mount contact, source alteration refusal, real SH028 alignment and speed/roll/attitude refusal, the lip sweep and the exterior launch. The native or owned platform results remain pending.

The controlled World approach uses the shared Goshawk simulation path with its own SH010 collision source, R3 speed limit of 40.0 and typed snapshot. Each accepted fixed tick sweeps the original station and the stationary SH025 observer. Aligned entry, overspeed refusal, off-aperture refusal, physical lip collision and observer collision have authored World regressions. The terminal pose remains at the first admitted point or the preceding clear pose. No commander, purchase, save or ordinary actor population is created by this fixture.

The Showroom tests exercise the real source identity, authenticated texture/vertex and support shader data, the original projection in solo and Compare at the minimum supported viewport, and refusal of an unavailable hull exchange. Native rendering/shimmer and ordinary placement remain separate acceptance checks. Use the [candidate criteria](../integration/sh010-goshawk-custom/CRITERIA.json).

To verify the source artifacts and the independent numeric witness without launching the application:

```bash
python3 -B tools/prepare_goshawk_custom.py --check
python3 -B tools/verify_goshawk_custom_fit.py
python3 -B -m unittest discover -s tools -p 'test_goshawk_custom.py' -v
python3 -B -m unittest discover -s tools -p 'test_audit_goshawk_custom_surface.py' -v
```
