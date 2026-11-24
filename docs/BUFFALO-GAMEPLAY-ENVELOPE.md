# Buffalo source shield

SH013 is the original Buffalo drum freighter. Meridian's existing
catalogue identifies the hull; this work prepares its source shield and
visual. The authenticated legacy resource at `a11fe169` carries the
latest short opaque plinth, lowered bridge and cream-maroon material
assignments. Both of its corrected meshes already use metres and span
76 m nose to tail. The source authoring ratio was 23.75 m per original
unit. No additional scale enters the Meridian encoding.

| Corrected source | Vertices | Triangles | Original components |
| --- | ---: | ---: | ---: |
| Detailed | 2,056 | 3,460 | 163 |
| Native | 280 | 500 | 15 |

The corrected native mesh is within the original 512-face limit. The
older unscaled 928-face export is kept as provenance. The shield and
visual derive from the corrected resource.

The candidate contains both meshes in 57 local convex supports with a
20 mm rounded offset. The cargo drums, local service wells, struts and
bridge assemblies keep their own supports. Both original interdrum
passages clear an 18 m continuous translation by at least 1.652 and
1.655 m under the guarded proof. The solid drum control blocks a small
triangle placed at original detailed vertex 0.

The tapered opaque plinth is 4 m high. Exact source geometry encloses a
0.5 m cube in the joint between the plinth and the command deck. The
detailed opaque cabin face 2910 and a point of the native opaque cabin
face also lie strictly inside the tapered plinth. Both models carry
the same eight plinth vertices.

An aligned motion at the original SH028 port has at least 26.949 m of
swept triangle clearance. The offset lip route starts with 0.979 m of
clearance, and original native drum vertex 2088 in the combined source
stream crosses strict interior of SH028 face 482 halfway through.

## Fit and material evidence

The certified directed exposed surface maximum lies within 1.956–1.959 m
against detailed and 4.097–4.101 m against native. Both peaks are at
different source well rims. The corresponding mandatory native rim
vertex is over 1.937 m from the detailed triangles. A mandatory detailed
rim vertex is over 4.079 m from native. Maintaining both sources obliges
the shield to contain these points. The recommended controlled candidate
keeps the local supports and both original meshes.

The packed visual uses 10,380 original corners, face order, tint,
emissive flags and repeating UVs. The existing type-0 and type-2
original texture maps match the source identities. No renderer policy
change is needed for absent textures. SH013 Showroom routing and native
visual qualification remain separate.

SteveMBA ran source topology at 07:42:14 UTC, the final partition and
fit at 07:46:14–07:46:18 UTC, and contact and source-difference proofs at
07:48:39–07:48:40 UTC on 19 November 2026. The fit audit independently
enumerated the exact supporting facets of the near-coplanar right
thruster bracket. The retained source evidence covers both bound types.

```bash
python3 -B tools/prepare_buffalo_geometry.py --check
python3 -B tools/verify_buffalo_geometry.py
cargo test --locked -p meridian-core buffalo::tests:: -- --nocapture
```

The three Rust collision cases await owned execution. Stock mass,
ordinary placement and admission require separate source and validation.
