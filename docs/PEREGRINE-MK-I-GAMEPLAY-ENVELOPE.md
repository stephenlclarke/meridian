# SH022 Peregrine Mk I original hull candidate

Peregrine Mk I is bound to frozen original L337 revision
`a11fe16924d87d06021816f9037952afcbac08db`, its own resource,
current R3 contract, native export, creator-component ownership
record and physical budget. The
[manifest](../assets/fleet/sh022/MANIFEST.json) pins the inputs.
The source is distinct from Goshawk Explorer even though its topology
counts happen to match, and distinct from every Peregrine peer.

The detailed original has 2,072 vertices, 3,484 triangles and
165 connected components. Its packaged original native source
has 536 vertices, 964 triangles and 27 connected components, above
its original 512-face exact limit.

## Unqualified native alternative

The frozen peer reduction blueprint initially describes an
unexecuted source generator. A later frozen qualification-input
bridge identifies an actual cached SH022 input, but its status
explicitly excludes fidelity, native, physical and runtime
qualification. The cached input was identified as
`actual-run/candidates/SH022.json`, 68,833 bytes, with the SHA
recorded in the
[native candidate status receipt](../assets/fleet/sh022/NATIVE-CANDIDATE-STATUS.json).
That receipt is an exact SH022-only projection of the two pinned
legacy parent records. The actual candidate geometry bytes and
face count are not present in this frozen Git tree. No continuous
fidelity metric, tolerance or adoption is issued. Neither the
reduction proposal nor the later cached input is run, selected,
or qualified here. The original packaged 964-face resource and
face-limit refusal remain explicit.

## Original engineering

The adopted exact scale is 800/59 m per source unit with the
original unchanged origin, +Z forward and +Y up. The measured
detailed bounds are approximately 32.442 × 15.546 × 40 m. The R3
28 × 11 × 40 m target estimate does not authorize an additional
width or height rescale.

The adopted dry source budget is 103,972 kg with centre of mass
(−0.137297372, −0.969151185, −4.024231080) m and the full signed
centre-of-mass inertia tensor. Exact fields are retained in
[physical.json](../assets/fleet/sh022/physical.json).
Independent source and Rust tests bind the actual binary64 values.
The current frozen R3 contract gives an inclusive 37-unit
controlled approach speed. The source budget is not an as-built
measurement. Actual material construction, hardware volumes,
loaded mass, occupied crew, paid equipment and role entitlement
remain unresolved.

## Close local rounded hull

The original silhouette retains the rounded central command hull,
two hooked longitudinal shoulders, stepped lower cargo deck and
rear drives. Its collision candidate uses 69 source-local convex
supports with a nominal 20 mm spherical offset. The two actual fork
longitudinal members and aft cross-bridge remain separate, as do
the pod bodies, pressure body, lower prow, glazing, drive assemblies
and local wells. Detailed and native well domains are kept separate.
Each original vertex and all 4,448 triangles are enclosed by their
declared support. There is no whole-hull convex fill.

An independent small-triangle probe sweeps the actual hooked
forward bow gap over 10 m with 2.7188 m minimum guarded separation
across every piece. This is a forward bow control, not proof of an
open aft route through the pressure deck.

| Compared original | Certified exposed lower bound | Full rounded-surface upper bound |
| --- | ---: | ---: |
| Detailed | 1,156 mm | 1,160 mm |
| Native candidate | 1,708 mm | 1,736 mm |

Mandatory detailed source vertex 336 is at least 1,692 mm from the
native candidate. Mandatory native vertex 74 is at least 1,138 mm
from the detailed surface. Any union enclosing both must retain
those source differences. Both longitudinal fork members have full
rounded-piece upper bounds of 572 mm against detailed and 1,124 mm
against native. The aft bridge's corresponding bounds are 759 mm
and 1,497 mm. These include local convex fill and hidden supports;
they are not native exact-collision acceptance.
[Per-piece certificate](../integration/sh022-peregrine-mk-i/SURFACE-FIT.json)

The original SH028 aligned sweep has 41.606 m guarded clearance.
The x = 100 m off-aperture route from Z = 220 m to 135 m remains
physically clear by 3.2366 m but fails admission. Original detailed
vertex 96 crosses the strict interior of station triangle 482
while the ship origin is still outside the port. Its initial
pose has 0.979 m clearance; outward-rounded launch Z is
1156.59765625 m.
[Independent fit witness](../integration/sh022-peregrine-mk-i/FIT-WITNESS.json)

The controlled World uses the reviewed typed station boundary
and stationary original SH025 observer. Authored cases test aligned
entry, overspeed, lip contact, off-aperture refusal and observer
contact, plus exact R3 speed and source-scoped snapshots. Existing
SH008 engineering publication and SH010 validation scope remain
distinct. No commander, ordinary population or save is created.

## Showroom and validation

In the development source, open the station Showroom with 3 and
search for Peregrine Mk I. The original cobalt-cream visual submits
all 3,484 detailed triangles with original materials, winding,
licensed shared textures and UVs. The close rounded shimmer is a
separate presentation layer. Compare retains SH025 as the new
commander's baseline. The joined development source also registers
Mk III as its own SH023 preview. Search switches between real hull
identities and ordinary Pin Baseline compares them in either order.
Their actual visual and support payloads refuse crossed inputs;
the two source assets are not interchangeable. Exchange is refused.

The authored CPU and Mac-owner Metal tests bind exact source visual
and shader, minimum 640×360 logical projection, same-pose shimmer
and full-target direct RGB/cache identity with Explorer and Custom
in both pane orders. Joined evidence adds both original Peregrines
in both orders and duplicate same-hull panes with different poses
and independent reference passes. Rust, Metal and exact native collision execution
remain pending, as do ordinary actor placement, acquisition, paid
fittings and actual window resizing/compositor or GPU timing. No
published preview changes.

Run the source importer and independent checks with assertions and
the existing pinned scientific dependencies:

```bash
python3 -B tools/prepare_peregrine_mk_i.py --check
python3 -B tools/verify_peregrine_mk_i_fit.py
python3 -B -m unittest discover -s tools -p 'test_audit_peregrine_mk_i_surface.py' -v
```

[Source, owned Rust and Metal criteria](../integration/sh022-peregrine-mk-i/CRITERIA.json)
· [Joined sibling criteria](../integration/sh022-sh023-peregrine/CRITERIA.json)
