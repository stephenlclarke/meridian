# SH036 Zebu original hull

This authored slice uses the original long-haul trader Zebu, SH036, not a predecessor hull or a new design. Both geometry levels are frozen from the immutable L337 revision `0d8cc9ebe0018bf74b0d9fa10ef23b70339dd6b0`; the selected native registration identifies `fleet-completion-20261019/batch-02-native-lod/SH036-design.toml` and its original SHA-256. The exact native geometry in the resource and archived export agrees. One source unit is one metre. No positions, axes, origin, winding or scale are altered.

| Mesh | Vertices | Triangles | Connected components |
| --- | ---: | ---: | ---: |
| Original detailed | 1,790 | 3,468 | 28 |
| Selected native | 114 | 192 | 9 |

The native face count is below 512. The original sand-maroon palette, eight materials, alpha, emissive flags, UVs and three MIT-licensed shared surface maps are preserved. Both packed visual streams include every source corner; unmapped textures retain the original `-1` selector as `0xffffffff`. Native appearance remains in the archived source and its dedicated packed stream; the current Studio schema defines detailed face materials and texture mappings.

## Conservative hull geometry

Twenty-four rounded local convex supports contain every vertex and triangle from both original geometry levels. The lower fuselage, hump, reserve tanks, struts, glazing and service features each retain local support; each drive's casing, recess and emitter share that drive's support. Each local convex hull has 20 mm rounding. Separate supports preserve the two passages alongside the hump and the opening between the rear drives. This is a conservative collision approximation, not a globally measured surface-fit certificate. No whole-ship box or invented geometry is used.

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

## Original physical and gameplay data

The frozen resource carries the original 33 m/s maximum speed, 72 t cargo, 230 shield capacity per bank, four missile slots, and laser-facing mask 3 as authored values. The frozen contract separately records crew, equipment, purchase and salvage proposals; these are design requirements, not installed equipment, earned entitlement, or proof of accepted gameplay. The 64,051 kg dry engineering budget and separate finite-intrinsic empty-dry reference both survive unchanged. They carry distinct centers and inertia tensors and explicitly deny native or operational qualification. The Studio definition leaves active engineering null; no operational mass, loadout, crew, cargo, fuel or campaign placement is invented or granted.

## Reproduce and execute

```sh
python3 -B tools/prepare_zebu_studio.py --check
python3 -B -m unittest discover -s tools -p test_prepare_zebu_studio.py -v
python3 -B tools/verify_zebu_studio_export.py \
  --studio "$HOME/Applications/Meridian Hull Studio.app/Contents/MacOS/Meridian Hull Studio" \
  --validator "$HOME/Applications/Meridian Hull Studio.app/Contents/Frameworks/meridian-hull-tool" \
  --output /tmp/zebu-evidence-new
```

On 19 November 2026, the installed signed Mac Studio exported the exact authoring package; the export differs only by the final newline. The bundled Rust validator reported all 24 original support groups with exact ordered source points, all three opening probes passing, zero diagnostics, and all three original-solid contact probes specifically blocked when submitted as additional clearance routes. The independent scripted replay reproduced both raw reports and the export byte-for-byte. It did not run the game or compile a new Mac build.

To inspect the hull, open the definition file in the generic currently installed Hull Studio, build and validate it, and use its Showroom action in an isolated profile after the owned native lane qualifies this exact hull. The authored source and installed Studio export do not establish native game rendering, ordinary acquisition, mission placement, operational physical behavior or admission into a released catalogue. These remain separate owner gates.
