# SH015 Raven original hull candidate

Raven is sourced from original frozen L337 revision
`a11fe16924d87d06021816f9037952afcbac08db`, its own resource, R3
contract, native export, creator-component ownership and adopted physical
budget. The [manifest](../assets/fleet/sh015/MANIFEST.json) pins each input.
No geometry, material, or engineering is substituted from a sibling hull.

The detailed original preserves all 2,080 vertices, 3,496 triangles, 166
index-connected components, face winding, material assignments, texture
identities and UVs. The 544-vertex original native candidate has 976
triangles and 28 components. It exceeds its original 512-face exact
qualification limit. Both use the source origin, +Z forward, +Y up and the
adopted 30000/2521 m per source unit (30 m detailed length). Both have
approximately 29.869 × 11.319 × 30 m bounds.

## Original engineering

The frozen physical ledger adopts 57,064 kg dry mass, with COM
(−0.000003503, 0.502720211, −3.068180732) m relative to the source origin.
The full row-major centre-of-mass inertia matrix (kg·m²), including the
signed off-diagonal terms, is retained without substitution in
[Raven's physical record](../assets/fleet/sh015/physical.json). Rust has
the exact adopted fields and independent binary64 tests. Its authority
is a new authored source budget, not native qualification or measured
as-built engineering. Actual material density/thickness, hardware
volumes, loaded mass, crew representation and occupied/paid fit are not
established. The source contract gives a maximum speed of 46 units in
the isolated controlled approach; no ordinary player speed changes.

## Source-local rounded envelope

Seventy independent local convex supports use a nominal 20 mm spherical
offset. The bilateral spar members, pods, shoulders, cockpit, crossmember
and engine elements remain distinct. The original detailed/native wells
retain their separate source-local supports. Each original vertex and all
4,472 triangles lie inside their own declared support by convexity. This
conservatively covers both variants without a whole-hull convex fill.
It deliberately does not claim exact native collision.

| Compared original | Certified exposed lower bound | Full rounded-surface upper bound |
| --- | ---: | ---: |
| Detailed | 864 mm | 900 mm |
| Native candidate | 1,524 mm | 1,530 mm |

The mandatory detailed source vertex 336 is at least 1,508 mm from the
native surface; mandatory native vertex 86 is at least 862 mm from the
detailed surface. These cross-source differences cannot be removed by
convex repartition alone. Both source-local forebody probe paths at
x = ±7.5 m and y = 1–1.3 m have more than 0.740 m guarded clearance
during an 8 m sweep. The per-piece
[surface certificate](../integration/sh015-raven/SURFACE-FIT.json) and
[fit witness](../integration/sh015-raven/FIT-WITNESS.json) retain the exact
bounds and limiting features. The upper bound includes hidden member
surfaces and rounded corners, rather than relying on sampled maxima.

The source fit uses the actual SH028 triangles and aperture. The
tested aligned crossing has more than 43.04 m guarded clearance and the
off-aperture x = 100 m sweep more than 9.01 m, but the latter fails
aperture admission. The forward point at original detailed vertex 96
crosses the strict interior of station triangle 482 while the ship's
origin remains outside the port. Its initial pose has 0.979 m clearance.
The outward-rounded launch Z is 1151.5601806640625 m.

The controlled World reuses the station's swept contact and entry
policy and a stationary original SH025 observer. Authored tests exercise
aligned entry, overspeed, lip contact, off-aperture refusal and observer
contact, plus tick/snapshot identity and exact source speed boundaries.
It provides no commander, ordinary actor or persistence entitlement.
The typed snapshots do not accept either SH008 or SH010 as Raven, and
SH008 retains its separately adopted World engineering publication.

## Showroom and validation

In the development source, open Ship Showroom with 3 at a station and
search for Raven. The presentation sends all 3,496 detailed triangles
using their original rust-ivory palette and shared source textures.
The rounded support shimmer is a separate presentation layer. Compare
and single-ship preview share the existing controls. Selection grants
no exchange or player acquisition.

The authored desktop checks bind the exact visual, support count and
pipeline layout, verify source identity and test three orientations at
640×360 logical in solo and Compare. The separate Mac-owner Metal tests
cover near-surface shimmer, minimum-size actual RGB and mixed-hull cache
orders, including independent direct render identities and full-target
RGB union for Explorer/Raven and Custom/Raven in both pane orders. No
such native or Rust run is claimed by this source candidate.
GPU timing, native resizing, compositor output and ordinary gameplay
admission are also open. No published preview is updated by this packet.

Run the source importer and fit replay with assertions enabled and the
pinned surface packages in `tools/requirements-goshawk-surface-fit.txt`:

```bash
python3 -B tools/prepare_raven.py --check
python3 -B tools/verify_raven_fit.py
python3 -B -m unittest discover -s tools -p 'test_audit_raven_surface.py' -v
```

The exact owned Rust, Metal and source acceptance commands are in
[CRITERIA.json](../integration/sh015-raven/CRITERIA.json).
