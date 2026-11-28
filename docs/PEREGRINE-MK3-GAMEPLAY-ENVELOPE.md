# Peregrine Mk III source shield

SH023 is the original Peregrine Mk III. The legacy repository's current resource, native archive and station-ring successor agree on the native mesh. The original activation binds that successor and a scale of 250000/19057 metres per source unit. The candidate applies it once to both meshes and preserves their source origin, axes and 40 m span.

| Original mesh | Vertices | Triangles | Index-connected components |
| --- | ---: | ---: | ---: |
| Detailed | 2,072 | 3,484 | 165 |
| Native | 310 | 512 | 27 |

The active native mesh meets the 512-face budget. Meridian currently lists the hull for blueprint fabrication. Runtime mass, hardware and ordinary admission remain open. The original 101,208 kg engineering budget remains source evidence.

## Bow geometry

The candidate uses 69 local convex supports with 20 mm spherical rounding. The two arms, each longitudinal shoulder, aft cross bridge, pressure body and lower split prow keep separate supports. Each of the 36 detailed service wells groups its local rim and recessed struts. Twelve native service wells retain their distinct positions. Both drive assemblies retain separate brackets, casings and luminous recesses.

| Swept control | Travel | Guarded clearance |
| --- | ---: | ---: |
| Axial bow gap | 10 m | 1.961 m |
| Lateral bow gap | 4.7 m | 2.746 m |
| Along either inner shoulder | 8.75 m | 2.090 m each |
| Aligned motion at the original SH028 port | 2 m | 41.904 m |
| Offset approach before lip contact | Static start | 0.978 m |

The solid controls use original detailed source points on each arm and the lower prow. Native arm vertex 17 crosses the strict interior of original station lip face 482 halfway through the offset sweep. Exact interior rays verify the opaque cockpit crossmember's overlap with the glazing in both meshes. The original primary body and that crossmember have at least 360 mm of vertical separation. The support partition preserves that spacing.

## Fit and original visuals

The certified directed exposed-surface fit is 1.129 to 1.133 m against detailed and 2.405 to 2.407 m against native. Both peaks occur at differing original well rims. Mandatory opposite-mesh vertices already require distances greater than 1.111 m and 2.385 m respectively. The sweep controls certify the bow gap and shoulder routes. Traversability inside individual recesses remains unmeasured.

Both cobalt-and-cream `PRG3VIZ1` streams preserve original face order, normals derived from winding, tint, emissive flags and UVs. The detailed and native meshes contain 10,452 and 1,536 packed corners. Original faces use repeating texture maps 0 and 2.

## Joined Showroom and validation

Search for Peregrine Mk III in the development station Showroom. The unique SH023 identity submits its original detailed `visual.bin`, not the native visual or the Mk I asset. Its presentation shimmer uses its own 69 source supports from the detailed and active native successor geometry. Compare initially retains SH025; ordinary Pin Baseline permits either Peregrine order or two copies of the same model. No exchange or commander state change is authorized.

The joined portable fixtures check ordinary selection, both real crossed payloads, minimum 640×360 logical projection and genuine pinned pane identities. The Mac-owner fixtures add same-pose shimmer, independent direct RGB references, exact isolated-pane union, recovery of both original solo views, and independently rotated duplicate-hull panes (renderer-only). These fixtures still require owned execution; no preview, new runtime mass, flight admission or Meridian native collision acceptance is implied.

SteveMBA verified the metrics, fit and contact reports on 19 November 2026 and replayed all three reports byte-for-byte. Both tilted engine brackets use an exact binary64 supporting-facet enumeration after the floating fan guard refused the first one. The fit driver requires paired outward edges in each closed fan.

```bash
python3 -B tools/prepare_peregrine_mk3_geometry.py --check
python3 -B tools/verify_peregrine_mk3_geometry.py
cargo test --locked -p meridian-core peregrine_mk3::tests:: -- --nocapture
```

Rust and renderer qualification await owned execution. See the [joined acceptance criteria](../integration/sh022-sh023-peregrine/CRITERIA.json) and [original source criteria](../integration/sh023-peregrine-mk3/CRITERIA.json).
