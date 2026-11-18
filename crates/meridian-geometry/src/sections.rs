//! One-sided closed sections, scoped to one original facet's outward plane.
use super::{Bounds, ComponentBounds, FacetBoundsIndex, log_cost};
use crate::exact::{add, orient, point, project, scale, sub};
use crate::{Budget, Facet, Failure, FailureKind, Point2, Segment, integer};
use num_traits::{Signed, Zero};
use std::collections::BTreeMap;

pub(super) struct Frame {
    owners: BTreeMap<u32, Vec<Segment>>,
    members: usize,
}

impl Bounds {
    // Support interval of n.x-offset. The positive endpoint is strict because
    // the query plane is n.x=offset+epsilon, epsilon>0, in the limiting sense.
    #[cfg(test)]
    fn meets_outward_plane(&self, face: &Facet, b: &mut Budget) -> Result<bool, Failure> {
        let mut low = -face.offset();
        let mut high = low.clone();
        for k in 0..3 {
            b.step(1)?;
            let n = &face.normal()[k];
            let (a, c) = if n.is_negative() {
                (&self.upper[k], &self.lower[k])
            } else {
                (&self.lower[k], &self.upper[k])
            };
            low += n * a;
            high += n * c;
        }
        b.step(1)?;
        Ok(low <= integer(0) && high > integer(0))
    }
}

impl FacetBoundsIndex {
    #[cfg(test)]
    fn outward_candidates(
        &self,
        face: &Facet,
        owners: &BTreeMap<u32, Vec<Segment>>,
        originals: &[Facet],
        b: &mut Budget,
    ) -> Result<Vec<usize>, Failure> {
        // Caller admits N returned ordinals, in addition to this traversal.
        b.with_retained(Self::stack_members(self.facets.len()), |b| {
            let mut pending = vec![self.root];
            let mut candidates = Vec::new();
            while let Some(index) = pending.pop() {
                b.step(1)?;
                let node = &self.nodes[index];
                if !node.bounds.meets_outward_plane(face, b)? {
                    continue;
                }
                if let Some([left, right]) = node.children {
                    pending.push(right);
                    pending.push(left);
                } else {
                    for (offset, bounds) in self.facets[node.start..node.end].iter().enumerate() {
                        b.step(1)?;
                        let ordinal = node.start + offset;
                        log_cost(owners.len(), b)?;
                        if owners.contains_key(&originals[ordinal].id().component)
                            && bounds.meets_outward_plane(face, b)?
                        {
                            candidates.push(ordinal);
                        }
                    }
                }
            }
            Ok(candidates)
        })
    }
}

// A zero-distance vertex is below the outward shifted plane. Taking the limit
// of each crossing retains coincident edges WITH multiplicity, including a
// point when both crossing endpoints converge to one source vertex.
#[cfg(test)]
fn limiting_segment(
    face: &Facet,
    other: &Facet,
    b: &mut Budget,
) -> Result<Option<Segment>, Failure> {
    b.guard(64)?;
    let mut distances = Vec::with_capacity(3);
    for vertex in other.vertices() {
        b.step(1)?;
        distances.push(face.distance(&vertex.point));
    }
    let mut hits = Vec::with_capacity(2);
    for i in 0..3 {
        b.step(1)?;
        let j = (i + 1) % 3;
        if distances[i].is_positive() == distances[j].is_positive() {
            continue;
        }
        let t = &distances[i] / (&distances[i] - &distances[j]);
        let hit = add(
            &other.vertices()[i].point,
            &scale(
                &sub(&other.vertices()[j].point, &other.vertices()[i].point),
                &t,
            ),
        );
        point(&hit, b)?;
        hits.push(project(&hit, face.axis()));
    }
    match hits.as_slice() {
        [] => Ok(None),
        [a, c] => Ok(Some([a.clone(), c.clone()])),
        _ => Err(b.fail(
            FailureKind::UncertifiedContact,
            "one-sided triangle has incomplete section",
        )),
    }
}

impl Frame {
    #[cfg(test)]
    pub(super) fn build(
        face: &Facet,
        face_bounds: &Bounds,
        originals: &[Facet],
        owners: &BTreeMap<u32, ComponentBounds>,
        index: &FacetBoundsIndex,
        b: &mut Budget,
    ) -> Result<Self, Failure> {
        let entry = b.retained_memberships();
        let result = Self::build_admitted(face, face_bounds, originals, owners, index, b);
        if result.is_err() {
            b.release(b.retained_memberships() - entry);
        }
        result
    }

    #[cfg(test)]
    fn build_admitted(
        face: &Facet,
        face_bounds: &Bounds,
        originals: &[Facet],
        owners: &BTreeMap<u32, ComponentBounds>,
        index: &FacetBoundsIndex,
        b: &mut Budget,
    ) -> Result<Self, Failure> {
        // Frame/map/vector headers and scalar work space. Each owner has a key,
        // vector header and record slots; each segment has four coordinates,
        // two endpoint memberships and its containing vector membership.
        b.retain(16)?;
        let mut frame = Self {
            owners: BTreeMap::new(),
            members: 16,
        };
        for (owner, group) in owners {
            b.step(1)?;
            if face_bounds.meets(&group.bounds, b)? {
                log_cost(frame.owners.len(), b)?;
                b.retain(6)?;
                frame.members += 6;
                frame.owners.insert(*owner, Vec::new());
            }
        }
        b.with_retained(originals.len(), |b| {
            let candidates = index.outward_candidates(face, &frame.owners, originals, b)?;
            for ordinal in candidates {
                b.step(1)?;
                let other = &originals[ordinal];
                if let Some(segment) = limiting_segment(face, other, b)? {
                    log_cost(frame.owners.len(), b)?;
                    let edges = frame.owners.get_mut(&other.id().component).ok_or_else(|| {
                        b.fail(FailureKind::UncertifiedContact, "section owner missing")
                    })?;
                    b.retain(7)?;
                    frame.members += 7;
                    edges.push(segment);
                }
            }
            Ok(())
        })?;
        Ok(frame)
    }

    pub(super) fn memberships(&self) -> usize {
        self.members
    }

    // None preserves the old generic-ray decision on any boundary ambiguity.
    // No values or segments survive into the next source facet's frame.
    pub(super) fn occupied(
        &self,
        sample: &Point2,
        b: &mut Budget,
    ) -> Result<Option<bool>, Failure> {
        b.guard(32)?;
        for edges in self.owners.values() {
            b.step(1)?;
            let mut inside = false;
            for edge in edges {
                match crosses(edge, sample, b)? {
                    Some(value) => inside ^= value,
                    None => return Ok(None),
                }
            }
            if inside {
                return Ok(Some(true));
            }
        }
        Ok(Some(false))
    }
}

fn crosses([a, c]: &Segment, p: &Point2, b: &mut Budget) -> Result<Option<bool>, Failure> {
    b.step(1)?;
    // The half-open y interval counts a vertex exactly once. Horizontal edges
    // still need the boundary check; duplicate edges are never deduplicated.
    if p == a || p == c {
        return Ok(None);
    }
    if a[1] == c[1] {
        return Ok(
            if p[1] == a[1]
                && a[0].clone().min(c[0].clone()) <= p[0]
                && p[0] <= a[0].clone().max(c[0].clone())
            {
                None
            } else {
                Some(false)
            },
        );
    }
    if (a[1] > p[1]) == (c[1] > p[1]) {
        return Ok(Some(false));
    }
    let side = orient(a, c, p, b)?;
    if side.is_zero() {
        return Ok(None);
    }
    Ok(Some(side.is_positive() == (c[1] > a[1])))
}

pub(super) fn fields_visible(
    face: &Facet,
    sample: &Point2,
    coplanar: &[&Facet],
    b: &mut Budget,
) -> Result<bool, Failure> {
    b.guard(48)?;
    let p = face.lift(sample, b)?;
    let mut winner = face.id();
    for other in coplanar {
        b.step(1)?;
        if !other.contains(&p, b)?.0 {
            continue;
        }
        if other.style() != face.style() {
            return Err(b.fail(
                FailureKind::ConflictingFields,
                "exposed coplanar material or texture conflict",
            ));
        }
        // Equality at three independent source vertices proves equality of the affine maps.
        for vertex in face.vertices() {
            if face.affine_uv(&vertex.point, b)? != other.affine_uv(&vertex.point, b)? {
                return Err(b.fail(
                    FailureKind::ConflictingFields,
                    "exposed complete affine UV conflict",
                ));
            }
        }
        winner = winner.min(other.id());
    }
    Ok(winner == face.id())
}

// A single source-ordered traversal supplies finite physical constraints and
// whole-owner outward sections. Three logical fields are admitted per result.
// One source-facet query only: omitted axes have exactly zero coefficient.
// No support value, intersection, ray or occupancy result is cached.
struct OutwardPlane<'a> {
    face: &'a Facet,
    axes: [usize; 3],
    count: usize,
}
impl<'a> OutwardPlane<'a> {
    fn new(face: &'a Facet, b: &mut Budget) -> Result<Self, Failure> {
        let mut result = Self {
            face,
            axes: [0; 3],
            count: 0,
        };
        for (axis, coefficient) in face.normal().iter().enumerate() {
            b.step(1)?;
            if !coefficient.is_zero() {
                result.axes[result.count] = axis;
                result.count += 1;
            }
        }
        Ok(result)
    }
    fn meets(&self, bounds: &Bounds, b: &mut Budget) -> Result<bool, Failure> {
        let mut low = -self.face.offset();
        let mut high = low.clone();
        for axis in &self.axes[..self.count] {
            b.step(1)?;
            let n = &self.face.normal()[*axis];
            let (a, c) = if n.is_negative() {
                (&bounds.upper[*axis], &bounds.lower[*axis])
            } else {
                (&bounds.lower[*axis], &bounds.upper[*axis])
            };
            low += n * a;
            high += n * c;
        }
        b.step(1)?;
        Ok(low <= integer(0) && high > integer(0))
    }
}

impl FacetBoundsIndex {
    fn collection_candidates(
        &self,
        face: &Facet,
        finite: &Bounds,
        owners: &BTreeMap<u32, Vec<Segment>>,
        originals: &[Facet],
        b: &mut Budget,
    ) -> Result<Vec<(usize, bool, bool)>, Failure> {
        b.with_retained(Self::stack_members(self.facets.len()) + 8, |b| {
            let plane = OutwardPlane::new(face, b)?;
            // Four fields per pending node fit the existing eight-membership
            // per-depth bound. Carry both exclusions to the descendants.
            let mut pending = vec![(self.root, true, true, false)];
            let mut result = Vec::new();
            while let Some((index, physical_possible, outward_possible, owner_known)) =
                pending.pop()
            {
                b.step(1)?;
                let node = &self.nodes[index];
                let physical_possible = physical_possible && finite.meets(&node.bounds, b)?;
                // A uniform owner is exact metadata about this source range,
                // including unordered inputs. Excluded owners cannot supply an
                // outward section; physical cuts remain independently eligible.
                let owner_allowed = if outward_possible && !owner_known {
                    if let Some(owner) = node.owner {
                        log_cost(owners.len(), b)?;
                        owners.contains_key(&owner)
                    } else {
                        true
                    }
                } else {
                    outward_possible
                };
                let owner_known = owner_known || node.owner.is_some();
                let outward_possible =
                    outward_possible && owner_allowed && plane.meets(&node.bounds, b)?;
                if !physical_possible && !outward_possible {
                    continue;
                }
                if let Some([left, right]) = node.children {
                    pending.push((right, physical_possible, outward_possible, owner_known));
                    pending.push((left, physical_possible, outward_possible, owner_known));
                } else {
                    for (offset, bounds) in self.facets[node.start..node.end].iter().enumerate() {
                        b.step(1)?;
                        let ordinal = node.start + offset;
                        let physical = physical_possible && finite.meets(bounds, b)?;
                        let outward = if outward_possible {
                            let owner_allowed = if node.owner.is_some() {
                                true
                            } else {
                                log_cost(owners.len(), b)?;
                                owners.contains_key(&originals[ordinal].id().component)
                            };
                            owner_allowed && plane.meets(bounds, b)?
                        } else {
                            false
                        };
                        if physical || outward {
                            result.push((ordinal, physical, outward));
                        }
                    }
                }
            }
            Ok(result)
        })
    }
}

type JointSection = (Vec<[crate::Point3; 2]>, bool, Option<Segment>);

// Caller retains 96 scratch memberships through construction and publication.
// Distances, edge visits and nonzero-edge hit coordinates are shared only when
// both representations require that exact same original-triangle calculation.
fn joint_section(
    face: &Facet,
    other: &Facet,
    physical: bool,
    outward: bool,
    b: &mut Budget,
) -> Result<JointSection, Failure> {
    if physical && !outward {
        let (segments, coplanar) = face.section(other, b)?;
        return Ok((segments, coplanar, None));
    }
    let mut distances = Vec::with_capacity(3);
    for vertex in other.vertices() {
        b.step(1)?;
        distances.push(face.distance(&vertex.point));
    }
    if distances.iter().all(Zero::is_zero) {
        let edges = if physical {
            (0..3)
                .map(|i| {
                    [
                        other.vertices()[i].point.clone(),
                        other.vertices()[(i + 1) % 3].point.clone(),
                    ]
                })
                .collect()
        } else {
            Vec::new()
        };
        return Ok((edges, true, None));
    }
    let mut hits = std::collections::BTreeSet::new();
    let mut limiting = Vec::with_capacity(2);
    for i in 0..3 {
        b.step(1)?;
        let j = (i + 1) % 3;
        if physical && distances[i].is_zero() {
            hits.insert(other.vertices()[i].point.clone());
        }
        let original_crossing = physical && (&distances[i] * &distances[j]).is_negative();
        let outward_crossing =
            outward && (distances[i].is_positive() != distances[j].is_positive());
        if !original_crossing && !outward_crossing {
            continue;
        }
        let t = &distances[i] / (&distances[i] - &distances[j]);
        let hit = add(
            &other.vertices()[i].point,
            &scale(
                &sub(&other.vertices()[j].point, &other.vertices()[i].point),
                &t,
            ),
        );
        point(&hit, b)?;
        if original_crossing {
            hits.insert(hit.clone());
        }
        if outward_crossing {
            limiting.push(project(&hit, face.axis()));
        }
    }
    let physical = if physical {
        b.step(hits.len() as u64)?;
        match (hits.first(), hits.last()) {
            (Some(a), Some(c)) => vec![[a.clone(), c.clone()]],
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let limiting = match limiting.as_slice() {
        [] => None,
        [a, c] => Some([a.clone(), c.clone()]),
        _ => {
            return Err(b.fail(
                FailureKind::UncertifiedContact,
                "one-sided triangle has incomplete section",
            ));
        }
    };
    Ok((physical, false, limiting))
}

impl Frame {
    pub(super) fn collect<'a>(
        face: &Facet,
        finite: &Bounds,
        originals: &'a [Facet],
        owners: &BTreeMap<u32, ComponentBounds>,
        index: &FacetBoundsIndex,
        b: &mut Budget,
    ) -> Result<(Vec<Segment>, Vec<&'a Facet>, Self), Failure> {
        let entry = b.retained_memberships();
        let result = Self::collect_admitted(face, finite, originals, owners, index, b);
        if result.is_err() {
            b.release(b.retained_memberships() - entry);
        }
        result
    }

    fn collect_admitted<'a>(
        face: &Facet,
        finite: &Bounds,
        originals: &'a [Facet],
        owners: &BTreeMap<u32, ComponentBounds>,
        index: &FacetBoundsIndex,
        b: &mut Budget,
    ) -> Result<(Vec<Segment>, Vec<&'a Facet>, Self), Failure> {
        b.retain(24)?;
        let mut frame = Self {
            owners: BTreeMap::new(),
            members: 24,
        };
        for (owner, group) in owners {
            b.step(1)?;
            if finite.meets(&group.bounds, b)? {
                log_cost(frame.owners.len(), b)?;
                b.retain(6)?;
                frame.members += 6;
                frame.owners.insert(*owner, Vec::new());
            }
        }
        let mut barriers = Vec::new();
        let mut coplanar = Vec::new();
        b.state([
            frame.members,
            frame.owners.len(),
            barriers.len(),
            coplanar.len(),
            originals.len(),
            0,
            0,
            0,
        ]);
        b.with_retained(3 * originals.len(), |b| {
            let candidates =
                index.collection_candidates(face, finite, &frame.owners, originals, b)?;
            for (ordinal, physical, outward) in candidates {
                b.state([
                    frame.members,
                    frame.owners.len(),
                    barriers.len(),
                    coplanar.len(),
                    originals.len(),
                    ordinal,
                    96,
                    0,
                ]);
                b.step(1)?;
                let other = &originals[ordinal];
                // This exact triangle is entirely on its own source plane and
                // contributes neither a physical cut nor an outward section.
                if face.id() == other.id() {
                    continue;
                }
                b.with_retained(96, |b| {
                    let (segments, same_plane, limiting) =
                        joint_section(face, other, physical, outward, b)?;
                    b.retain(8 * segments.len())?;
                    barriers.extend(
                        segments
                            .into_iter()
                            .map(|[a, c]| [project(&a, face.axis()), project(&c, face.axis())]),
                    );
                    if physical
                        && same_plane
                        && crate::exact::dot(face.normal(), other.normal()) > integer(0)
                    {
                        b.step(1)?;
                        b.retain(1)?;
                        coplanar.push(other);
                    }
                    if let Some(segment) = limiting {
                        log_cost(frame.owners.len(), b)?;
                        let edges =
                            frame.owners.get_mut(&other.id().component).ok_or_else(|| {
                                b.fail(FailureKind::UncertifiedContact, "section owner missing")
                            })?;
                        b.retain(7)?;
                        frame.members += 7;
                        edges.push(segment);
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?;
        Ok((barriers, coplanar, frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceId, Vertex, fraction};
    use std::collections::BTreeSet;

    fn triangle(points: [[i64; 3]; 3], owner: u32, ordinal: u32) -> Result<Facet, Failure> {
        let mut b = Budget::new(Limits::default())?;
        Facet::new(
            SourceId {
                component: owner,
                facet: ordinal,
            },
            points.map(|p| Vertex {
                point: p.map(integer),
                uv: [integer(0), integer(0)],
            }),
            "test".into(),
            &mut b,
        )
    }
    fn source() -> Result<Facet, Failure> {
        triangle([[0, 0, 0], [4, 0, 0], [0, 4, 0]], 100, 0)
    }
    fn key([a, c]: Segment) -> Segment {
        if a < c { [a, c] } else { [c, a] }
    }

    #[test]
    fn limiting_sections_match_hand_derived_zero_sign_and_tangent_cases() -> Result<(), Failure> {
        let face = source()?;
        // Other triangle has projected corners (0,0),(2,0),(0,2).
        // Expected endpoints are independently enumerated, including collapsed
        // points; all-coplanar and entirely negative/positive have no section.
        let cases = [
            ([0, 0, 1], Some([[0, 0], [2, 0]])),
            ([0, 0, -1], None),
            ([1, 1, 0], Some([[0, 2], [0, 2]])),
            ([-1, -1, 0], None),
            ([0, 0, 0], None),
            ([1, 1, 1], None),
            ([-1, -1, -1], None),
            ([-1, 1, 1], Some([[1, 0], [0, 1]])),
            ([0, -1, 1], Some([[0, 0], [1, 1]])),
        ];
        for (i, (z, expected)) in cases.into_iter().enumerate() {
            let other = triangle([[0, 0, z[0]], [2, 0, z[1]], [0, 2, z[2]]], 0, i as u32)?;
            let mut b = Budget::new(Limits::default())?;
            assert_eq!(
                limiting_segment(&face, &other, &mut b)?.map(key),
                expected.map(|p| key(p.map(|q| q.map(integer)))),
                "z={z:?}"
            );
        }
        Ok(())
    }

    fn square(low: i64, high: i64) -> Vec<Segment> {
        let p = [[low, low], [high, low], [high, high], [low, high]].map(|p| p.map(integer));
        (0..4)
            .map(|i| [p[i].clone(), p[(i + 1) % 4].clone()])
            .collect()
    }

    #[test]
    fn half_open_parity_preserves_cavity_multiplicity_and_boundary_ambiguity() -> Result<(), Failure>
    {
        let mut edges = square(-2, 2);
        edges.extend(square(-1, 1));
        let frame = Frame {
            owners: BTreeMap::from([(0, edges)]),
            members: 0,
        };
        let mut b = Budget::new(Limits::default())?;
        for (p, expected) in [
            (
                [fraction("0", "1", &mut b)?, fraction("0", "1", &mut b)?],
                Some(false),
            ),
            (
                [fraction("3", "2", &mut b)?, fraction("0", "1", &mut b)?],
                Some(true),
            ),
            (
                [fraction("3", "1", &mut b)?, fraction("0", "1", &mut b)?],
                Some(false),
            ),
            (
                [fraction("3", "2", &mut b)?, fraction("1", "1", &mut b)?],
                Some(true),
            ),
            (
                [fraction("1", "1", &mut b)?, fraction("0", "1", &mut b)?],
                None,
            ),
            (
                [fraction("2", "1", &mut b)?, fraction("2", "1", &mut b)?],
                None,
            ),
            (
                [fraction("0", "1", &mut b)?, fraction("-2", "1", &mut b)?],
                None,
            ),
        ] {
            assert_eq!(frame.occupied(&p, &mut b)?, expected, "{p:?}");
        }
        let mut twice = square(-2, 2);
        twice.extend(square(-2, 2));
        let duplicate = Frame {
            owners: BTreeMap::from([(0, twice)]),
            members: 0,
        };
        assert_eq!(
            duplicate.occupied(&[integer(0), integer(0)], &mut b)?,
            Some(false)
        );
        Ok(())
    }

    #[test]
    fn whole_plane_query_matches_independent_box_corner_support_and_storage_boundary()
    -> Result<(), Failure> {
        let face = source()?;
        let mut originals = Vec::new();
        for i in 0..25 {
            let z = i % 5 - 2;
            originals.push(triangle(
                [[i, 0, z], [i + 1, 0, z + 1], [i, 1, z]],
                (i % 3) as u32,
                i as u32,
            )?);
        }
        let mut setup = Budget::new(Limits::default())?;
        let index = FacetBoundsIndex::build(&originals, &mut setup)?;
        let owners = BTreeMap::from([(0, Vec::new()), (2, Vec::new())]);
        // Enumerate all eight box corners; do not call the support predicate.
        let mut expected = Vec::new();
        for (i, (other, bounds)) in originals.iter().zip(&index.facets).enumerate() {
            let distances: Vec<_> = (0..8)
                .map(|mask| {
                    let p = std::array::from_fn(|k| {
                        if mask & (1 << k) == 0 {
                            bounds.lower[k].clone()
                        } else {
                            bounds.upper[k].clone()
                        }
                    });
                    face.distance(&p)
                })
                .collect();
            if owners.contains_key(&other.id().component)
                && distances.iter().any(|d| d <= &integer(0))
                && distances.iter().any(|d| d > &integer(0))
            {
                expected.push(i);
            }
        }
        let mut b = Budget::new(Limits::default())?;
        let actual = b.with_retained(originals.len(), |b| {
            index.outward_candidates(&face, &owners, &originals, b)
        })?;
        assert_eq!(actual, expected);
        let peak = b.peak_memberships();
        let mut tight = Budget::new(Limits {
            storage_memberships: peak - 1,
            ..Limits::default()
        })?;
        let failure = tight
            .with_retained(originals.len(), |b| {
                index.outward_candidates(&face, &owners, &originals, b)
            })
            .err();
        assert_eq!(failure.map(|e| e.kind), Some(FailureKind::Storage));
        assert_eq!(tight.retained_memberships(), 0);
        // Oblique and reversed source normals exercise all support signs.
        for points in [
            [[0, 0, 0], [1, 1, 0], [0, 1, 1]],
            [[0, 0, 0], [0, 1, 1], [1, 1, 0]],
        ] {
            let face = triangle(points, 100, 1)?;
            let mut expected = BTreeSet::new();
            for (i, (other, bounds)) in originals.iter().zip(&index.facets).enumerate() {
                let values: Vec<_> = (0..8)
                    .map(|mask| {
                        face.distance(&std::array::from_fn(|k| {
                            if mask & (1 << k) == 0 {
                                bounds.lower[k].clone()
                            } else {
                                bounds.upper[k].clone()
                            }
                        }))
                    })
                    .collect();
                if owners.contains_key(&other.id().component)
                    && values.iter().any(|d| d <= &integer(0))
                    && values.iter().any(|d| d > &integer(0))
                {
                    expected.insert(i);
                }
            }
            let mut b = Budget::new(Limits::default())?;
            let found = b.with_retained(originals.len(), |b| {
                index.outward_candidates(&face, &owners, &originals, b)
            })?;
            assert_eq!(found.into_iter().collect::<BTreeSet<_>>(), expected);
        }
        Ok(())
    }

    #[test]
    fn section_frame_reserves_complete_ownership_and_releases_failed_build() -> Result<(), Failure>
    {
        // Two spanning source triangles exercise complete frame reservation.
        // Closed-owner enclosure is covered by the full buried/cavity controls.
        let face = triangle([[0, 0, 0], [1, 0, 0], [0, 1, 0]], 100, 0)?;
        let originals = vec![
            triangle([[-10, -10, -1], [10, -10, 1], [10, 10, 1]], 0, 0)?,
            triangle([[-10, -10, -1], [10, 10, 1], [-10, 10, -1]], 0, 1)?,
        ];
        let mut setup = Budget::new(Limits::default())?;
        let index = FacetBoundsIndex::build(&originals, &mut setup)?;
        let owners = super::super::component_bounds(&originals, &index.facets, &mut setup)?;
        let face_bounds = Bounds::for_facet(&face, &mut setup)?;
        let mut b = Budget::new(Limits::default())?;
        let frame = Frame::build(&face, &face_bounds, &originals, &owners, &index, &mut b)?;
        assert_eq!(frame.memberships(), 16 + 6 + 7 * 2);
        assert_eq!(b.retained_memberships(), frame.memberships());
        let peak = b.peak_memberships();
        let operations = b.operations();
        let members = frame.memberships();
        drop(frame);
        b.release(members);
        assert_eq!(b.retained_memberships(), 0);
        let mut exact = Budget::new(Limits {
            storage_memberships: peak,
            ..Limits::default()
        })?;
        let frame = Frame::build(&face, &face_bounds, &originals, &owners, &index, &mut exact)?;
        let members = frame.memberships();
        drop(frame);
        exact.release(members);
        let mut tight = Budget::new(Limits {
            storage_memberships: peak - 1,
            ..Limits::default()
        })?;
        let failure =
            Frame::build(&face, &face_bounds, &originals, &owners, &index, &mut tight).err();
        assert_eq!(failure.map(|e| e.kind), Some(FailureKind::Storage));
        assert_eq!(tight.retained_memberships(), 0);
        let mut work = Budget::new(Limits {
            operations: operations - 1,
            ..Limits::default()
        })?;
        let failure =
            Frame::build(&face, &face_bounds, &originals, &owners, &index, &mut work).err();
        assert_eq!(failure.map(|e| e.kind), Some(FailureKind::Operations));
        assert_eq!(work.retained_memberships(), 0);
        Ok(())
    }
    #[test]
    fn accounted_callback_preserves_arrangement_and_adds_live_classifier_storage()
    -> Result<(), Failure> {
        use crate::Arrangement;
        let domain = [[0, 0], [4, 0], [0, 4]].map(|p| p.map(integer));
        let barriers = [[[0, 1], [3, 1]].map(|p| p.map(integer))];
        let mut old = Budget::new(Limits::default())?;
        let expected = Arrangement::build(&domain, &barriers, &mut old, |_, b| {
            b.guard(10_000)?;
            Ok(true)
        })?;
        let mut new = Budget::new(Limits::default())?;
        let actual = Arrangement::build_accounted(&domain, &barriers, &mut new, |_, b| {
            assert!(b.retained_memberships() > 0);
            b.guard(10_000)?;
            Ok(true)
        })?;
        assert_eq!(actual.cells, expected.cells);
        assert_eq!(actual.physical, expected.physical);
        assert_eq!(actual.protected, expected.protected);
        assert_eq!(actual.work, expected.work);
        assert_eq!(new.operations(), old.operations());
        assert!(new.peak_memberships() > old.peak_memberships());
        assert_eq!(new.retained_memberships(), 0);
        let mut tight = Budget::new(Limits {
            storage_memberships: new.peak_memberships() - 1,
            ..Limits::default()
        })?;
        let failure = Arrangement::build_accounted(&domain, &barriers, &mut tight, |_, b| {
            b.guard(10_000)?;
            Ok(true)
        })
        .err();
        assert_eq!(failure.map(|e| e.kind), Some(FailureKind::Storage));
        assert_eq!(tight.retained_memberships(), 0);
        Ok(())
    }
    #[test]
    fn joint_sections_match_both_separate_oracles_for_all_signed_zero_triples()
    -> Result<(), Failure> {
        for face in [
            source()?,
            triangle([[0, 0, 0], [0, 4, 0], [4, 0, 0]], 100, 0)?,
        ] {
            for a in -1..=1 {
                for c in -1..=1 {
                    for d in -1..=1 {
                        let other = triangle([[0, 0, a], [2, 0, c], [0, 2, d]], 0, 0)?;
                        for physical in [false, true] {
                            for outward in [false, true] {
                                let mut old = Budget::new(Limits::default())?;
                                let expected_physical = if physical {
                                    Some(face.section(&other, &mut old)?)
                                } else {
                                    None
                                };
                                let expected_outward = if outward {
                                    limiting_segment(&face, &other, &mut old)?
                                } else {
                                    None
                                };
                                let mut new = Budget::new(Limits::default())?;
                                let (segments, coplanar, limiting) = new
                                    .with_retained(96, |b| {
                                        joint_section(&face, &other, physical, outward, b)
                                    })?;
                                if let Some((expected, same_plane)) = expected_physical {
                                    assert_eq!(
                                        segments, expected,
                                        "physical distances={a},{c},{d}"
                                    );
                                    assert_eq!(coplanar, same_plane);
                                } else {
                                    assert!(segments.is_empty());
                                }
                                assert_eq!(
                                    limiting, expected_outward,
                                    "limiting distances={a},{c},{d}"
                                );
                                if physical && outward {
                                    assert!(
                                        new.operations() < old.operations(),
                                        "shared calculation must eliminate duplicate charged work"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn joint_collection_preserves_order_remote_loops_and_complete_admission() -> Result<(), Failure>
    {
        for face in [
            source()?,
            triangle([[0, 0, 0], [2, 2, 0], [0, 2, 2]], 100, 0)?,
        ] {
            let mut originals = Vec::new();
            let mut ordinal = 0;
            for a in -1..=1 {
                for c in -1..=1 {
                    for d in -1..=1 {
                        originals.push(triangle(
                            [[0, 0, a], [2, 0, c], [0, 2, d]],
                            ordinal % 2,
                            ordinal,
                        )?);
                        ordinal += 1;
                    }
                }
            }
            // A remote pair remains in the whole-plane frame when its owner is
            // already possible because that owner also has nearby original facets.
            originals.push(triangle([[10, 0, -1], [12, 0, 1], [10, 2, 1]], 0, 100)?);
            originals.push(face.clone());
            originals.push(triangle([[20, 0, -1], [22, 0, 1], [20, 2, 1]], 7, 0)?);
            let mut setup = Budget::new(Limits::default())?;
            let index = FacetBoundsIndex::build(&originals, &mut setup)?;
            let owners = super::super::component_bounds(&originals, &index.facets, &mut setup)?;
            let finite = Bounds::for_facet(&face, &mut setup)?;
            let mut old = Budget::new(Limits::default())?;
            let frame = Frame::build(&face, &finite, &originals, &owners, &index, &mut old)?;
            let expected_owners = frame.owners.clone();
            let mut expected_barriers = Vec::new();
            let mut expected_coplanar = Vec::new();
            old.with_retained(originals.len(), |b| {
                for ordinal in index.candidates(&finite, b)? {
                    b.step(1)?;
                    let other = &originals[ordinal];
                    if face.id() == other.id() {
                        continue;
                    }
                    let (segments, same_plane) = face.section(other, b)?;
                    expected_barriers.extend(
                        segments
                            .into_iter()
                            .map(|[a, c]| [project(&a, face.axis()), project(&c, face.axis())]),
                    );
                    if same_plane && crate::exact::dot(face.normal(), other.normal()) > integer(0) {
                        b.step(1)?;
                        expected_coplanar.push(other.id());
                    }
                }
                Ok(())
            })?;
            let mut b = Budget::new(Limits::default())?;
            let (barriers, coplanar, actual) =
                Frame::collect(&face, &finite, &originals, &owners, &index, &mut b)?;
            assert_eq!(barriers, expected_barriers);
            assert_eq!(
                coplanar.iter().map(|f| f.id()).collect::<Vec<_>>(),
                expected_coplanar
            );
            assert_eq!(actual.owners, expected_owners);
            eprintln!(
                "jointCollection normal={:?} separateOperations={} jointOperations={} separatePeak={} jointPeak={} retained={}",
                face.normal(),
                old.operations(),
                b.operations(),
                old.peak_memberships(),
                b.peak_memberships(),
                b.retained_memberships()
            );
            assert!(
                b.operations() < old.operations(),
                "joint collector must eliminate duplicated traversal/construction work"
            );
            let members = actual.memberships() + 8 * barriers.len() + coplanar.len();
            assert_eq!(b.retained_memberships(), members);
            let peak = b.peak_memberships();
            let operations = b.operations();
            drop((barriers, coplanar, actual));
            b.release(members);
            assert_eq!(b.retained_memberships(), 0);
            for limits in [
                Limits {
                    storage_memberships: peak - 1,
                    ..Limits::default()
                },
                Limits {
                    operations: operations - 1,
                    ..Limits::default()
                },
            ] {
                let mut tight = Budget::new(limits)?;
                let expected = if limits.operations < Limits::default().operations {
                    FailureKind::Operations
                } else {
                    FailureKind::Storage
                };
                assert_eq!(
                    Frame::collect(&face, &finite, &originals, &owners, &index, &mut tight)
                        .err()
                        .map(|e| e.kind),
                    Some(expected)
                );
                assert_eq!(tight.retained_memberships(), 0);
            }
            let mut exact = Budget::new(Limits {
                storage_memberships: peak,
                operations,
                ..Limits::default()
            })?;
            let (barriers, coplanar, frame) =
                Frame::collect(&face, &finite, &originals, &owners, &index, &mut exact)?;
            let members = frame.memberships() + 8 * barriers.len() + coplanar.len();
            drop((barriers, coplanar, frame));
            exact.release(members);
            assert_eq!(exact.retained_memberships(), 0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod live_overlap_tests {
    use super::*;
    use crate::{Limits, SourceId, Vertex};
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/live-overlap-reference/compiled-collection-control.rs"
    ));

    #[test]
    fn owner_pruning_matches_frozen_candidates_for_uniform_mixed_and_unordered_ranges()
    -> Result<(), Failure> {
        let mut setup = Budget::new(Limits::default())?;
        let make = |i: u32, owner: u32, b: &mut Budget| {
            Facet::new(
                SourceId {
                    component: owner,
                    facet: i,
                },
                [[-10, -10, -1], [10, -10, 1], [0, 10, 1]].map(|p| Vertex {
                    point: p.map(integer),
                    uv: [integer(0), integer(0)],
                }),
                "test".into(),
                b,
            )
        };
        let face = Facet::new(
            SourceId {
                component: 100,
                facet: 0,
            },
            [[0, 0, 0], [1, 0, 0], [0, 1, 0]].map(|p| Vertex {
                point: p.map(integer),
                uv: [integer(0), integer(0)],
            }),
            "test".into(),
            &mut setup,
        )?;
        // Every source crosses the plane; finite exclusion isolates outward
        // owner pruning from the independently preserved physical query.
        let far = Bounds {
            lower: [integer(100), integer(100), integer(100)],
            upper: [integer(101), integer(101), integer(101)],
        };
        for unordered in [false, true] {
            let mut originals = Vec::new();
            for i in 0..128 {
                originals.push(make(i, if unordered { i % 3 } else { i / 32 }, &mut setup)?);
            }
            let index = FacetBoundsIndex::build(&originals, &mut setup)?;
            for node in &index.nodes {
                let ids: std::collections::BTreeSet<_> = originals[node.start..node.end]
                    .iter()
                    .map(|f| f.id().component)
                    .collect();
                let expected = if ids.len() == 1 {
                    ids.first().copied()
                } else {
                    None
                };
                assert_eq!(node.owner, expected);
            }
            for owners in [
                BTreeMap::new(),
                BTreeMap::from([(1, Vec::new())]),
                BTreeMap::from([
                    (0, Vec::new()),
                    (1, Vec::new()),
                    (2, Vec::new()),
                    (3, Vec::new()),
                ]),
            ] {
                for bounds in [&far, &index.facets[0]] {
                    let mut old = Budget::new(Limits::default())?;
                    let mut new = Budget::new(Limits::default())?;
                    let expected = index.frozen_collection_candidates(
                        &face, bounds, &owners, &originals, &mut old,
                    )?;
                    let found = index
                        .collection_candidates(&face, bounds, &owners, &originals, &mut new)?;
                    assert_eq!(found, expected);
                    assert_eq!(new.retained_memberships(), 0);
                    if !unordered && owners.is_empty() {
                        assert!(new.operations() < old.operations());
                    }
                    let mut tight = Budget::new(Limits {
                        storage_memberships: FacetBoundsIndex::stack_members(originals.len()) - 1,
                        ..Limits::default()
                    })?;
                    assert_eq!(
                        index
                            .collection_candidates(&face, bounds, &owners, &originals, &mut tight)
                            .err()
                            .map(|e| e.kind),
                        Some(FailureKind::Storage)
                    );
                    assert_eq!(tight.retained_memberships(), 0);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod plane_support_tests {
    use super::*;
    use crate::{Limits, SourceId, Vertex};

    #[test]
    fn zero_axis_support_matches_full_dot_for_oblique_reversed_and_boundary_boxes()
    -> Result<(), Failure> {
        let mut setup = Budget::new(Limits::default())?;
        for points in [
            [[0, 0, 0], [2, 0, 0], [0, 2, 0]],
            [[0, 0, 0], [0, 2, 0], [2, 0, 0]],
            [[0, 0, 0], [0, 2, 0], [0, 0, 2]],
            [[0, 0, 0], [1, 1, 0], [0, 1, 1]],
            [[0, 0, 0], [0, 1, 1], [1, 1, 0]],
        ] {
            let face = Facet::new(
                SourceId {
                    component: 0,
                    facet: 0,
                },
                points.map(|p| Vertex {
                    point: p.map(integer),
                    uv: [integer(0), integer(0)],
                }),
                "test".into(),
                &mut setup,
            )?;
            let mut old = Budget::new(Limits::default())?;
            let mut new = Budget::new(Limits::default())?;
            let plane = OutwardPlane::new(&face, &mut new)?;
            for lower in [
                [-2, -2, -2],
                [0, 0, 0],
                [1, 1, 1],
                [-1, -1, 0],
                [-1, -1, -1],
            ] {
                for width in [0, 1, 4] {
                    let bounds = Bounds {
                        lower: lower.map(integer),
                        upper: lower.map(|n| integer(n + width)),
                    };
                    assert_eq!(
                        plane.meets(&bounds, &mut new)?,
                        bounds.meets_outward_plane(&face, &mut old)?
                    );
                }
            }
            if plane.count == 1 {
                assert!(new.operations() < old.operations());
            }
            if plane.count == 3 {
                assert_eq!(new.operations(), old.operations() + 3);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod owner_inheritance_tests {
    use super::*;
    use crate::{Limits, SourceId, Vertex};
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/proved-visits-reference/compiled-collection-control.rs"
    ));

    #[test]
    fn inherited_owner_proof_preserves_all_candidates_and_removes_repeat_lookups()
    -> Result<(), Failure> {
        let mut setup = Budget::new(Limits::default())?;
        let make = |owner, facet, b: &mut Budget| {
            Facet::new(
                SourceId {
                    component: owner,
                    facet,
                },
                [[-2, -2, -1], [2, -2, 1], [0, 2, 1]].map(|p| Vertex {
                    point: p.map(integer),
                    uv: [integer(0), integer(0)],
                }),
                "test".into(),
                b,
            )
        };
        let face = Facet::new(
            SourceId {
                component: 9,
                facet: 0,
            },
            [[0, 0, 0], [1, 0, 0], [0, 1, 0]].map(|p| Vertex {
                point: p.map(integer),
                uv: [integer(0), integer(0)],
            }),
            "test".into(),
            &mut setup,
        )?;
        let far = Bounds {
            lower: [integer(10), integer(10), integer(10)],
            upper: [integer(11), integer(11), integer(11)],
        };
        for mode in 0..3 {
            let mut originals = Vec::new();
            for i in 0..128 {
                let owner = match mode {
                    0 => 1,
                    1 => 1 + i / 64,
                    _ => 1 + i % 2,
                };
                originals.push(make(owner, i, &mut setup)?);
            }
            let index = FacetBoundsIndex::build(&originals, &mut setup)?;
            for node in &index.nodes {
                if let (Some(owner), Some([left, right])) = (node.owner, node.children) {
                    assert_eq!(index.nodes[left].owner, Some(owner));
                    assert_eq!(index.nodes[right].owner, Some(owner));
                }
            }
            for owners in [
                BTreeMap::new(),
                BTreeMap::from([(1, Vec::new())]),
                BTreeMap::from([(1, Vec::new()), (2, Vec::new())]),
            ] {
                for bounds in [&far, &index.facets[0]] {
                    let mut old = Budget::new(Limits::default())?;
                    let mut new = Budget::new(Limits::default())?;
                    let expected = index.owner_inheritance_reference(
                        &face, bounds, &owners, &originals, &mut old,
                    )?;
                    let found = index
                        .collection_candidates(&face, bounds, &owners, &originals, &mut new)?;
                    assert_eq!(found, expected);
                    assert!(new.operations() <= old.operations());
                    if mode < 2 && owners.contains_key(&1) {
                        assert!(new.operations() < old.operations());
                    }
                    assert_eq!(new.retained_memberships(), 0);
                    let mut tight = Budget::new(Limits {
                        storage_memberships: FacetBoundsIndex::stack_members(originals.len()) + 8
                            - 1,
                        ..Limits::default()
                    })?;
                    assert_eq!(
                        index
                            .collection_candidates(&face, bounds, &owners, &originals, &mut tight)
                            .err()
                            .map(|e| e.kind),
                        Some(FailureKind::Storage)
                    );
                    assert_eq!(tight.retained_memberships(), 0);
                }
            }
        }
        Ok(())
    }
}

#[path = "parity_index.rs"]
mod parity_index;
pub(super) use parity_index::ParityIndex;
