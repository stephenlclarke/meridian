# Condor source evidence

SH055 preserves the original 4,504-triangle detailed mesh and 244-triangle native
mesh, both in metres. The original detailed bounds measure 31.81 m wide, 10.70 m
tall and 31.57 m long. The finite empty-dry reference is
`finite-intrinsic-empty-condor` at 34,306 kg. The native mesh fits within the
512-triangle cap and retains its original geometry and appearance.

The inspection envelope has 24 local support groups with 20 mm rounding. Nested
surfaces share only their own pod's group. A separate exact vertex witness gives
a strict detailed-to-native lower bound of 1,346 mm. The wings set the largest
conservative stand-off bounds because their convex supports span concave areas
in the artwork. The upper bounds are 6,047 mm against the detailed mesh and
6,180 mm against the native mesh. Across the other groups the corresponding
upper bounds are 1,397 mm and 2,013 mm.

Seven exact swept-triangle proofs give 793–1,429 mm of guarded clearance. They
cover the upper centreline, the space above and below both wings and two outboard
aft passages. Exact point-in-solid checks establish eight overlapping joints in
both original meshes, including the connection of each pod to its boom, each boom
to the crossbar and each wing to the forward hull.

The installed Developer ID–signed Hull Studio produced two identical offline
artwork exports and responses for the 4,504/244 definition. The source verifier
supplies the clearance and joint evidence. The Rust module exposes catalogue
inspection queries. The original runtime catalogue lists Condor as unflyable and
unavailable for normal purchase. The R3 contract records blueprint fabrication
as the intended purchase policy.

[Frozen sources](../integration/sh055-condor/FROZEN-SOURCES.json) ·
[Inspection criteria](../integration/sh055-condor/CRITERIA.json) ·
[Exact contact proof](../integration/sh055-condor/CONTACT-RESULT.json)
