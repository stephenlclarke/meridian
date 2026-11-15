# Source-backed docking geometry component

## Result and scope

Add `crates/meridian-core/src/docking.rs`, export `docking` from `lib.rs` and document its geometry-only contract in the core README. The module provides explicit f64 vectors/column bases/rigid poses, validated caller-supplied convex ports, entrance coordinates/phase/spin gates, aligned and oriented whole-mesh aperture projection, and source-derived native approach radius/distance/point. No dependencies, assets, mutable World fields, docking flags, receipts, station IDs, collision authority, filesystem or clocks are added. Flight/freight/renderer/root Cargo and existing tests remain unchanged. Authoring is based on reviewed Meridian `eb28cd0de054d1ca07cfa56372064a845eee5087`; Root integrates after this source handoff, with no author commit.

The same station +Z roll is applied before the source column orientation. Coordinates remain in the caller's native model/reference frame, with no invented scale or quaternion adapter. The entry gate and entrance-region test remain separate from the real polygon/whole-mesh projection. The point gate uses25/8 offsets and does not replace the124.64×101.6 SH028 aperture. Tunnel depth53.52 is retained data; a transverse projection alone does not prove safe passage through that tunnel.

## Immutable reference packet

Reference commit `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. The inspected machine-local checkout is `/Volumes/SSD/offload/l337-g1-pr335-equipment-compose.UtqLRJ/required-cold-runtime.a11fe169/elite`; it is provenance, not a runtime/build path. No human WIP is used.

| Reference file | SHA-256 | Consumer |
| --- | --- | --- |
| Sources/L337Core/DockingGeometry.swift | 5a14e298ea5ce6f2d7ba75a5eececa0eaf22be67f115fd32eea8add4310fc52a | Entrance-region and point-entry equations/strict bounds/roll rules |
| Sources/L337Core/AutoTargetControl.swift | 216cf0ace14cec4412e0cc1444ea3adb54a71f24195c9fcbd63edccd50bf01fb | Station modelVector/entrance frame, max mesh radius+400 and approach+1000 |
| Sources/L337Core/NativeStationGeometry.swift | 26dfbd4dfde2c893a46ba417f4fe550b38b99284ae122101aab40f15d2b85794 | Port validation, face/index and strict aligned-aperture containment |
| Sources/L337Core/NativeStationRuntime.swift | dfcce4a42715eac848942eef3b697525dd2d8f01d1eea8f1992582caba55f50c | nativeStationHullFits whole-vertex station-local projection; not its admission/sweep/commit machinery |
| Sources/L337Math/Vector3.swift | a64b5e8c2b4c54b36bdce1b8e2031b9193612d784a03ce8aceeb21b4131091c4 | Dot/length/local roll equations |
| Sources/L337Math/Basis3.swift | d900bfb9375ce0f3aa8f3240dc302d618d639e36cd9e1b91e2b8e4cfab6d9123 | Column apply, dot/transpose coordinates and strict1e-4 validation |
| Sources/L337Core/Resources/Ships/SH028.toml | 65d68f37ca2ad8552d35a54550651afa86ff7f60625c401bb79c7250c4167ca9 | Test-only nativeDocking center/axes/aperture/depth |
| Tests/L337CoreTests/NativeMainStationEntranceGateTests.swift | 01010e595957d2c55da0e19d22ee00560c606cb1eb156b277cb79842ccffb960 | Independent expected interior/depth/edge refusals |
| Tests/L337CoreTests/NativeStationGeometryTests.swift | 75e4f384105946a9b692d755bd274b496b95fb3f01ba326fcfc60d7310a93e57 | Source whole-mesh/convex aperture rules and geometry limitations |
| Tests/L337CoreTests/StationNavigationTests.swift | abb29f045d795eee7d4230ca11f24aaf3244fc87ba993226d0a561a08b5d737b | Misaligned0.101 versus aligned0.099 phase and matched0.3 roll/speed144 refusal |
| LICENSE | 033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766 | MIT notice, copyright2026 Stephen Clarke |

The full reference MIT notice is already retained at [assets/legacy/Elite-L337-MIT.txt](assets/legacy/Elite-L337-MIT.txt); the new module records attribution and provenance. This slice imports no mesh, texture or additional notice-bearing resource. Source geometryContext calls SH028's80metres/sourceunit conversion a new authoring estimate and discloses omitted microgeometry/coarse curves. This packet does not upgrade that estimate to measured calibration, station/collision acceptance or commercial asset permission.

## Reference fixtures and boundary contract

SH028 test port: center(0,0,136.32), outward(0,0,1), up(0,1,0), tunnel53.52; counterclockwise aperture[(-62.32,-50.8),(62.32,-50.8),(62.32,50.8),(-62.32,50.8)]. These constants are parsed/pinned reference test inputs, never default station data. Synthetic squares/triangles/poses test algorithms without presenting them as real stations.

| Independent fixture | Expected result |
| --- | --- |
| Reference facing pose, depth2 | Interior false; point-entry gate false |
| Depth1,0.5,0 | Interior false; entry gate true with roll0/speed48 |
| Depth-0.5,-1,-24 | Interior true; entry gate true |
| Depth-25,-89 | Interior true; entry gate false |
| Depth-90 | Interior false; entry gate false |
| Absolute lateral25 or vertical8 | Point entry refuses exact edge contact |
| Absolute entrance-region projection90 | Interior refuses exact boundary |
| Phase±0.099 versus±0.101, matched roll0.3 | Gate accepts versus refuses |
| Outward Z-0.98 versus-0.981 | Facing gate refuses versus accepts |
| Speed144 versus143.99 | Refuses versus accepts with other checks satisfied |
| Spin1, roll1.099 versus1.101 | Matched-rate branch accepts versus refuses |
| Unmatched roll0.149 versus0.15 | Low-roll fallback accepts versus refuses |
| Basis yaw90, position(200,3,4), SH028 center | Entrance(336.32,3,4), outward(1,0,0), player coordinates(4,-3,-336.32), approach10 gives(346.32,3,4) |
| Offset center(2,3,4), local roll90 then diag(-1,1,-1), position(1,2,3) | Entrance(4,4,-1), up(1,0,0), phaseπ/2 |
| Synthetic square halfwidth10/halfheight6, hull halfwidth9/halfheight5 | Margin0 fits; exact margin1 refuses; smaller8/4 hull clears margin1 |
| Triangle(0,0),(6,0),(0,6), vertex(4,2) | Refuses exact hypotenuse contact, x+y=6 |
| SH028 port, hull halfwidth60/halfheight10 | Aligned projection fits; rotating the hull90 degrees puts height60 outside50.8 and refuses |
| Mesh vertices(3,4,0),(0,0,-12) | Clearance412; approach distance1412; facing entrance Z0.5 gives approach Z-1411.5 |

The depth/region fixtures are independently stated by the pinned Swift tests. Orientation fixtures are hand-expanded column transforms, and the triangle/margin fixtures use independent exact half-plane arithmetic. Floating-point coordinate expectations use1e-10 tolerance for these small fixtures; strict gameplay inequalities have no added epsilon. Cross-platform trig/bit identity is not claimed. A long hull can pass the transverse-only test even if longer than tunnel depth; this explicit test prevents mistaking the predicate for a swept-clear route.

## Deliberate input boundary

The source expressions are preserved for admitted finite geometry. Public typed operations refuse non-finite pose/motion and invalid/overflowed geometry, rather than treating invalid data as a source binding or default port. Source port/mesh validation retains coordinate bound1e12,3–64 strictly convex planar aperture points, axis length/dot tolerance1e-8, vertex/face/index requirements and strict distance>margin. Basis validation retains its strict1e-4 tolerance and absence of a determinant condition; it never silently normalizes. The source dot/transpose transform is not replaced by a general inverse. The World adapter must admit appropriate actual rotations/content and supply a station pose with axial roll included for transformed mesh tests.

The reference speed scalar has no lower bound and its low-roll fallback remains even when spin is unmatched. Tests retain those facts. These geometric comparisons do not establish physically attainable motion. Negative/non-finite approach distances and margins refuse at the typed API boundary; zero distance is the entrance point. No missing-source fallback station, aperture, port or clearance radius is manufactured.

## Validation and remaining criteria

Fourteen meaningful Rust tests are authored. Rustfmt parses/formats the new module and the one-line export with child formatting disabled; no unrelated file is formatted. Static source/reference/hash/scope and documentation checks accompany the handoff. These checks do not compile Rust or execute the Rust tests. Cargo check/test/clippy, Swift reference execution, GPU/native/probes, physical station collision, real admission and runtime parity remain UNRUN. G1 owns all compiler/native execution while Swift courier+rescue548/app delivery proceeds.

Root integrates/reviews this one packet when appropriate, then G1 runs the core/workspace check/tests/clippy in the existing lane and closes actual findings. Verify all14 docking tests, original tests and finite-refusal paths. Later P2 integration must use qualified station and effective hull content, actual relative pose/motion/source binding, station triangle/other actor/celestial sweeps and a World-owned transactional entry/launch transition. Save/receipts/paid services/mission authority require their existing independent boundaries. Only an actual player-controlled launch-flight-dock loop with those checks can close docking acceptance; this component alone cannot close P2 or the first playable.

## Links

- [Request and scope](ISSUE-docking-geometry-primitives.md).
- [Core API contract](crates/meridian-core/README.md).
- [Architecture](docs/ARCHITECTURE.md) and [first playable](docs/FIRST-PLAYABLE.md).
