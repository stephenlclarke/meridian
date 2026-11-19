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
The per-hull Rust contact tests and a Stag Liner Showroom launch await
their owned runs.

Regenerate with `python3 -B tools/prepare_stag_liner_studio.py`.
Use `--check` to verify the current definition byte for byte. An explicit `--output` may select the canonical definition or a regular JSON file in an existing system temporary directory. The generator refuses paths to other repository files and symbolic-link redirects before reading or writing the output.
Add `--check` to compare the committed definition byte for byte.
