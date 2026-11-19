# Stag Liner SH035 source envelope

The original Stag Liner has a broad main hull, an upper gallery and
two towers linked through diagonal bridges. Both pearl-teal meshes come
from Elite/L337 revision `a11fe16924d87d06021816f9037952afcbac08db`.
The original Stag registration and physical contract verify a scale of
one metre per unit and the identity of the selected native archive.
Both meshes span `[-13.248, -9.0, -36.09]` to `[13.248, 17.5, 33.0]`
metres. The original calibration leaves stock mass, centre of mass
and inertia unassigned.

| Original mesh | Vertices | Triangles | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 1,806 | 3,492 | 30 |
| Selected native | 130 | 216 | 11 |

The selected native mesh meets the 512-face limit. The envelope uses
26 convex supports with 20 mm rounding. Each drive groups its original
casing, recess and emitter. Separate supports keep the space under the
bridges and beside and between the towers open. Both packed visual
streams preserve original positions, winding, normals, colours,
emission, texture choices and UVs.

## MBA verification

| Rounded envelope to original mesh | Certified exposed lower bound | Full upper bound |
| --- | ---: | ---: |
| Detailed | 685 mm | 705 mm |
| Native | 3,035 mm | 3,038 mm |

The native witness lies near the forebody. Detailed source vertex 108
is already more than 3,016 mm from the selected native mesh before
rounding. The reverse original-mesh comparison gives a strict 89 mm
lower bound near a drive.

| Swept triangle | Certified clearance |
| --- | ---: |
| Under each diagonal bridge | 1,440 mm |
| Inboard of each tower | 834 mm |
| Outboard of each tower | 748 mm |
| Centre aisle | 918 mm |

The bounds include a 21 mm guard. Eight strict interior witnesses verify
the main hull-to-gallery joint, opaque support behind the pilot glazing
and both tower windows, and the bridge connection at each gallery and
tower end in both meshes.

The aligned SH028 sweep has at least 33,278 mm clearance. The offset
sweep begins at least 979 mm clear before original native vertex 1818
in the packed stream crosses the interior of SH028 triangle 482 at
`[69.4, -2.0, 136.32]` metres. The fit, original-mesh comparison and exact
contact reports reproduced byte for byte on SteveMBA. The repository
pins the measured files and replay helpers to stable checkout bytes.

[Source pins, exact qualification and replay commands](../integration/sh035-stag-liner/CRITERIA.json)

## Hull Studio definition

The [Stag Liner definition](../integration/sh035-stag-liner/STAG-LINER-STUDIO.json)
contains both original meshes, the detailed appearance, 26 support
groups, the original 35 m/s speed and seven 100 mm sphere routes.
Schema 1 carries detailed face appearance. The preserved native stream
retains the selected native appearance.

```sh
python3 tools/hull_studio/studio.py --definition integration/sh035-stag-liner/STAG-LINER-STUDIO.json
```

The pinned Studio Python loader and JSON round trip passed on the MBA.
The installed signed macOS Studio then validated and exported that exact
definition. All 26 support groups and seven sphere routes passed with no
diagnostics. The output equals the input except for its final line feed.
Separate signed private inspection at `eef206f` visibly showed Stag in
Compare and solo within its disposable profile. The ordinary stock
renderer requires independent qualification; its tests and the per-hull contact
tests await their owned runs in the joined development source.

Regenerate with `python3 -B tools/prepare_stag_liner_studio.py`.
Add `--check` to compare the committed definition byte for byte.

## Ordinary stock Showroom

In the joined development game, press 3 at a station and search **Stag Liner** or `SH035`. No local package flag is required. Compare initially retains the current owned hull, C changes to solo and Pin can compare Stag with Sparrowhawk or Kite. The `LOCAL:SH035` explicit inspection remains separate. Exchange is unavailable.

The stock descriptor authenticates the exact reviewed JSON and its installed-qualified export projection (only the final LF is removed); the shared package validator and renderer consume the actual export bytes. All 10,476 original detailed corner positions, winding, normals and material attributes remain bound, including the 1,240 unmapped faces. The original 26.496 × 26.5 × 69.09 m dimensions are preserved in the uniform presentation fit; neither the native low-detail mesh nor another hull is substituted.

The authored stock tests cover ordinary navigation between Stag, both earlier stock peers and the owned baseline; actual minimum panes; independently prepared exact RGB references, both orders and independently oriented duplicate panes with each peer. They do not repeat private Studio inspection, grant paid ownership, stock mass, new shield shimmer or a campaign encounter. [Stock integration criteria and owner commands](../integration/sh035-stock-showroom/CRITERIA.json)
