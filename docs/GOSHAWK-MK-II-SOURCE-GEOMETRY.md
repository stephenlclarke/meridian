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

## Detailed Showroom visual

The SH009 display asset retains all 3,532 calibrated detailed faces in their original order, with 10,596 corner records. Each record uses the SH008 Showroom layout of 56 bytes: binary32 position, flat normal from the ordered face, original material RGBA, corner UV, texture index and emissive bit. The SH009-specific `GOS2VIZ1` header and runtime FNV check prevent a changed or wrong-hull visual from entering the decoder. The native candidate is excluded from display record generation.

The original three-map texture table is retained without reordering or copying the shared image bytes. The Mk II uses 1,356 patrol diffuse faces and 2,176 service detail faces; its declared strike diffuse slot is unused. The model has 184 emissive faces. The three original MIT maps remain in `assets/fleet/sh001`. `assets/fleet/sh009/VISUAL-PROVENANCE.json` binds the legacy revision, staged geometry, output and exact shared texture identities.

The existing Showroom visual loader can decode the MK2 geometry key `assets/fleet/sh009/visual.bin`. The SH008 pass continues to use its own original asset. Actual SH009 selection, the registered pass and draw count, rounded shield, World placement and native visual checks await the joined implementation. The source loader alone grants no gameplay authority.

```bash
python3 -B tools/encode_goshawk_mk2_visual.py --check
python3 -B -m unittest discover -s tools -p 'test_encode_goshawk*visual.py' -v
```

The focused Python parity and mutation tests inspect all ordered records against the original TOML and check that source, winding, texture, geometry and packed-output mutations are refused. The Rust loader regressions are authored for designated compiler execution. No native SH009 Showroom observation follows from the source checks.
