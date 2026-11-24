# Condor source evidence

SH055 preserves the original 4,504-triangle detailed mesh and 244-triangle native
mesh, both in metres. The original detailed bounds measure 31.81 m wide, 10.70 m
tall and 31.57 m long. The finite empty-dry reference is
`finite-intrinsic-empty-condor` at 34,306 kg. The native mesh fits within the
512-triangle cap and retains its original geometry and appearance.

The collision envelope has 26 local support groups with 20 mm rounding. Its
two parts per wing overlap at original cross-sections, with 56 vertices shared
between adjacent groups. All 4,748 original faces, including 1,504 wing faces,
sit within at least one local support. Nested surfaces continue to share their
own pod's group.

Each original wing-wide convex support spans the concave center of the wing.
Exact witnesses establish that the original supports reach beyond 5,083 mm from
the detailed mesh and 5,126 mm from the native mesh. With the wing split, the
conservative stand-off upper bound falls from 6,047 mm to 3,596 mm against the
detailed mesh and from 6,180 mm to 3,596 mm against the native mesh. A separate
original-mesh vertex witness gives a strict detailed-to-native lower bound of
1,346 mm.

Seven exact swept-triangle proofs give 793–1,429 mm of guarded clearance. They
cover the upper centreline, the space above and below both wings and two outboard
aft passages. All seven certified clearances are identical after the collision split. Exact point-in-solid checks establish eight overlapping joints in
both original meshes, including the connection of each pod to its boom, each boom
to the crossbar and each wing to the forward hull.

The installed Developer ID–signed Hull Studio produced two identical offline
artwork exports and responses for the 4,504/244 definition. It preserves its
original 24 artwork groups. The source verifier
supplies the clearance and joint evidence. The Rust module exposes catalogue
inspection queries. The original runtime catalogue lists Condor as unflyable and
unavailable for normal purchase. The R3 contract records blueprint fabrication
as the intended purchase policy.

[Frozen sources](../integration/sh055-condor/FROZEN-SOURCES.json) ·
[Inspection criteria](../integration/sh055-condor/CRITERIA.json) ·
[Collision contact proof](../integration/sh055-condor/COLLISION-CONTACT-RESULT.json) ·
[Collision fit proof](../integration/sh055-condor/COLLISION-FIT.json)
