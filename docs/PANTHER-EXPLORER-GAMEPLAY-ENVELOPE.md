# Panther Explorer SH032 source envelope

The Panther Explorer has a deep equipment keel, a panoramic pressure
cabin, two curved supports and low drive sleds. Its forest-ivory meshes
come from Elite/L337 revision `a11fe16924d87d06021816f9037952afcbac08db`.
The family registration assigns the Explorer its own original
specification and native archive. The physical contract confirms a scale
of one metre per source unit. The source assigns no stock mass, centre of
mass or inertia.

| Original Explorer mesh | Vertices | Triangles | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 2,866 | 5,596 | 34 |
| Selected native | 206 | 352 | 15 |

The original native archive meets the 512-triangle limit. Both meshes span
`[-9.829, -6.2, -20.09]` to `[9.829, y, 17.0]` metres, where the detailed
canopy crest reaches 7.56 m and the native glazing reaches 7.48 m. Both
packed visual streams retain original face order, winding, positions,
normals, colours, emission, texture selection and UVs.

## Collision support and fit

The envelope uses 36 local convex supports with 20 mm rounding. Each
drive groups its original casing, recess and emitter. Each curved arch is
divided into four spans along original cross-section rings. The collision
and Studio meshes duplicate 72 detailed and 24 native seam vertices so
each original face belongs to one span. Every original face retains its
three corner positions and winding. The archive files retain the original
vertex indices.

| Envelope | Detailed exposed lower bound | Detailed full upper bound | Native exposed lower bound | Native full upper bound |
| --- | ---: | ---: | ---: | ---: |
| One piece per arch | 2,348 mm | 2,428 mm | 2,479 mm | 2,567 mm |
| Four spans per arch | 437 mm | 1,001 mm | 2,127 mm | 2,170 mm |

The full bounds cover all piece surfaces, including the closures between
spans. The native exposed witness is on the right pipe mount. Before any
rounding, an original detailed vertex on that mount already gives a
strict 2,109 mm lower bound to the selected native mesh. The reverse
original-mesh comparison gives a strict 88 mm lower bound near a drive.

[Original registration, fit reports and replay commands](../integration/sh032-panther-explorer/CRITERIA.json)

## Clearance and contact evidence

| Swept triangle on each side | Certified clearance |
| --- | ---: |
| Between keel and sled ahead of the tie | 328 mm |
| Above the sled | 448 mm |
| Below the aft tie | 558 mm |
| Under the main span of the arch | 930 mm |
| Fore arch shoulder | 304 mm |
| Mid arch shoulder | 252 mm |
| Aft arch shoulder | 213 mm |

These exact bounds include a 21 mm guard. The whole arch and each of the
three adjacent pair merges contain a point strictly inside a corresponding
swept triangle on both sides. The eight counterexamples use convex
combinations of original native vertices. This shows why each of the
chosen separations matters to the verified routes.

Ten interior overlaps in both meshes verify opaque structure behind the
panoramic glass and both side windows, the cabin-to-keel connection and
the connections linking each curved support, sponson, aft tie and sled.
The aligned SH028 passage has a conservative 43,218 mm clearance bound.
The offset sweep begins at least 979 mm clear before native bow vertex
2950 in the packed stream crosses the interior of original station
triangle 482 at `[69.4, -1.8, 136.32]` metres. SteveMBA reproduced the
fit, contact and direct original-mesh comparison reports byte for byte.

## Hull Studio definition

The [Panther Explorer definition](../integration/sh032-panther-explorer/PANTHER-EXPLORER-STUDIO.json)
contains both mesh views with shared support groups, original detailed
appearance, the legacy 47 m/s speed and fourteen 100 mm sphere routes.
Hull Studio schema 1 carries detailed face appearance. The original
native appearance remains in the archive and packed stream.

```sh
python3 tools/hull_studio/studio.py --definition integration/sh032-panther-explorer/PANTHER-EXPLORER-STUDIO.json
```

The pinned Studio Python loader accepted the exact definition and
preserved it through a JSON round trip on the MBA. Rust Studio validation
and export, the explicit Panther Explorer Rust contact tests, and
Showroom qualification await their owned runs.

Regenerate with `python3 -B tools/prepare_panther_explorer_studio.py`.
Add `--check` for byte comparison.
