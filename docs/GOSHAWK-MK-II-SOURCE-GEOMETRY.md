# Goshawk Mk II source geometry

SH009 follows SH008 in original hull order. The source preparation binds the
Goshawk Mk II from original revision `a11fe169`. It keeps the active work on
the preceding hull's integration and engineering decisions.

| Source | Vertices | Triangles | Index-connected components |
| --- | ---: | ---: | ---: |
| Calibrated detailed display | 2,104 | 3,532 | 169 |
| Original native candidate | 568 | 1,012 | 31 |

The detailed file is the immutable staged SH009 resource. The native file
is the original revision's Showroom export, bound through its exact
export-manifest entry and the original design TOML digest named by the
R3 contract. Both sources retain face order, forest-silver material
assignments, texture identities and UVs. The original MIT notice accompanies
the source.

The detailed source already contains the adopted metre calibration. The
native export uses the same exact ratio of 30,000,000/3,186,991 metres per
source unit once, with the authored origin and axes. Both sets span
21.953 by 10.745 by 30.000 m (+Z forward, +Y up). These are measured mesh
extents. The authored R3 width and height are layout estimates. The
1,012-face native candidate exceeds the exact 512-face qualification limit.

The preparation writes ordered little-endian binary64 vertex triples and
unsigned triangle indices. The vertex header contains the ID-specific
magic and detailed/native counts. Native indices in the combined triangle
stream use the detailed vertex count as their offset. `COMPONENTS.json`
records original local face and vertex ranges by index connectivity.
`SOURCE-GEOMETRY.json` binds the exact inputs, calibrated outputs and
qualification status. The geometry is staged for a future SH009-specific
envelope, renderer and runtime review.

```bash
python3 -B tools/prepare_goshawk_mk2_geometry.py --check
```
