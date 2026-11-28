# SH041 Serval Fighter original hull

The original Serval Fighter is SH041, preserved from immutable L337 revision `0d8cc9ebe0018bf74b0d9fa10ef23b70339dd6b0`. Its selected native registration identifies `fleet-completion-20261019/batch-02-native-lod/SH041-design.toml`; the archived native mesh equals the native mesh in the original resource. One source unit is exactly one metre. The original coordinates, axes, origin, indices, winding and appearance are not altered.

| Original mesh | Vertices | Faces | Connected components |
| --- | ---: | ---: | ---: |
| Detailed | 2,172 | 4,236 | 27 |
| Selected native | 128 | 224 | 8 |

The original native mesh fits the 512-face cap without any replacement LOD. Both packed visual streams retain every original vertex corner, face material, computed source winding normal, alpha, emission, texture identity and UV. The slate-orange source uses the three existing project-authored MIT-licensed surface maps. Unmapped faces keep the original `-1` texture sentinel.

Twenty-three rounded source-local convex supports contain every triangle of both original meshes, with 20 mm rounding. The fuselage, wings, struts, cockpit glazing and small service features retain their own supports. Each drive casing, recess and emitter share that drive's local support. No whole-ship box, scale change or new geometry is introduced.

[Exact Studio definition](../integration/sh041-serval-fighter/SERVAL-STUDIO.json) · [Original source manifest](../assets/fleet/sh041/SOURCE-GEOMETRY.json) · [Criteria](../integration/sh041-serval-fighter/CRITERIA.json)

## Openings and solid controls

The two upper aft channels above the drive struts and the central stern opening remain clear. The net certificate subtracts the 20 mm rounded support radius, 100 mm sphere probe radius and a further 1 mm guard.

| Opening | Net clearance |
| --- | ---: |
| Port upper aft channel | 79 mm |
| Starboard upper aft channel | 79 mm |
| Stern between drives | 179 mm |

Separate source-solid centrelines cross actual triangles of both original meshes. Each measured intersection has positive barycentric weights, not just an AABB overlap.

| Contact | Detailed face | Native face |
| --- | ---: | ---: |
| Lower fuselage | 168 | 37 |
| Starboard wing | 2157 | 149 |
| Port wing | 831 | 79 |

[Opening and contact witnesses](../integration/sh041-serval-fighter/CONTACT-WITNESSES.json)

## Bounded fit to both original surfaces

The independent MBA audit certifies exposed rounded support samples against every original triangle and covers every fan patch with a conservative upper bound, including buried pieces. The upper bound also covers the exposed union; it is not an exact, symmetric or purely exposed Hausdorff value.

| Envelope to original | Strict exposed lower | Complete piece upper |
| --- | ---: | ---: |
| Detailed | 278 mm | 348 mm |
| Native | 1,317 mm | 1,323 mm |

The native witness lies on the detailed-only canopy crest. Original detailed vertex 2054 alone is more than 1,301 mm from every native source triangle. A mandatory native source vertex in the combined stream, 2240, is more than 88 mm from the detailed surface. This is a genuine LOD difference, not an admission of close native fit or a reason to remove the source crest. Both source meshes remain completely enclosed by their own local groups regardless of their visible differences.

[Certified full surface report](../integration/sh041-serval-fighter/SURFACE-FIT.json) · [Direct original-to-original source witness](../integration/sh041-serval-fighter/ORIGINAL-DIFFERENCE.json)

## Physical SH028 station lip

The original 272-vertex, 488-triangle SH028 physics mesh is hash-pinned. At fixed half-turn yaw the aligned actor moves from center Z = 150 m to 116 m across the complete 134.32–136.32 m lip band. Its guarded leading and trailing edges are 137.479 and 133.111 m. Exact binary64 rational separating boxes certify at least 45,138 mm clearance after the 20 mm rounding and 1 mm guard.

The offset actor at X = 69.9 m starts at Z = 150 m with at least 1,159 mm clearance. Original native vertex 26 crosses the interior of source station face 482 at `[69.9, 1.928, 136.32]` m when actor center Z = 148.82 m. All barycentric coordinates are positive. This qualifies a fixed-orientation source witness, not actual runtime docking or flight dispatch.

[Station certificate](../integration/sh041-serval-fighter/STATION-RESULT.json)

## Installed Studio and physical authority

On 19 November 2026 the installed signed Hull Studio exported the exact source definition, with only the final LF omitted from the output. Its bundled validator verified 23 support groups, all 2,300 original ordered coordinates, and three passing opening probes. When the three original solid routes were added, it refused exactly those three while preserving the same support groups. Both reports and the exported package replayed byte-for-byte. No Mac compiler or game launch was performed.

The original authored resource provides 43.6 m/s speed, 20 t cargo, six missile slots, and laser-facing mask 7. The original 23,775 kg dry engineering budget and separate finite-intrinsic empty-dry reference retain their distinct centers and inertia tensors and both explicitly deny native or operating qualification. Active engineering remains null. The recorded crew, purchase and salvage rules are proposals, not physical operation, actual crew, cargo, paid equipment, earned rights or campaign placement. Nothing is granted or inferred by Studio export.

## Replay

```sh
python3 -B tools/prepare_serval_studio.py --check
python3 -B tools/verify_serval_station.py | cmp - integration/sh041-serval-fighter/STATION-RESULT.json
python3 -B tools/measure_serval_surface_fit.py | cmp - integration/sh041-serval-fighter/SURFACE-FIT.json
python3 -B tools/measure_serval_surface_fit.py --difference | cmp - integration/sh041-serval-fighter/ORIGINAL-DIFFERENCE.json
python3 -B -m unittest discover -s tools -p 'test*serval*.py' -v
```

The measured reports replayed on SteveMBA with the pinned scientific dependencies and Python 3.14.3. Report stdout uses canonical UTF-8/LF; raw assets and report paths use `-text` and seven pinned tools use narrow `text eol=lf` protection. The installed Studio command can be reproduced with `tools/verify_serval_studio.py --studio <installed-executable> --validator <bundled-validator> --output <new-evidence-directory>`.

## Stock Showroom and runtime collision

The stock station Showroom opens the signed Studio export under SH041. It uses the original detailed artwork and the shared Hull Studio collision loader. The MacBook Air tested the same 23 pieces against the original station. The aligned hull crossed the complete lip. The offset hull started clear and made contact during its sweep. Active engineering and campaign acquisition remain unassigned.

The two original floating-point reports above are author records from the MacBook Air. Both Linux and native Windows selected different floating-point witnesses on the original PR. The portable fit check uses frozen source facets, exact integer triangle distances and the authored exposed witnesses. It recertifies the 278–348 mm detailed and 1,317–1,323 mm native bounds. The portable LOD check records all equally near faces. Both use the original 23 pieces.

```sh
python3 -B tools/measure_serval_fighter.py | cmp - integration/sh041-serval-fighter/CERTIFIED-FIT.json
python3 -B tools/measure_serval_fighter.py --difference | cmp - integration/sh041-serval-fighter/CERTIFIED-DIFFERENCE.json
```
