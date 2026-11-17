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
