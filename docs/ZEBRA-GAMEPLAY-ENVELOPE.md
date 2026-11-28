# SH021 Zebra original hull candidate

Zebra is bound to frozen original L337 revision
`a11fe16924d87d06021816f9037952afcbac08db`, its own resource,
current R3 contract, native export, creator-component ownership and
physical budget. The [manifest](../assets/fleet/sh021/MANIFEST.json)
pins the exact originals and associated licence. No sibling shape
or appearance is substituted.

The preserved detailed source has 1,488 vertices, 2,528 triangles
and 112 connected components. Its original native candidate has
412 vertices, 744 triangles and 20 components. That candidate exceeds
the original 512-triangle limit. A separate frozen 20-part native
reduction package is retained as unexecuted source evidence
([blueprint](../assets/fleet/sh021/native-candidate-blueprint.json)).
The packaged original resource remains unchanged. This port neither
executes nor selects the alternative and grants no cap waiver or
native qualification.

## Original engineering

The exact calibrated frame uses 40/3 m per source unit, unchanged
origin, +Z forward and +Y up. The detailed measured bounds are
approximately 23.336 × 17.662 × 40 m; the original R3 target estimate
of 28 × 11 × 40 m is not used to rescale or infer width or height.
The source scale is the adopted 40 m detailed length.

The adopted original dry budget gives 60,108 kg, centre of mass
(−0.165990829, −1.584649843, −3.723334976) m and a full signed
centre-of-mass inertia matrix. The exact data is retained in the
[original physical record](../assets/fleet/sh021/physical.json).
Independent Rust and source tests bind the actual binary64 values.
The current R3 contract gives an inclusive 34-unit controlled
approach speed. These source values are not measured as-built or
loaded properties. Actual materials, hardware volume, occupied crew
and installed paid role equipment remain unresolved.

## Simple close-fit rounded shield

The original outline comprises two deep freight keels, two
longitudinally distinct cross-bridges, an upper cabin and twin
rear drives. The collision candidate uses 48 local convex supports
with a nominal 20 mm spherical offset. Both keels, both bridges,
cockpit, engine parts and corresponding local well regions remain
separate; detailed and native wells retain separate local ownership.
Each original vertex and all 3,272 triangles are enclosed by their
own declared local piece. There is no full-hull convex fill.

A small independently certified physical triangle passes through
the original centre trench for the full 40 m translation. Its
minimum guarded separation across every piece is 1.37316 m,
including the actual bridges and engine supports. The authored Rust
case uses this same path and actual source contact controls.

| Compared original | Certified exposed lower bound | Full rounded-surface upper bound |
| --- | ---: | ---: |
| Detailed | 1,060 mm | 1,063 mm |
| Native candidate | 1,515 mm | 1,597 mm |

Mandatory detailed vertex 904 is at least 1,498 mm from the original
native candidate; native vertex 182 is at least 1,041 mm from the
detailed surface. These source differences cannot be removed solely
by changing the convex partition. Both cross-bridges have full
rounded-piece upper bounds of 581 mm against the detailed source
and 1,142 mm against the native candidate. The bounds include hidden
local support surfaces. They do not claim exact recessed-well
geometry or native acceptance.
[Per-piece surface certificate](../integration/sh021-zebra/SURFACE-FIT.json)

The actual SH028 aligned crossing has 39.659 m guarded clearance.
The x = 100 m off-aperture sweep from Z = 220 m to 135 m remains
physically clear by 2.979 m but fails aperture admission. Original
detailed vertex 96 passes through the strict interior of station
face 482 while the ship origin is still outside the port. Its
initial pose has 0.979 m clearance; launch outward-rounds to
1156.340087890625 m.
[Independent fit witness](../integration/sh021-zebra/FIT-WITNESS.json)

The controlled World uses the reviewed typed snapshot and station
swept-contact boundary. Its authored cases cover aligned entry,
overspeed, lip contact, off-aperture refusal, observer contact and
exact source speed and snapshot scope. SH008 stock engineering and
SH010's separate source remain intact. It creates no commander,
ordinary placement or save.

## Original Showroom and acceptance

In the development source, open Ship Showroom with 3 and search for
Zebra. The original ice-blue rigid hull submits all 2,528 detailed
triangles using their original material assignments, winding and
licensed shared textures and UVs. The close rounded shield is a
separate subtle presentation layer. Compare and solo reuse the
existing station controls; selecting Zebra does not grant exchange.
SH020 Crow Prototype is a separate original; its selection cannot
route Zebra geometry, and its implementation remains separately owned.

On the earlier Zebra revision `aeefaeb1`, the Mac passed the controlled
approach and Showroom cases and all three Metal checks. These covered the
640×360 minimum view, shield shimmer and direct RGB comparison in both
pane orders with Explorer and Custom. The private Mac build displayed
both freight keels and the crosspieces. WSL2 and native Windows also
passed the controlled approach and Showroom cases on that revision.
See the [Zebra pull request](https://github.com/stephenlclarke/meridian/pull/83)
for results by revision. Ordinary acquisition, qualification against
the 512-face limit, GPU timing and live window resizing remain open.

Run the importer and independent source checks with assertions and
the existing pinned scientific dependencies:

```bash
python3 -B tools/prepare_zebra.py --check
python3 -B tools/verify_zebra_fit.py
python3 -B -m unittest discover -s tools -p 'test_audit_zebra_surface.py' -v
```

[Source, owned Rust and Metal criteria](../integration/sh021-zebra/CRITERIA.json)
