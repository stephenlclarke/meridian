# SH036 Zebu original hull

This authored slice uses the original long-haul trader Zebu, SH036, not a predecessor hull or a new design. Both geometry levels are frozen from the immutable L337 revision `0d8cc9ebe0018bf74b0d9fa10ef23b70339dd6b0`; the selected native registration identifies `fleet-completion-20261019/batch-02-native-lod/SH036-design.toml` and its original SHA-256. The exact native geometry in the resource and archived export agrees. One source unit is one metre. No positions, axes, origin, winding or scale are altered.

| Mesh | Vertices | Triangles | Connected components |
| --- | ---: | ---: | ---: |
| Original detailed | 1,790 | 3,468 | 28 |
| Selected native | 114 | 192 | 9 |

The native face count is below 512. The original sand-maroon palette, eight materials, alpha, emissive flags, UVs and three MIT-licensed shared surface maps are preserved. Both packed visual streams include every source corner; unmapped textures retain the original `-1` selector as `0xffffffff`. Native appearance remains in the archived source and its dedicated packed stream; the current Studio schema defines detailed face materials and texture mappings.

## Conservative hull geometry

Twenty-four rounded local convex supports contain every vertex and triangle from both original geometry levels. The lower fuselage, hump, reserve tanks, struts, glazing and service features each retain local support; each drive's casing, recess and emitter share that drive's support. Each local convex hull has 20 mm rounding. Separate supports preserve the two passages alongside the hump and the opening between the rear drives. No whole-ship box or invented geometry is used.

The following net clearances are certified using a separating coordinate axis for every piece along each straight swept-sphere route, subtracting rounding, sphere radius and a further 1 mm guard.

| Opening | Sphere radius | Conservative net clearance |
| --- | ---: | ---: |
| Port upper reserve channel | 0.05 m | 0.029 m |
| Starboard upper reserve channel | 0.05 m | 0.029 m |
| Stern between drives | 0.10 m | 0.179 m |

Three other centerline sweeps cross the interior of actual source triangles, rather than only intersecting a broad bound. The independent witness records the detected triangle, fractional travel, intersection position and barycentric weights for both source meshes.

| Contact probe | Detailed face | Native face |
| --- | ---: | ---: |
| Lower fuselage | 169 | 19 |
| Port reserve | 911 | 79 |
| Starboard reserve | 1645 | 137 |

[Exact definition](../integration/sh036-zebu/ZEBU-STUDIO.json) · [Opening and contact witnesses](../integration/sh036-zebu/CONTACT-WITNESSES.json) · [Criteria and validation evidence](../integration/sh036-zebu/CRITERIA.json) · [Source manifest](../assets/fleet/sh036/SOURCE-GEOMETRY.json)

## Bounded fit against both original meshes

An independent MBA replay checked all 24 exact support fans, including their radial coverage. The certified exposed lower bound proves at least one rounded surface point is outside every other rounded piece and farther than the bound from every original triangle. The full upper bound covers every patch of every rounded piece, including buried surfaces; it is therefore conservative for the exposed union. Eight adaptive subdivision levels give the detailed upper bound below. No sampled point or viewport image is treated as a global bound.

| Distance from exposed envelope to original surface | Strict exposed lower | Full upper |
| --- | ---: | ---: |
| Detailed mesh | 546 mm | 691 mm |
| Native mesh | 4,980 mm | 4,989 mm |

The native discrepancy is large evidence of the source-level difference, not an acceptable assertion of close native fit. Its exposed witness is on the starboard pipe mount. Original detailed vertex 1744 alone is strictly more than 4,967 mm from every original native triangle; conversely, original native vertex 1852 in the combined stream gives an 88 mm strict lower bound to the detailed mesh. The active envelope must preserve both sources, so deleting the original pipe support to reduce the native-distance result would invalidate the source. Any closer native representation would require a separately authored and qualified native variant. The global upper bound is not an exact or symmetric Hausdorff distance, and this does not establish acceptable visual/gameplay fit.

[Complete patch bounds and certified witness positions](../integration/sh036-zebu/SURFACE-FIT.json) · [Direct original-to-original difference](../integration/sh036-zebu/ORIGINAL-DIFFERENCE.json)

## Physical crossing of original SH028

The original 272-vertex, 488-triangle SH028 physics mesh is hash-pinned. The aligned Zebu has a half-turn yaw, zero lateral offset, and translates from actor center Z = 163 m to 112 m at fixed orientation. The guarded leading and trailing edges begin at 137.479 m and finish at 133.111 m, fully crossing the 134.32–136.32 m lip band. Exact rational swept support bounds and the actual station triangle bounds certify at least 39,038 mm net clearance, including rounding and a 1 mm guard.

Offset the actor to X = 69.9 m, retaining its orientation, and move from Z = 163 m to 160 m. The initial rounded hull is still at least 1,159 mm clear. Original selected native vertex 31 (combined-stream vertex 1821) crosses the interior of original station triangle 482 at `[69.9, 5.03, 136.32]` m, when the actor center reaches Z = 161.82 m. All three exact barycentric weights are positive. This is a genuine native-source contact, not a control inferred from an AABB overlap. These are fixed-orientation source certificates, not evidence of a live station approach, rotating body or game physics dispatch.

[Station fixture and source pins](../integration/sh036-zebu/STATION-FIXTURE.json) · [Exact station result](../integration/sh036-zebu/STATION-RESULT.json)

## Original physical and gameplay data

The frozen resource carries the original 33 m/s maximum speed, 72 t cargo, 230 shield capacity per bank, four missile slots, and laser-facing mask 3 as authored values. The frozen contract separately records crew, equipment, purchase and salvage proposals; these are design requirements, not installed equipment, earned entitlement, or proof of accepted gameplay. The 64,051 kg dry engineering budget and separate finite-intrinsic empty-dry reference both survive unchanged. They carry distinct centers and inertia tensors and explicitly deny native or operational qualification. The Studio definition leaves active engineering null; no operational mass, loadout, crew, cargo, fuel or campaign placement is invented or granted.

## Reproduce and execute

```sh
python3 -B tools/prepare_zebu_studio.py --check
python3 -B -m unittest discover -s tools -p test_prepare_zebu_studio.py -v
python3 -B tools/verify_zebu_station.py | cmp - integration/sh036-zebu/STATION-RESULT.json
python3 -B tools/measure_zebu_surface_fit.py | cmp - integration/sh036-zebu/SURFACE-FIT.json
python3 -B tools/measure_zebu_surface_fit.py --difference | cmp - integration/sh036-zebu/ORIGINAL-DIFFERENCE.json
python3 -B tools/verify_zebu_studio_export.py \
  --studio "$HOME/Applications/Meridian Hull Studio.app/Contents/MacOS/Meridian Hull Studio" \
  --validator "$HOME/Applications/Meridian Hull Studio.app/Contents/Frameworks/meridian-hull-tool" \
  --output /tmp/zebu-evidence-new
```

On 19 November 2026, the installed signed Mac Studio exported the exact authoring package; the export differs only by the final newline. The bundled Rust validator reported all 24 original support groups with exact ordered source points, all three opening probes passing, zero diagnostics, and all three original-solid contact probes specifically blocked when submitted as additional clearance routes. The independent scripted replay reproduced both raw reports and the export byte-for-byte. It did not run the game or compile a new Mac build.

The surface fit and direct difference also reproduced byte-for-byte on SteveMBA with Python 3.14.3 and the pinned scientific dependencies. The separate station verifier uses only the standard library and exact original binary64 rational values. Those three source reports are independent of the installed Studio proof. The pinned physical references, station physics bytes and report JSON use explicit `-text` Git attributes; the three hash-pinned Python certificate tools use `text eol=lf`. A fresh CRLF checkout preserved all 26 pinned repository inputs, while an unrelated Python file retained normal CRLF conversion. The first native Windows replay confirmed all 26 pins but exposed text-stdout conversion: the 969-byte station report became 1,007 bytes. Both Zebu certificate commands now write canonical UTF-8/LF bytes directly to redirected stdout, preserving the immutable report hashes. Regression tests reproduce the former CRLF failure and pass with the corrected output. All three reports replayed byte-for-byte on SteveMBA; Native Windows subsequently passed seven focused tests and reproduced the exact station bytes at the corrected source revision.

## Stock Showroom candidate

Search for **SH036** or **Zebu** in the ordinary station Showroom. The stock descriptor authenticates the archived definition and its signed Studio export. The presentation test compares all 10,404 rendered corners with the original artwork. The candidate grants inspection only.

The production validator blocks both upper channels and the gap between the drives when all source vertices share one convex group. The original 24 groups keep all three passages open and block all three solid probes. The runtime station test uses these same groups. It clears the aligned crossing, starts clear at the offset and detects contact during the offset motion. The original shield capacity remains reference data.

The fixed support fans give complete exposed-to-source bounds of 546–690 mm for the detailed mesh and 4,980–4,989 mm for the native mesh. The direct original-mesh check tests all opposite-source vertices and reports every face tied for the nearest distance at the worst vertex. Routine tests recalculate both certificates and compare their output bytes with the committed receipts. The earlier author reports remain available above.

[Portable fit certificate](../integration/sh036-zebu/CERTIFIED-FIT.json) · [Portable original-mesh difference](../integration/sh036-zebu/CERTIFIED-DIFFERENCE.json)
