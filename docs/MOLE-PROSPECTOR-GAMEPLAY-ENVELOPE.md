# SH017 Mole Prospector original hull candidate

Mole Prospector is bound to frozen original L337 revision
`a11fe16924d87d06021816f9037952afcbac08db`, its own resource, R3
contract, native export, creator-component ownership and physical
budget. The [manifest](../assets/fleet/sh017/MANIFEST.json) pins each
input. No sibling shape or engineering is substituted.

The detailed mesh retains 6,760 vertices, 12,836 triangles, 179
index-connected components, source winding, materials, texture
identities and UVs. The original native candidate has 616 vertices,
1,100 triangles and 33 components. It exceeds its original 512-face
native exact qualification limit. Both use +Z forward, +Y up, the
unchanged source origin and the adopted exact calibration
920000/56367 m per source unit (46 m detailed length). The detailed
bounds are approximately 31.896 × 20.761 × 46 m.

## Original engineering

The adopted frozen source budget gives a dry mass of 116,421 kg and
centre of mass (−0.000119615, −1.731577746, −2.530785813) m in source-local
coordinates. The full signed row-major inertia tensor about the centre
of mass is retained in the
[original physical record](../assets/fleet/sh017/physical.json).
Rust contains the exact values and independent binary64 tests. The
original R3 contract specifies an inclusive 29-unit controlled approach
speed. It is not a change to the ordinary player.

This is an authored source budget, not native qualification or an
as-built measurement. Actual material densities, sheet thickness,
hardware volumes, loaded mass, occupied crew and fitted paid equipment
are unestablished. The original role estimate does not grant crew,
mining kit, scoop, survey array or any resource production rights.

## Source-local rounded union

The candidate uses 83 local convex supports with nominal 20 mm
spherical rounding. It keeps the two source boring heads, structural
supports, rectangular cradle lateral and transverse members, markers,
pods, cockpit and engine structures separate. All eight detailed
boring-head collars have independent axial and side-local supports.
The native candidate omits those eight detailed components. Detailed
and native pressure-well domains stay separate. Each original vertex
and all 13,936 triangles are enclosed by their own declared supports.

An annular collar is enclosed locally by a convex disk. This can fill
space inside the collar even where the boring head does not cover it.
No exact annular fit or full concavity is claimed. The separate piece
bounds include these hidden support surfaces; they give a conservative
upper certificate for the complete rounded union, not a measurement
of the exposed maximum.

| Compared original | Certified exposed lower bound | Full rounded-surface upper bound |
| --- | ---: | ---: |
| Detailed | 1,273 mm | 1,998 mm |
| Native candidate | 2,021 mm | 2,130 mm |

Mandatory detailed source vertex 337 is at least 2,004 mm from the
native candidate. Mandatory native vertex 91 is at least 723 mm from
the detailed surface. These source differences cannot be eliminated
by convex repartition alone. The detailed upper bound is controlled
by the final collar on either side; the nominal rounding is not the
entire discrepancy.

The eight collar disk upper bounds, by axial order 0–3, are:

| Boring-head side | Detailed upper bounds (mm) | Native candidate upper bounds (mm) |
| --- | --- | --- |
| Negative X | 1,485 / 1,479 / 1,500 / 1,998 | 2,047 / 2,024 / 2,024 / 2,048 |
| Positive X | 1,493 / 1,479 / 1,500 / 1,998 | 2,047 / 2,024 / 2,024 / 2,048 |

[Per-piece surface evidence](../integration/sh017-mole-prospector/SURFACE-FIT.json)

The independent fit witness sweeps a small triangle through the
central tool cradle between the two boring heads. It retains at least
2.7269 m of guarded separating-plane clearance over 8 m of translation.
The real SH028 aligned crossing retains 40.0070 m of clearance;
the x = 100 m off-aperture crossing is physically clear but refused
admission. The entire off-aperture approach from 220 to 135 m has
0.6967 m minimum guarded clearance. Original forebody vertex 1776
crosses the strict interior of real station triangle 482 with the
ship centre still outside the port. The initial pose has 0.979 m
clearance and the outward-rounded launch Z is 1160.057861328125 m.
[Fit witness](../integration/sh017-mole-prospector/FIT-WITNESS.json)

The controlled World uses existing station swept-contact and entry
policy with the original stationary SH025 observer. Authored tests
cover aligned entry, overspeed, lip contact, off-aperture refusal and
observer contact, and verify source-scoped snapshots and the exact
speed boundary. It creates no commander, ordinary population or save.
The joined typed World keeps Explorer's stock engineering publication
and Custom's own source and snapshot paths distinct. Neither of their
snapshots authorizes the Prospector.

## Showroom and validation

In the development source, open Ship Showroom with 3 and search for
Mole Prospector. Its own slate-orange source visual submits all 12,836
triangles with original textures. The rounded shimmer is a separate
presentation layer. Compare and solo reuse the ordinary Showroom
controls. Selection does not grant exchange or player acquisition.

The desktop source tests bind the exact visual, support count and
pipeline layout; verify three orientations at the minimum 640×360
logical viewport; and reject sibling visual identity. The Mac owner's
separate tests cover same-pose shimmer, minimum-size RGB and both
Explorer/Prospector and Custom/Prospector cache orders with direct
render identity and full-target pixel union. Rust and native execution
are pending.
Native resizing, compositor output, GPU timing, ordinary placement and
exact native collision remain open. No published preview changes.

The joined SH016 Mole Miner and SH017 Mole Prospector remain separate
catalogue identities. Their frozen R3 controlled speed requirements
are respectively 25 and 29 units, and both retain the original 512-face
native exact limit. The source and Showroom regressions reject a
relabelled sibling, crossed visual keys and selection of Prospector
when searching for Miner. SH016 runtime admission is outside this
packet; no SH016 source asset or separate implementation is imported.

Run the importer and fit replay with assertions enabled and the existing
`tools/requirements-goshawk-surface-fit.txt` scientific dependencies:

```bash
python3 -B tools/prepare_mole_prospector.py --check
python3 -B tools/verify_mole_prospector_fit.py
python3 -B -m unittest discover -s tools -p 'test_audit_mole_prospector_surface.py' -v
```

The exact source, Rust and Metal acceptance commands are in
[CRITERIA.json](../integration/sh017-mole-prospector/CRITERIA.json).
