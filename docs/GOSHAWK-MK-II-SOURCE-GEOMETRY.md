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

## Rounded shield and station fixtures

The Mk II uses 73 local convex supports with a 20 mm spherical offset.
Corresponding major assemblies share supports. Each well stays local, and
the two ventral brackets and two runners use separate supports. Every
original triangle stays in one complete support. Pairing each bracket with
its runner would fill a corner now clear by at least 0.614 m on either side.
This is an SH009-specific choice.

A 10 m translation in the central forebody channel has at least 1.462 m
of guarded clearance. The control uses a triangle touching original
detailed drive-arm vertex 8. The positive SH028 control sweeps an aligned
Mk II across the source port plane. The offset negative control starts
clear and carries detailed rail vertex 6 through the strict interior of
original SH028 face 482. These assertions exercise authenticated hull and
station geometry through the same Rust swept-contact implementation.

The independent metre-based surface audit on SteveMBA used the original
triangles and all 73 rounded supports. The directed maximum lies within
0.592–0.681 m of the detailed mesh and 1.061–1.140 m of the native candidate.
The largest witnessed gaps are at body wells. The upper bound covers all
member surfaces, including the cockpit crossmember where exposure sampling
was unresolved. The native candidate still requires exact native
qualification under the 512-face cap.

The exact fixture and fit results are retained with the SH009 evidence.
Run the source check and targeted Rust tests as follows.

```bash
python3 -B tools/verify_goshawk_mk2_fixture_source.py
cargo test --locked -p meridian-core goshawk_mk2::tests:: -- --nocapture
```

## Adopted stock engineering

SH009's adopted dry mass is **33,072 kg**, including both drive liners in
the drive assemblies. The empty 140 kg equipment interface is split into
two 70 kg point masses at the original left and right longitudinal support
centres. The authenticated SH009 hull publishes the matched centre of mass,
full XYZ tensor about that centre, and the two mass locations for use in
certification. Utility volume, equipment access, occupied fittings, and
ordinary insertion await separate qualification.
