# Kite Interceptor SH031 source envelope

The original Kite Interceptor has twin vertical blades and a navy-copper
finish. Both meshes come from Elite/L337 revision
`a11fe16924d87d06021816f9037952afcbac08db`. The original activation and
physical contract verify one metre per model unit for the selected native
archive. Both meshes span `[-6.67, -6.5, -19.5]` to `[6.67, 6.5, 19.5]`
metres.

| Original mesh | Vertices | Triangles | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 1,612 | 3,108 | 29 |
| Selected native | 96 | 160 | 8 |

The selected native mesh meets the 512-triangle limit. The hull envelope
uses 25 convex supports with 20 mm rounding. Each drive support groups
the original casing, recess and emitter. Separate supports retain the
bow and both sides of each blade. The detailed cross-braces have their
own supports. Both packed visual streams preserve original positions,
winding, face normals, colours, emission, texture assignment and UVs.

## MBA geometry evidence

| Distance from rounded envelope to original mesh | Certified exposed lower bound | Full upper bound |
| --- | ---: | ---: |
| Detailed | 210 mm | 321 mm |
| Native | 1,229 mm | 1,438 mm |

The native exposed witness lies on the left pipe mount. Separately,
detailed source vertex 399 on the left cross-brace gives a strict
1,405 mm lower bound to the native mesh before adding rounding. The
reverse source-vertex comparison gives a strict 88 mm lower bound from
the native drive to the detailed mesh.

Exact separating planes certify a swept triangle at 979 mm in both bow
passages, 448 mm inboard of both blades and 548 mm outboard of both blades.
The bounds include a 21 mm guard. Five interior witnesses verify the
opaque fuselage beneath the glazing and each blade’s overlaps with the
fuselage and its nacelle in both meshes.

The aligned SH028 sweep has a conservative 44,278 mm clearance bound.
The offset pose begins at least 979 mm clear before native vertex 1624
crosses original SH028 source triangle 482 at `[69.4, 0, 136.32]` metres.
The fit, contact and direct original-mesh comparison reports reproduced
byte for byte on SteveMBA.

[Source pins, reproducible commands and evidence](../integration/sh031-kite-interceptor/CRITERIA.json)

## Hull Studio definition

Open the [Kite Interceptor definition](../integration/sh031-kite-interceptor/KITE-INTERCEPTOR-STUDIO.json)
with the Hull Studio Open command or start Studio with this definition
from a checkout that includes Hull Studio.

```sh
python3 tools/hull_studio/studio.py --definition integration/sh031-kite-interceptor/KITE-INTERCEPTOR-STUDIO.json
```

The definition carries both original meshes, their shared groups, the
original detailed appearance, the original 55 m/s speed and six
100 mm sphere routes through the verified passages. The original native
appearance remains in the frozen asset and packed native stream. Mass,
centre of mass and inertia remain unassigned in the original calibration.

The pinned Hull Studio Python loader on the MBA accepted the definition
and preserved it through a JSON round trip. The installed signed macOS
Rust Studio validator then accepted all 25 support groups and all six
sphere routes on that exact definition. Its report records the original
support points and 20 mm rounding. The installed signed Studio exported
the exact definition from `f5f7a36` with all 25 groups and six routes passing.
The resulting JSON matches the input except for its final line feed. The
per-hull Rust contact tests and a Kite Interceptor Showroom launch await
their owned runs. Campaign placement and acquisition remain separate.

Regenerate the definition with `python3 -B tools/prepare_kite_interceptor_studio.py`.
Add `--check` to check the committed definition.
