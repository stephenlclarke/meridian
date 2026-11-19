# Sparrowhawk SH030 source envelope

The original Sparrowhawk is a forward-swept slab interceptor with the
ice-cobalt palette. Both meshes come from Elite/L337 revision
`a11fe16924d87d06021816f9037952afcbac08db`. The original activation and
physical contract verify the selected native archive and a scale of one
metre per model unit. The original bounds are
`[-12.0, -2.8, -12.5]` to `[12.0, 5.5, 12.5]` metres for both meshes.

| Original mesh | Vertices | Triangles | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 1,612 | 3,108 | 29 |
| Selected native | 128 | 216 | 10 |

The native source meets the 512-triangle limit. The collision envelope
uses 25 local convex supports with 20 mm rounding. The source casing,
recess and emitter form a shared support on each drive. Separate supports
retain the longitudinal bow, underslab and inboard dorsal passages.
The packed visual streams preserve original winding and positions, face
normals, material colours, emission, atlas selections and UVs. Native and
detailed appearance data are both available in the frozen source assets.

## MBA geometry evidence

| Distance from rounded envelope to original mesh | Certified lower bound | Full upper bound |
| --- | ---: | ---: |
| Detailed | 274 mm | 295 mm |
| Native | 1,325 mm | 1,328 mm |

The native worst-case sample lies at the left pipe mount, which the
native mesh omits. An original detailed vertex there already establishes
a strict 1,306 mm lower bound to the native surface before rounding. The
reverse source-vertex comparison establishes a strict 88 mm lower bound
from the native drive to the detailed surface. The measured distances
include the differences between the original meshes.

On each side, exact separating planes certify swept triangles in the bow
channel (1,706 mm), below the slab (528 mm) and inboard of the dorsal fin
(599 mm) after a 21 mm guard. Five strict interior overlap witnesses cover
the opaque fuselage beneath the glazing and both slabs where they meet the
fuselage and booms. Each overlap is present in both meshes. The aligned
SH028 sweep has a conservative 45,278 mm clearance bound. The offset pose
starts at least 979 mm clear before native vertex 1636 crosses the interior
of the original SH028 station face 482 at `[69.4, 0, 136.32]` metres. The
source checks and fit reports reproduced byte for byte on SteveMBA.

[Source pins, reproducible commands and evidence](../integration/sh030-sparrowhawk/CRITERIA.json)

## Hull Studio definition

Open the [Sparrowhawk definition](../integration/sh030-sparrowhawk/SPARROWHAWK-STUDIO.json)
from Hull Studio using its Open command or the following terminal command
from a checkout that includes Hull Studio.

```sh
python3 tools/hull_studio/studio.py --definition integration/sh030-sparrowhawk/SPARROWHAWK-STUDIO.json
```

The definition embeds both original meshes, shared group numbers,
detailed appearance, the original 51 m/s speed and six 100 mm radius
probes through the certified passages. Stock mass, centre of mass and
inertia remain unassigned in the original physical source. Schema 1
carries detailed face appearance. The original native face appearance
remains available in its source and encoded stream.

The pinned Studio Python loader accepted the definition and preserved it
through preparation and a JSON round trip on SteveMBA. The orchestrator
then ran the Rust Studio validator and export on the exact definition at
`12d24b8`. The validator accepted all 25 groups and all six 100 mm sphere
routes. The exported JSON preserved the original definition and its
SHA-256 is recorded in the evidence. The per-hull Rust contact regressions
and a Sparrowhawk Showroom launch await the owned run. Campaign encounters,
blueprint fabrication and installed preview acceptance remain separate.

Regenerate with `python3 -B tools/prepare_sparrowhawk_studio.py`.
Run the byte check with `python3 -B tools/prepare_sparrowhawk_studio.py --check`.
