# Buffalo Mk II source shield

SH014 is the original Buffalo Mk II transporter. Meridian lists it for blueprint fabrication. The authenticated legacy resource at `a11fe169` carries the lowered bridge, short opaque plinth and original cream-maroon materials. Both corrected meshes already span 110 m nose to tail. The source authoring ratio is 33.212560386473434 m per original unit. Meridian uses the corrected metre coordinates directly.

| Corrected source | Vertices | Triangles | Original components |
| --- | ---: | ---: | ---: |
| Detailed | 2,056 | 3,460 | 163 |
| Native | 280 | 500 | 15 |

The corrected native mesh fits the original 512-face limit. The older unscaled 928-face export remains as provenance. The two corrected meshes supply the shield and visuals.

The candidate uses 57 local convex supports with 20 mm rounded padding. Each cargo drum, local service well, strut and bridge assembly has its own support. The two interdrum passages clear a 26 m continuous translation by at least 2.211 m and 2.205 m under the guarded proof. A control triangle at original detailed drum vertex 0 blocks.

The two meshes share the same eight tapered plinth vertices. Exact source geometry places a 0.5 m cube inside both the plinth and the command deck. The detailed face 2910 centroid and native vertex 166 lie strictly inside the plinth, and their lower cabin faces both use opaque material 2.

The aligned crossing at the original SH028 port has at least 11.847 m of swept triangle clearance. The offset lip route starts with at least 0.978 m of clearance. Corrected native drum vertex 2088 in the combined source stream crosses the strict interior of original station face 482 halfway through the sweep.

## Fit and material evidence

The certified directed exposed-surface maximum is 2.708 to 2.711 m against the detailed mesh and 5.733 to 5.737 m against native. Both peaks occur at differing well rims. Mandatory original vertices already require shield distances greater than 2.689 m from detailed and 5.715 m from native. The candidate keeps both original meshes in the local supports.

The packed detailed and native visuals contain 10,380 and 1,500 original corners respectively. Both preserve face order and winding, tint, emissive flags and repeating UVs. The two meshes use original texture maps 0 and 2. Both packed files use the `BM2LVIZ1` header with 56-byte corner records.

SteveMBA checked the source topology at 07:58:18 UTC, reproduced the encodings and measured fit at 08:01:01 to 08:01:05 UTC, and verified the contact and source-difference proofs at 08:01:21 to 08:01:22 UTC, all on 19 November 2026.

```bash
python3 -B tools/prepare_buffalo_mk2_geometry.py --check
python3 -B tools/verify_buffalo_mk2_geometry.py
cargo test --locked -p meridian-core buffalo_mk2::tests:: -- --nocapture
```

Rust and visual qualification remain pending. Ordinary placement, admission and stock mass are outside this controlled candidate.
