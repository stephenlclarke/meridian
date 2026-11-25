# Hobby Fighter source evidence

SH054 preserves the original 4,620-triangle detailed mesh and 192-triangle native mesh,
both in metres. The source dimensions are 16.00 m wide, 9.78 m tall and 24.69 m long.
The finite empty-dry reference is `finite-intrinsic-empty-hobby` at 19,880 kg.
The native mesh meets the 512-triangle cap with its original faces, vertices and appearance.

The inspection envelope has 24 local support groups. Each stack has two independent
pod groups. Each pod's nested surfaces share the pod's own group. The original wings,
canopy, ribs, and service details retain their local supports. Rounded supports have
20 mm of padding. Their measured maximum upper stand-off from the original meshes is
1,437 mm against the detailed mesh and 2,441 mm against the native mesh. These are
conservative support-envelope bounds. A separate exact source vertex witness gives a
strict detailed-to-native lower bound of 1,845 mm near the omitted detail geometry.

Seven exact swept-triangle proofs give guarded clearance of 558–1,094 mm around the
wing passages, ventral route, and aft passages between the stacked pods. Both wider
aft passages independently prove 1,094 mm of guarded clearance. Exact point-in-solid
checks prove that both wings and the canopy overlap the forward hull in both original
meshes. The source contact results and stand-off bounds are linked from the inspection
criteria.

The installed Developer ID–signed Hull Studio produced two identical offline artwork
exports and responses for the 4,620/192 definition. The source verifier supplies the
numeric clearance and joint evidence. The Rust module exposes catalogue inspection
queries and has no flight admission path. The original catalogue lists the hull as
unflyable and unavailable for normal purchase, while the R3 contract records a planned
blueprint-fabrication access policy.

[Frozen sources](../integration/sh054-hobby-fighter/FROZEN-SOURCES.json) ·
[Inspection criteria](../integration/sh054-hobby-fighter/CRITERIA.json) ·
[Exact contact proof](../integration/sh054-hobby-fighter/CONTACT-RESULT.json)
