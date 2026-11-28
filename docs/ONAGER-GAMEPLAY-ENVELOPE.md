# Onager source shield

SH018 is the original Onager trader. Meridian lists it for blueprint fabrication. The legacy activation and its approved native export identify the source geometry in authored metres, with a scale of 1 m per source unit. Both original meshes span 31.09 m and keep their source axes and origin.

| Original mesh | Vertices | Triangles | Index-connected components |
| --- | ---: | ---: | ---: |
| Detailed | 1,992 | 3,864 | 30 |
| Native | 140 | 236 | 11 |

The native mesh matches the archived approved model and fits the original 512-face limit. Its top is at 8.06 m. The detailed canopy crest reaches 8.16 m.

## Openings and hatch

The candidate uses 26 local convex supports with 20 mm rounded padding. The four cargo pods, central chassis, lower rails, cockpit shell and glazing keep separate supports. Each drive groups its detailed casing, recess and emitter around the corresponding closed native assembly. The pipe runs and their mounts and clamps have individual supports.

| Swept control | Travel | Guarded clearance |
| --- | ---: | ---: |
| Port and starboard pod gaps | 4.7 m | 0.628 m each |
| Underframe between the original rails | 20 m | 0.209 m |
| Under each protected pipe run | 2.3 m | 0.138 m each |
| Outside the original C03 hatch | 2 m | 4 mm |

Both source meshes identify original C03 hatch faces 6 and 7 on the chassis plane at x = 6.5 m. The support has 20 mm nominal stand-off there. The inner control lies 19 mm from the source plane, inside the rounded support. The outer strip lies 26 mm from the plane. Two solid-pod controls use original detailed vertices 234 and 828. Exact interior rays verify overlap between each original opaque cockpit shell and its glazing.

At the original SH028 station, the aligned sweep has 42.618 m of guarded clearance. The offset route begins 0.979 m clear. Original native nose vertex 22 crosses strict interior of station lip face 482 halfway through that sweep.

## Fit and material evidence

The certified directed exposed-surface fit is 268 to 276 mm against detailed and 1.178 to 1.184 m against native. The detailed peak is at an inset in the opaque cabin body. Its upper bound is 276 mm, and the four individual pod bounds are at most 231 mm against detailed. The native peak occurs at the detailed canopy crest. One required original crest vertex already lies more than 1.162 m from native.

Both ochre-and-steel visuals preserve original face order, normals derived from winding, tint, emissive flags and UVs. The detailed and native `ONAGVIZ1` streams contain 11,592 and 708 corners. All three original shared texture maps are used. The original 1,216 detailed and 16 native unmapped faces carry the existing 32-bit unmapped selector `0xffffffff`.

SteveMBA reproduced the encodings and ran the source topology, fit, contact and original-to-original checks on 19 November 2026. The criteria pin the audit drivers, numerical helper, source inputs and reports.

```bash
python3 -B tools/prepare_onager_geometry.py --check
python3 -B tools/verify_onager_geometry.py
cargo test --locked -p meridian-core onager::tests:: -- --nocapture
```

All four Rust cases passed on Mac, WSL2 and native Windows at `6083f3c3`. Meridian renderer routing, ordinary placement, admission and stock mass remain open.
