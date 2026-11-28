# Vulture Mk I SH037 source envelope

The original Mk I uses a stocky russet pressure hull, an all-around
viewing belt, planar shoulder bridges and paired drive pods. Both
original meshes and the adopted engineering record come from Elite/L337
revision `a11fe16924d87d06021816f9037952afcbac08db`. The coordinates
are relative. The adopted scale is exactly `6000/611` metres per
source unit, giving the original detailed hull an axial span of 30 m.
The source origin is retained. The engineering record supplies a dry
mass of 51,330 kg, centre of mass and inertia.

| Mesh | Vertices | Faces |
| --- | ---: | ---: |
| Original detailed | 5,132 | 8,272 |
| Original native | 1,812 | 3,016 |
| Authored Studio proxy | 272 | 512 |

Both original visual streams preserve face order and winding, vertex
positions after the adopted conversion, geometric normals, original
palette and emission. The original meshes have no texture mapping.
The 3,016-face native archive exceeds the 512-face Studio limit.
The Studio native slot uses a separate, eight-piece proxy based on
outward supporting planes of the original parts. The two original
meshes and their encodings remain in the source packet.

The eight local supports cover the pressure hull, cockpit, two shoulders,
two drive bodies and two forward sensors. An exact dyadic check verifies
closed oriented proxy surfaces and places all 6,944 original vertices
strictly inside the faces of their assigned supports before adding
20 mm rounding. The cockpit support encloses the viewing belt and its
backing shell. Four directional sections per mesh show the original
opaque backing sits 227 to 263 mm behind the glazing at those sections.

## Independent gameplay fit and openings

| Rounded proxy to original mesh | Certified exposed lower bound | Full upper bound |
| --- | ---: | ---: |
| Detailed | 931 mm | 986 mm |
| Original native | 941 mm | 1,030 mm |

The direct original-mesh comparison has strict source-vertex lower bounds
of 146 mm from the original native to detail and 323 mm from detail
to the original native. The bounds above use the actual proxy pieces
and compare against both original visual meshes.

| Symmetric swept triangle | Certified clearance on each side |
| --- | ---: |
| Under the shoulder | 243 mm |
| Inboard of the aft drive | 180 mm |
| Outside the forward cockpit | 953 mm |

The exact contact controls include a 21 mm guard. Five original overlaps
link the pressure hull and cockpit and link both shoulder bridges between
the pressure hull and drive pods. The aligned SH028 passage has a
44,572 mm conservative clearance. An offset native bow vertex starts
at least 978 mm clear before crossing SH028 triangle 482 at
`[69.4, 0.7855973813420621, 136.32]` metres. SteveMBA reproduced
the proxy and the fit, original difference and contact reports byte for
byte.

### Cross-platform certificate

The historical reports gave positive-sensor detailed bounds of 945 mm on
the MBA and 905 mm on Windows. The old method allowed the convex-hull
triangulation and floating nearest-triangle query to choose the individual
patch witnesses. Either witness can yield a conservative bound. An
independent run using the authored fan and a 905 mm refinement target
certified the whole positive-sensor piece at 905 mm, which confirms both
historical limits. The full report retains the original refinement targets.
Its positive-sensor detailed bound is 944 mm and the two hull-wide bounds
are 986 mm and 1,030 mm.

Both platforms also measured the same distance from source vertex 6038.
Five original triangles share its exact nearest point. The report lists
faces 3654, 3655, 3658, 3659 and 3661, using 3654 as the lowest-index
representative. Patch bounds now use the frozen authored fan and exact
integers. The direct comparison searches exact distances over both full
meshes. The original reports and their hashes remain available.

[Evidence and replay commands](../integration/sh037-vulture-mk-i/CRITERIA.json)

## Hull Studio

Open the [Vulture Mk I definition](../integration/sh037-vulture-mk-i/VULTURE-MK-I-STUDIO.json)
to inspect the original detailed appearance, engineering, 512-face native
proxy and six 100 mm sphere probes using the adopted scale.

```sh
python3 tools/hull_studio/studio.py --definition integration/sh037-vulture-mk-i/VULTURE-MK-I-STUDIO.json
```

The installed signed Studio exported the exact definition with all eight
groups and six sphere routes passing and zero diagnostics. Its output
matches the input except for the final line feed. On the same installed
tool, replacing just the native slot with the untouched 3,016-face mesh
exited 1 with the 512-face-limit diagnostic and created no export. The
packet contains both process receipts and inputs. The stock Showroom now resolves SH037 through the same signed export. The runtime uses the eight-piece envelope against the original SH028 station. This inspection package grants no purchase, flight or docking rights.

Regenerate with `python3 -B tools/prepare_vulture_mk_i_studio.py`.
Add `--check` to compare the committed definition byte for byte.
