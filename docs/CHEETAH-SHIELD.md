# Cheetah inspection shield

The station Showroom uses SH082’s original 3,508 detailed faces, materials and UVs. The native inspection candidate has 512 faces across the same 29 parts as the archived 988-face native mesh. Each propulsion casing uses four points from each of its two original twelve-point rings, at original indices 1, 4, 7 and 10. This adds four triangles per casing to the historical 504-face candidate. The other 27 candidate parts keep their local vertices, faces, materials and UVs. Both earlier native meshes remain available as source evidence.

The MBA measured 24 original positive-volume overlaps. All 24 survive the repair. In particular, the original upper and lower service-well witness on each casing is inside both component meshes. Exact ray crossings verify these four points against the archived mesh and the repair. The historical candidate had separations of at least 50 mm at both upper contacts and 292 mm at both lower contacts. All six exterior clearance routes pass. The Studio validator also records a solid hit at each of the four original points.

The shield has 388 local groups with 20 mm rounding. Each recessed service well uses four wall sectors and a base. Splitting shared corners at these boundaries preserves every source triangle. All three original corners of each of the 3,508 detailed and 988 archived native triangles occur together in a single convex support. This gives continuous coverage over each full triangle. The conservative distance bounds for the rounded supports are 1,014 mm to the original detailed surface and 1,237 mm to the archived native surface. They include buried faces.

The reduced visual still differs from the original. The full-surface upper bound is 1,248 mm from archived native to candidate and 992 mm in reverse. The corresponding bounds against the detailed mesh are 1,730 mm and 994 mm. The finite authored dry reference is 21,358 kg. The candidate adds stock inspection only. Native station rendering and platform review remain with their owners.

## Reproduce

Use Python 3.11 or later, the pinned geometry requirements and the workspace Hull Tool.

```sh
python3 tools/prepare_cheetah_shield.py --check
python3 -m unittest discover -s tools -p 'test_cheetah_shield.py' -v
python3 tools/measure_cheetah_contacts.py contacts
python3 tools/measure_cheetah_contacts.py fidelity
python3 tools/measure_cheetah_shield.py
```

[Original assets and MIT notice](../assets/fleet/sh082) · [Studio input and measured reports](../integration/sh082-cheetah)
