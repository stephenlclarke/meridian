# SH039 Vulture Mk III: original and derived candidate

The frozen original **Vulture Mk III** is SH039, distinct from SH037 and SH038. Its original source and selected native registration identify `replacement-hulls/SH039-design.toml` at legacy revision `0d8cc9ebe0018bf74b0d9fa10ef23b70339dd6b0`. All six original input files, including the MIT licence, relative source calibration, creator ownership record and engineering budget, retain exact bytes. The archived native mesh matches the original resource. No sibling Vulture geometry was imported.

The original coordinates use relative units. The original engineering record gives source length `611/200` and the calibrated 30 m detailed length; applying the exact ratio `6000/611` once produces the frozen metre streams. The Studio definition retains the original relative coordinates and scale. The original dark russet, graphite and silver palette, seven materials, face assignments, and emissive drive elements are preserved; the original source uses no texture mapping.

| Source | Vertices | Faces | Index-connected components |
| --- | ---: | ---: | ---: |
| Original detailed | 5,132 | 8,272 | 501 |
| Archived native | 1,812 | 3,016 | 155 |
| Separate derived native proposal | 274 | 508 | 13 |

The original archived native geometry **fails** the existing 512-face cap. The installed signed Studio validator rejected the exact unchanged archived package with the explicit native face-limit diagnostic. Neither the cap nor original mesh was changed.

## Separate capped proposal

The separate configuration has ID `SH039-DERIVED-NATIVE-508`. It selects existing archived points from the fuselage, opaque cockpit, glazing band and rims, wings, nacelles, drive faces and fore tubes. It retains every axial ring while rebuilding the original strip winding from six or eight archived positions per ring and retains wing triangles unchanged. No new source coordinates are invented. The candidate drops surface-decoration features from its low-detail presentation but leaves the complete detailed and archived appearance and their separate packed visual streams untouched. All 508 candidate triangles are grouped and reference only 274 retained archived points. Both original and candidate retain their distinct identities.

The installed signed Studio exports the candidate, with 169 rounded groups built from the unchanged detailed mesh and candidate native mesh. The original source has 177 groups from both archived meshes. The export exactly matches the candidate input except the final LF; all three positive paths pass, all three added source-solid probes are refused, and the 5,406 installed support points equal the source coordinates multiplied by the configured binary64 scale. Their largest difference from the separately calibrated exact decimal-rational metre stream is 3.56e-15 m. These are data authoring controls, not game admission or native rendering checks.

[Original Studio input](../integration/sh039-vulture-mk3/VULTURE-MK3-ORIGINAL-STUDIO.json) · [Derived candidate input](../integration/sh039-vulture-mk3/VULTURE-MK3-CANDIDATE-STUDIO.json) · [Candidate topology and source indices](../assets/fleet/sh039/DERIVED-NATIVE.json) · [Source manifest](../assets/fleet/sh039/SOURCE-GEOMETRY.json)

## Opening and solid controls

Local rounded supports keep the two aft passages alongside the fuselage and the transverse opening behind it. A 20 mm rounding radius, 100 mm probe radius and further 1 mm guard are included in the conservative separating-axis controls. Both original and candidate support unions certify 271 mm net clearance on each aft route and 173 mm on the stern cross-route. The three other probe centrelines cross triangles in all three source variants.

| Solid control | Detailed face | Archived native face | Candidate face |
| --- | ---: | ---: | ---: |
| Under fuselage | 89 | 53 | 29 |
| Starboard nacelle | 2671 | 1287 | 329 |
| Port nacelle | 5919 | 2331 | 425 |

The original 488-triangle SH028 station physics mesh is hash-pinned. At a fixed half-turn yaw, the whole aligned Vulture hull translates from Z = 152 m to 117 m across the entire 134.32–136.32 m lip band. Both support unions have at least 43,099 mm guarded clearance. The offset start at X = 72.6 m is 1,420 mm clear. The retained original native vertex 120 (candidate vertex 72) then crosses the interior of station face 482 at `[69.899509, 0.981997, 136.32]` m when the actor center reaches Z = 150.558953 m. All barycentric weights are strictly positive. This is an exact fixed-pose source witness, not an actual gameplay physics dispatch.

[Opening and solid controls](../integration/sh039-vulture-mk3/CONTACT-WITNESSES.json) · [Station certificate](../integration/sh039-vulture-mk3/STATION-RESULT.json)

## Measured surface fidelity and admission blocker

The independent MBA surface audit computes strict lower witnesses against every original triangle and certified upper bounds over every surface patch. It does not infer global fit from source vertices or viewport screenshots. For the two original and candidate rounded unions the exposed lower and conservative full-piece upper bounds coincide; the latter includes buried surfaces, especially within convex glazing/rim supports. It is not an exact maximum of exposed surface distance.

| Rounded union to original mesh | Exposed lower | Full upper |
| --- | ---: | ---: |
| Original to detailed | 200 mm | 3,470 mm |
| Original to archived native | 343 mm | 3,436 mm |
| Candidate to detailed | 200 mm | 3,470 mm |
| Candidate to archived native | 343 mm | 3,436 mm |

The separate visual surface comparison gives directed bounds, including the omitted original decorative shapes. Each lower bound derives from an original vertex or an exact triangle centroid. Each upper bound covers the complete triangulated surface, not just vertices.

| Directed surface difference | Strict lower | Full upper |
| --- | ---: | ---: |
| Detailed to candidate | 1,161 mm | 1,188 mm |
| Candidate to detailed | 783 mm | 916 mm |
| Archived native to candidate | 1,174 mm | 1,200 mm |
| Candidate to archived native | 646 mm | 927 mm |

The candidate **does not retain complete original collision coverage**. The archived native vertex 1017 lies strictly more than 125 mm outside every rounded candidate support, certified with exact integer separating axes. Together with the large directed surface differences, this is a recorded admission blocker if the original source must retain its collision coverage. There is no verified closer compliant native variant in this packet. The candidate remains a separately named authoring proposal, not a replacement for the original archived native mesh and not a playable or purchasable ship.

[Original envelope fit](../integration/sh039-vulture-mk3/ORIGINAL-ENVELOPE-FIT.json) · [Candidate envelope fit](../integration/sh039-vulture-mk3/CANDIDATE-ENVELOPE-FIT.json) · [Directed surface comparisons](../integration/sh039-vulture-mk3/DIRECTED-FIT.json) · [Exact outside-coverage witness](../integration/sh039-vulture-mk3/COVERAGE-GAP.json)

## Physical and mission data

The original authored source specifies maximum speed 52.5 m/s, 24 t cargo, seven missile slots, and laser-facing mask 7. The original engineering budget contains a 64,757 kg authored empty-dry reference and a center and inertia tensor. It explicitly grants no native or physical-operating qualification, crew, occupied cabins, paid equipment or service rights. The active engineering field remains null, and no mission placement, acquisition, population or loaded-state authority is inferred. The frozen design contract and physical records retain these proposals for future qualified use.

## Reproduction

```sh
python3 -B tools/prepare_vulture_mk3_studio.py --check
python3 -B tools/verify_vulture_mk3_station.py | cmp - integration/sh039-vulture-mk3/STATION-RESULT.json
python3 -B tools/measure_vulture_mk3_fit.py | cmp - integration/sh039-vulture-mk3/DIRECTED-FIT.json
python3 -B tools/measure_vulture_mk3_fit.py --union candidate | cmp - integration/sh039-vulture-mk3/CANDIDATE-ENVELOPE-FIT.json
python3 -B tools/measure_vulture_mk3_fit.py --union original | cmp - integration/sh039-vulture-mk3/ORIGINAL-ENVELOPE-FIT.json
python3 -B tools/measure_vulture_mk3_fit.py --coverage-gap | cmp - integration/sh039-vulture-mk3/COVERAGE-GAP.json
python3 -B -m unittest discover -s tools -p 'test*vulture_mk3*.py' -v
```

The surface results use the pinned scientific dependencies and Python 3.14.3 with assertions enabled on SteveMBA. The station and fit commands emit canonical UTF-8/LF report bytes under redirected stdout. Raw source and reports use explicit `-text` attributes, and the six pinned tooling files use narrow `text eol=lf` rules. The complete [criteria](../integration/sh039-vulture-mk3/CRITERIA.json) records raw identity, prior refusal, and candidate execution evidence. No additional Mac compiler or game graphics run was started.
