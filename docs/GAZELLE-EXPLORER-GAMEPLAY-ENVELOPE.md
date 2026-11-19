# SH019 Gazelle Explorer original hull candidate

Gazelle Explorer is bound to frozen L337 revision
`a11fe16924d87d06021816f9037952afcbac08db` and its own original
resource, current R3 contract, native export and approved Swift
physical calibration. The [manifest](../assets/fleet/sh019/MANIFEST.json)
pins all inputs. No predecessor rendering or sibling hull is used.

Original Gazelle Explorer is explicitly activated in the frozen
legacy source against its authored native LOD in metres. It has
22.6 × 13.8 × 38.29 m bounds, the original unchanged source origin
and +Z forward, +Y up. Both retained meshes have these same bounds.
The detailed original contains 3,008 vertices and 5,900 triangles in
30 index-connected components. The selected native LOD contains
172 vertices and 304 triangles in 11 components, below its original
512-face exact qualification limit. These are source facts, not new
Meridian native execution or ordinary player activation.

## Engineering authority

The approved source calibration is exactly one metre per source unit.
The retained four frozen physical registration tables do not register
SH019, and there is no separate creator-ownership or physical budget
record for this hull in that revision. Dry mass, centre of mass,
inertia, loaded geometry, actual material construction, crew
population and installed equipment remain unknown. There is no
fabricated engineering estimate. The current frozen R3 contract gives
43 units as the inclusive controlled approach speed; it is independent
of ordinary flight and the older archived activation proposal.

## Source-local rounded hull

The original shape retains the flattened pressure body, long ventral
telescope keel, elliptical aft service loop, forward survey structures,
and upper glazing. The rounded collision candidate consists of 37 local
convex supports with nominal 20 mm spherical offset. Its labels are
derived from original connected geometry, not a creator-ownership
record.

The service loop is one connected torus in each original mesh. A
single convex support would fill its opening. Instead, use eight
45-degree source-local sectors, each enclosing the complete original
triangles in its own adjacent major-ring arc. The detailed original
has 64 major-ring rows and 12 points per row; the native LOD has 8 rows
and 4 points per row. The original meshes and indices are not
modified. Only the support stream duplicates the original seam
points in both neighbouring sectors: 96 detailed and 32 native
copies. A separate exact canonical index map proves each support
point comes from its original point and every one of the 6,204
original triangles is enclosed by its declared piece. The associated
3,308 support points do not replace or inflate the native face count.

| Original domain | Certified exposed lower bound | Full rounded-surface upper bound |
| --- | ---: | ---: |
| Detailed | 577 mm | 890 mm |
| Native LOD | 1,262 mm | 1,400 mm |

The detailed source upper bound of each loop sector is 890 mm. Their
native-source upper bounds by sector 0–7 are 921, 921, 921, 921, 1,204,
1,118, 1,118 and 1,204 mm. These bounds include local convex fill and
hidden support surfaces, not just exposed samples. Mandatory detailed
vertex 2966 is at least 1,242 mm from the native mesh; mandatory native
vertex 102 is at least 88 mm from the detailed mesh. The different
original surfaces cannot both be exact under a single rounded union.
[Per-piece certificate](../integration/sh019-gazelle/SURFACE-FIT.json)

Three actual local service-loop corridor probes each clear an
8 m translation: upper clearance 0.6816 m and negative and positive
lower clearances 0.4880 m. The real SH028 aligned sweep has 43.879 m
guarded clearance. The x = 100 m off-aperture route from 220 to
135 m has 3.259 m clearance but is refused entry. The original
foremost detailed vertex 2862 crosses strict station face 482
interior during the independent lip sweep, with the ship origin
outside the port. The initial pose has 0.979 m gap. Launch is
outward-rounded to 1153.1300048828125 m.
[Independent fit witness](../integration/sh019-gazelle/FIT-WITNESS.json)

The controlled World uses the reviewed typed snapshot boundary,
the actual station and a stationary SH025 observer. Authored
regressions cover clear entry, overspeed, lip contact, off-aperture
refusal, observer contact and source/scope/speed refusal. SH008
retains its independent adopted engineering and SH010 its own
controlled source. No commander, ordinary actor or save follows.

## Original Showroom

In the development source, open the station Showroom with 3 and
search for Gazelle Explorer. The detailed ice-cobalt visual retains
its original face winding, all eight material identities, the source
UV data and the three licensed shared maps. The original source
marks 1,216 detailed faces (960 glazing and 256 emitter) and 16
native faces with texture index −1. Legacy appearance resolves
that index to no map. Only SH019's packed visual decoder accepts
its corresponding material-only sentinel `3`; no fourth
texture or false texture substitution is introduced. The previous
SH008 and SH010 visual material outputs retain their original
source bytes and strict decoding behaviour.

The authored CPU and Metal checks cover exact material-only
attribute counts, the production no-map shader, visible source
material RGB in solid front/oblique/upper/aft poses, same-pose
shimmer, minimum 640×360 logical Showroom and direct full-target
RGB identity/cache returns with Explorer and Custom in both pane
orders. Search for Onager does not route Gazelle geometry; SH018
remains a separate original. Ordinary Showroom exchange is refused.

No Rust or native Metal execution is claimed by this source
candidate. Native exact collision, ordinary actor placement, actual
window resize/compositor, GPU timing, and acquisition remain open.
No published preview is changed.

Run the source importer and independent checks with assertions and
the existing pinned scientific dependencies:

```bash
python3 -B tools/prepare_gazelle.py --check
python3 -B tools/verify_gazelle_fit.py
python3 -B -m unittest discover -s tools -p 'test_audit_gazelle_surface.py' -v
```

The joined Sable/Gazelle renderer uses one shared material-only shader but separate authenticated format policies. Its all-face raw RGB proof and both actual pinned station comparison orders are described in [joined material qualification](SABLE-LINER-GAMEPLAY-ENVELOPE.md#joined-material-only-rendering-and-later-finishes). The future PR76 display finish requires its own styled qualification and does not replace raw source evidence.

[Source, owned Rust and Metal criteria](../integration/sh019-gazelle/CRITERIA.json)
