# SH001 Mule gameplay envelope

SH001 Mule is the first unimplemented ship in the immutable fleet backlog. Delivery proceeds one hull at a time. The selected later source remains the original 7,236-face display model and 1,772-face native source, with all appearance, component ownership, calibration and engineering records preserved. Its original exact native-union/512-face qualification remains unresolved. The smooth gameplay envelope is a separate approximation authorized by the human shield design direction.

## Surface and tightness

The surface is a union of sixteen rounded convex supports: the spine, six separate connecting supports, six panniers, cockpit, and two engines. Each piece is the convex hull of its assigned authenticated source vertices, plus a spherical offset of exactly 0.02 metres. The support function is `max(dot(source_vertex, direction)) + 0.02 * length(direction)`. This specifies the full actual surface from the packaged point/membership data; it does not require an ellipsoid-fit guess. The boundary of each rounded convex piece is continuously differentiable. Composite intersections may have visible seams, so the renderer must hide surfaces internal to the union.

Directional support expands by exactly two centimetres in every direction, apart from explicit floating-point query guards. This is a bound relative to each piece's source convex support. It is not a claim that concave service wells remain empty or that every envelope point is two centimetres from a detailed triangle. Small service recesses are covered by the approximation. Separate pieces retain the large inter-pannier gaps; the actual source-gap test and renderer inspection remain required. Original component labels, coordinates, fields and notices remain intact in the display asset.

Every detailed and native triangle belongs to one original creator component and one smooth piece. Containment of those vertices in that convex piece proves containment of its entire triangle. Source admission authenticates all 5,344 vertices, all 9,008 triangles and their original membership. The importer only copies bytes and encodes positions/indices; it does not run an envelope, union, contact or containment algorithm.

The existing exact calibration is 40000/3411 metres per source unit. The original origin and local +Z forward/+Y up convention remain. The 62,470 kg engineering budget, centre of mass and full inertia tensor are copied from their selected source record. They are never recomputed from the enclosing surface.

## Contact, docking and shields

Contact uses bounded convex-support iterations. Clearance requires a strict separating support inequality with a conservative numerical guard. Continuous translation is included in the support, and rotation adds an origin-radius angular-displacement bound. Tangency, under-specified angular travel, arithmetic uncertainty and nonconvergence cannot grant clearance. These tests are authored and UNRUN; no runtime performance or native-contact acceptance is claimed.

Port fit uses each actual smooth piece's support against every port edge, including the transformed station axial rotation. It checks the complete outer surface with strict clearance, rather than only source vertices or a large box. Original station triangles remain authoritative for the continuous wall check. Mule launch has its own source-aware port-fit and exterior-support checks; SH025 source identity is not relabelled. Entry retains the existing speed, attitude, roll, plane-crossing and tunnel requirements. Geometric checks grant no arrival receipt or World mutation.

Ray queries distinguish a confirmed inside-witness contact from uncertainty. An uncertain result must occlude or refuse the query; it must not be treated as clear space or as a damage/EMP capability. Current ad5 gameplay already has real paid EMP equipment, source-bound physical contact/occlusion, shield drain, electrical disable, finite NPC custody and save ownership. No duplicate EMP reducer, immunity, retraction or phasing rule is added here. Depleted or absent shield banks hide the shimmer, while the conservative physical envelope remains unchanged.

The mutable actor identity, pose, generation, current equipment and effect capability belong to World. `MuleHullSource` is immutable source geometry; it never admits an actor or grants purchase, crew, role equipment, a blueprint, salvage or a save entitlement. Current gameplay must select it only after the actual per-ID owner/physical/acquisition contracts are connected. The current prototype has not made that activation or increased the fleet count.

## Renderer dependency

`MulePresentation` shares immutable `Arc` surface data with collision. The existing renderer owner must draw that same rounded-support union. Maximum single-surface opacity is 0.018, with fourth-power grazing fade, full close fade through 32 metres and zero by 64 metres. Front/rear strength uses the existing local +Z bank split. Internal piece surfaces must be discarded and overlapping pieces must not accumulate into a bright halo. The detailed original model remains the visible hull; the envelope is a very slight close-up edge shimmer.

G1's normal owned qualification prints each actual piece's source bounds, point count, offset and support-gap parameters. G3 then implements the actual renderer dependency and G1 captures close, far, depleted, contact and rotating-dock views. Authoring source alone does not establish visual acceptance.

## Integration and remaining acceptance

The author branch retains older gameplay source. [The current-gameplay binding](../integration/sh001-mule/CURRENT-GAMEPLAY.json) verifies all 131 core files against qualified `ad5b9f9`. Root adds only new files/assets and the exact two module declarations in [the adaptation patch](../integration/sh001-mule/CURRENT-GAMEPLAY.patch). The patch uses zero context and requires `--unidiff-zero` after exact source validation. Root must preserve current gameplay, paid EMP, persistence and the other active owners.

Nine new core tests cover all source vertices/triangle interiors, directional support/engineering, tangency and corner clearance, continuous translation/rotation, an actual inter-pannier gap, strict rotated port fit, depleted shimmer with unchanged collision, source-aware launch and ray uncertainty. All are UNRUN. No author compiler, Cargo, Clippy, SUT, geometry constructor/probe, GPU or native job runs.

SH001 remains unadmitted. Current World actor selection, real per-ID equipment/crew/paid blueprint acquisition, current-save ownership, existing contact/EMP binding, renderer evidence, native docking/gameplay qualification and protected main integration are still required. Root retains merge ownership. The accepted baseline stays 23/162 ships and 4/6 stations. Do not advance to SH008 or apply blanket fleet admission until Mule's actual one-by-one acceptance is complete.
