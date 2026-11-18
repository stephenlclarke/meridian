#[path = "sections.rs"]
mod sections;

use crate::BudgetPhase;
use crate::exact::{add, area, cross, dot, orient, point, project, scale, sub};
use crate::planes::{CachedRay, PlaneGroups, RayCaches};
use crate::region::PackedRegion;
use crate::{
    Arrangement, Budget, Cell, Facet, Failure, FailureKind, Point2, Point3, Rational, SourceId,
    Vertex, integer,
};
use num_traits::{Signed, Zero};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactStar {
    pub point: Point2,
    pub sectors: Vec<[Point2; 2]>,
    /// True for a boundary vertex. Protected contacts can retain several local paths.
    pub boundary_path: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FacetWitness {
    pub source: SourceId,
    pub classified_cells: Vec<Cell>,
    pub physical_segments: Vec<[Point2; 2]>,
    pub protected_points: Vec<Point2>,
    pub output_triangles: Vec<[Point2; 3]>,
    pub contact_stars: Vec<ContactStar>,
    pub twice_area: Rational,
}
/// An audited candidate. Getters expose immutable geometry and witnesses.
/// Fleet registration and native admission require their own source binding.
#[derive(Debug)]
pub struct Hull {
    triangles: Vec<Facet>,
    witnesses: Vec<FacetWitness>,
    volume: Rational,
}
impl Hull {
    pub fn triangles(&self) -> &[Facet] {
        &self.triangles
    }
    pub fn witnesses(&self) -> &[FacetWitness] {
        &self.witnesses
    }
    pub fn exact_volume(&self) -> &Rational {
        &self.volume
    }
}

fn on_segment<const N: usize>(
    a: &[Rational; N],
    c: &[Rational; N],
    p: &[Rational; N],
    b: &mut Budget,
) -> Result<bool, Failure> {
    b.step(N as u64)?;
    let delta = sub(c, a);
    let from = sub(p, a);
    let Some(axis) = delta.iter().position(|v| !v.is_zero()) else {
        return Ok(a == p);
    };
    let t = &from[axis] / &delta[axis];
    Ok(t >= integer(0) && t <= integer(1) && (0..N).all(|i| from[i] == &t * &delta[i]))
}
fn ordered_edge<const N: usize>(a: &[Rational; N], c: &[Rational; N]) -> [[Rational; N]; 2] {
    if a < c {
        [a.clone(), c.clone()]
    } else {
        [c.clone(), a.clone()]
    }
}
fn log_cost(count: usize, b: &mut Budget) -> Result<(), Failure> {
    b.step((count.saturating_add(1).ilog2() + 1).into())
}

/// Validate geometric two-sided incidence and the connected link at each vertex.
fn audit_closed<'a>(
    faces: impl ExactSizeIterator<Item = &'a Facet> + Clone,
    per_component: bool,
    b: &mut Budget,
) -> Result<(), Failure> {
    b.guard(96 * faces.len() + 64)?;
    let mut edges: BTreeMap<(u32, [Point3; 2]), Vec<bool>> = BTreeMap::new();
    let mut stars: BTreeMap<(u32, Point3), Vec<[Point3; 2]>> = BTreeMap::new();
    for face in faces.clone() {
        b.at(face.id());
        let owner = if per_component {
            face.id().component
        } else {
            0
        };
        for i in 0..3 {
            b.step(3)?;
            let p = &face.vertices()[i].point;
            let next = &face.vertices()[(i + 1) % 3].point;
            let prev = &face.vertices()[(i + 2) % 3].point;
            log_cost(edges.len() + stars.len(), b)?;
            edges
                .entry((owner, ordered_edge(p, next)))
                .or_default()
                .push(p < next);
            stars
                .entry((owner, p.clone()))
                .or_default()
                .push([prev.clone(), next.clone()]);
        }
    }
    let mut signed_volumes: BTreeMap<u32, Rational> = BTreeMap::new();
    for face in faces {
        b.step(1)?;
        let owner = if per_component {
            face.id().component
        } else {
            0
        };
        *signed_volumes.entry(owner).or_insert_with(|| integer(0)) += dot(
            &face.vertices()[0].point,
            &cross(&face.vertices()[1].point, &face.vertices()[2].point),
        );
    }
    if signed_volumes.values().any(|v| v <= &integer(0)) {
        return Err(b.fail(
            FailureKind::NonManifold,
            "source component has nonpositive signed volume",
        ));
    }
    for entries in edges.values() {
        b.step(entries.len() as u64)?;
        if entries.len() != 2 || entries[0] == entries[1] {
            return Err(b.fail(
                FailureKind::NonManifold,
                "edge lacks reciprocal two-sided incidence",
            ));
        }
    }
    for sectors in stars.values() {
        connected_link(sectors, true, b)?;
    }
    Ok(())
}
fn audit_original_components(
    originals: &[Facet],
    owners: &BTreeSet<u32>,
    b: &mut Budget,
) -> Result<(), Failure> {
    // All original facets remain retained. Source topology maps are keyed by
    // component, so finish and release one component's audit before the next.
    // The completed union uses a separate global audit with no such partition.
    for owner in owners {
        b.step(originals.len() as u64)?;
        let count = originals
            .iter()
            .filter(|f| f.id().component == *owner)
            .count();
        b.with_retained(count, |budget| {
            let group = originals
                .iter()
                .filter(|f| f.id().component == *owner)
                .collect::<Vec<_>>();
            budget.step(originals.len() as u64)?;
            audit_closed(group.iter().copied(), true, budget)
        })?;
    }
    Ok(())
}

// The caller already authenticated the owner set against the same input.
// Verify ascending runs before auditing: an inversion must use the original
// owner-first order, including when a later owner has a semantic defect.
fn audit_original_owner_ranges(
    originals: &[Facet],
    owners: &BTreeSet<u32>,
    b: &mut Budget,
) -> Result<(), Failure> {
    b.step(1)?;
    if owners.len() == 1 {
        return audit_closed(originals.iter(), true, b);
    }
    let ordered = b.with_retained(owners.len() + 8, |b| {
        let mut ends = Vec::with_capacity(owners.len());
        for (offset, pair) in originals.windows(2).enumerate() {
            b.step(1)?;
            let left = pair[0].id().component;
            let right = pair[1].id().component;
            if left > right {
                return Ok(false);
            }
            if left < right {
                ends.push(offset + 1);
            }
        }
        ends.push(originals.len());
        let mut start = 0;
        for end in ends {
            b.step(1)?;
            audit_closed(originals[start..end].iter(), true, b)?;
            start = end;
        }
        Ok(true)
    })?;
    if ordered {
        Ok(())
    } else {
        audit_original_components(originals, owners, b)
    }
}

fn connected_link<const N: usize>(
    sectors: &[[[Rational; N]; 2]],
    cycle: bool,
    b: &mut Budget,
) -> Result<(), Failure> {
    b.guard(24 * sectors.len() + 64)?;
    let mut graph: BTreeMap<[Rational; N], Vec<[Rational; N]>> = BTreeMap::new();
    for [a, c] in sectors {
        b.step(2)?;
        log_cost(graph.len(), b)?;
        graph.entry(a.clone()).or_default().push(c.clone());
        graph.entry(c.clone()).or_default().push(a.clone());
    }
    let ends = graph.values().filter(|v| v.len() == 1).count();
    for neighbours in graph.values() {
        b.step(1)?;
        if neighbours.len() > 2 || neighbours.is_empty() {
            return Err(b.fail(
                FailureKind::UncertifiedContact,
                "branching contact-star link",
            ));
        }
    }
    if cycle && ends != 0 || !cycle && ends != 2 {
        return Err(b.fail(
            FailureKind::UncertifiedContact,
            "contact-star boundary type mismatch",
        ));
    }
    let Some(first) = graph.keys().next() else {
        return Err(b.fail(FailureKind::UncertifiedContact, "empty contact star"));
    };
    let mut pending = vec![first.clone()];
    let mut seen = BTreeSet::new();
    while let Some(p) = pending.pop() {
        b.step(1)?;
        log_cost(seen.len(), b)?;
        if !seen.insert(p.clone()) {
            continue;
        }
        for q in &graph[&p] {
            b.step(1)?;
            if !seen.contains(q) {
                pending.push(q.clone());
            }
        }
    }
    if seen.len() != graph.len() {
        return Err(b.fail(
            FailureKind::UncertifiedContact,
            "disconnected contact-star link",
        ));
    }
    Ok(())
}
#[derive(Clone)]
struct Bounds {
    lower: Point3,
    upper: Point3,
}
impl Bounds {
    fn for_facet(face: &Facet, b: &mut Budget) -> Result<Self, Failure> {
        b.step(6)?;
        let v = face.vertices();
        Ok(Self {
            lower: std::array::from_fn(|k| {
                v[0].point[k]
                    .clone()
                    .min(v[1].point[k].clone())
                    .min(v[2].point[k].clone())
            }),
            upper: std::array::from_fn(|k| {
                v[0].point[k]
                    .clone()
                    .max(v[1].point[k].clone())
                    .max(v[2].point[k].clone())
            }),
        })
    }
    fn include(&mut self, other: &Self) {
        for k in 0..3 {
            self.lower[k] = self.lower[k].clone().min(other.lower[k].clone());
            self.upper[k] = self.upper[k].clone().max(other.upper[k].clone());
        }
    }
    fn meets(&self, other: &Self, b: &mut Budget) -> Result<bool, Failure> {
        b.step(1)?;
        Ok((0..3).all(|k| self.lower[k] <= other.upper[k] && other.lower[k] <= self.upper[k]))
    }
    fn contains(&self, point: &Point3, b: &mut Budget) -> Result<bool, Failure> {
        b.step(1)?;
        Ok((0..3).all(|k| self.lower[k] <= point[k] && point[k] <= self.upper[k]))
    }
    // Necessary forward-ray conditions only. Equality remains eligible;
    // the exact triangle test still decides every hit and ambiguity.
    fn ray_may_meet(
        &self,
        origin: &Point3,
        direction: &Point3,
        b: &mut Budget,
    ) -> Result<bool, Failure> {
        for k in 0..3 {
            b.step(1)?;
            if direction[k].is_positive() && self.upper[k] < origin[k]
                || direction[k].is_negative() && self.lower[k] > origin[k]
                || direction[k].is_zero()
                    && (origin[k] < self.lower[k] || origin[k] > self.upper[k])
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    // Intersect one shared closed parameter interval, including t=0. These
    // exact ray parameters use the same arithmetic as the existing hit test.
    fn ray_intersects(
        &self,
        origin: &Point3,
        direction: &Point3,
        b: &mut Budget,
    ) -> Result<bool, Failure> {
        let mut near = integer(0);
        let mut far: Option<Rational> = None;
        for k in 0..3 {
            b.step(1)?;
            if direction[k].is_zero() {
                if origin[k] < self.lower[k] || origin[k] > self.upper[k] {
                    return Ok(false);
                }
                continue;
            }
            // Two newly computed exact endpoint parameters, in addition to
            // the original axis visit. No coordinate or guard price changes.
            b.step(2)?;
            let first = (&self.lower[k] - &origin[k]) / &direction[k];
            let second = (&self.upper[k] - &origin[k]) / &direction[k];
            let (entry, exit) = if direction[k].is_positive() {
                (first, second)
            } else {
                (second, first)
            };
            near = near.max(entry);
            far = Some(match far {
                Some(old) => old.min(exit),
                None => exit,
            });
            if far.as_ref().is_some_and(|exit| &near > exit) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
// Contiguous ranges preserve original source order without a sort or tie rule.
struct BoundsNode {
    bounds: Bounds,
    start: usize,
    end: usize,
    children: Option<[usize; 2]>,
    owner: Option<u32>,
}
struct FacetBoundsIndex {
    facets: Vec<Bounds>,
    nodes: Vec<BoundsNode>,
    root: usize,
    order: Option<Vec<usize>>,
}
struct RayQuery<'a> {
    origin: &'a Point3,
    direction: &'a Point3,
    originals: &'a [Facet],
    owner: u32,
    owner_bounds: &'a Bounds,
}

impl FacetBoundsIndex {
    const LEAF_SIZE: usize = 8;
    fn node_limit(count: usize) -> usize {
        2 * count.div_ceil(Self::LEAF_SIZE).next_power_of_two() - 1
    }
    fn memberships(count: usize) -> usize {
        // Cached six scalar bounds, record/order slots; node bounds, ranges,
        // child links, record/tag slots and exact uniform-owner tag/value; headers.
        8 * count + 14 * Self::node_limit(count) + 8
    }
    fn stack_members(count: usize) -> usize {
        8 * (count.ilog2() as usize + 2) + 32
    }
    // The caller retains the complete cache/node bound before entering here.
    fn build(originals: &[Facet], b: &mut Budget) -> Result<Self, Failure> {
        let mut index = Self {
            facets: originals
                .iter()
                .map(|face| Bounds::for_facet(face, b))
                .collect::<Result<_, _>>()?,
            nodes: Vec::with_capacity(Self::node_limit(originals.len())),
            root: 0,
            order: None,
        };
        index.root = b.with_retained(Self::stack_members(originals.len()), |b| {
            index.build_range(originals, 0, originals.len(), b)
        })?;
        Ok(index)
    }
    fn build_range(
        &mut self,
        originals: &[Facet],
        start: usize,
        end: usize,
        b: &mut Budget,
    ) -> Result<usize, Failure> {
        let (bounds, children, owner) = if end - start <= Self::LEAF_SIZE {
            b.step(6)?;
            let mut bounds = self.facets[start].clone();
            for other in &self.facets[start + 1..end] {
                b.step(6)?;
                bounds.include(other);
            }
            let mut owner = Some(originals[start].id().component);
            for face in &originals[start + 1..end] {
                b.step(1)?;
                if owner != Some(face.id().component) {
                    owner = None;
                }
            }
            (bounds, None, owner)
        } else {
            let middle = start + (end - start) / 2;
            let left = self.build_range(originals, start, middle, b)?;
            let right = self.build_range(originals, middle, end, b)?;
            b.step(12)?;
            let mut bounds = self.nodes[left].bounds.clone();
            bounds.include(&self.nodes[right].bounds);
            b.step(1)?;
            let owner = self.nodes[left]
                .owner
                .filter(|owner| Some(*owner) == self.nodes[right].owner);
            (bounds, Some([left, right]), owner)
        };
        b.step(1)?;
        let node = self.nodes.len();
        self.nodes.push(BoundsNode {
            bounds,
            start,
            end,
            children,
            owner,
        });
        Ok(node)
    }
    // The caller retains up to one candidate ordinal per original facet.
    fn candidates(&self, query: &Bounds, b: &mut Budget) -> Result<Vec<usize>, Failure> {
        b.with_retained(Self::stack_members(self.facets.len()), |b| {
            let mut pending = vec![self.root];
            let mut candidates = Vec::new();
            while let Some(index) = pending.pop() {
                b.step(1)?;
                let node = &self.nodes[index];
                if !query.meets(&node.bounds, b)? {
                    continue;
                }
                if let Some([left, right]) = node.children {
                    pending.push(right);
                    pending.push(left);
                } else {
                    for position in node.start..node.end {
                        b.step(1)?;
                        let ordinal = self.source_ordinal(position);
                        let bounds = &self.facets[ordinal];
                        if query.meets(bounds, b)? {
                            candidates.push(ordinal);
                        }
                    }
                }
            }
            self.source_order(candidates, 1, |ordinal| *ordinal, b)
        })
    }
    // Retained for the exact axis-aligned specialization and frozen tests.
    fn ray_candidates(
        &self,
        origin: &Point3,
        direction: &Point3,
        originals: &[Facet],
        owner: u32,
        owner_bounds: &Bounds,
        b: &mut Budget,
    ) -> Result<Vec<usize>, Failure> {
        self.query_ray(
            RayQuery {
                origin,
                direction,
                originals,
                owner,
                owner_bounds,
            },
            Bounds::ray_may_meet,
            b,
        )
    }
    fn ray_intersections(
        &self,
        origin: &Point3,
        direction: &Point3,
        originals: &[Facet],
        owner: u32,
        owner_bounds: &Bounds,
        b: &mut Budget,
    ) -> Result<Vec<usize>, Failure> {
        b.step(3)?;
        if direction.iter().filter(|v| !v.is_zero()).count() <= 1 {
            // For one nonzero axis, the original half-line predicate is exact.
            return self.ray_candidates(origin, direction, originals, owner, owner_bounds, b);
        }
        self.query_ray(
            RayQuery {
                origin,
                direction,
                originals,
                owner,
                owner_bounds,
            },
            Bounds::ray_intersects,
            b,
        )
    }
    // The caller admits N returned ordinals. The existing stack bound includes
    // query references and the shared near/far endpoint scratch.
    fn query_ray(
        &self,
        query: RayQuery<'_>,
        possible: fn(&Bounds, &Point3, &Point3, &mut Budget) -> Result<bool, Failure>,
        b: &mut Budget,
    ) -> Result<Vec<usize>, Failure> {
        b.with_retained(Self::stack_members(self.facets.len()), |b| {
            let mut pending = vec![self.root];
            let mut candidates = Vec::new();
            while let Some(index) = pending.pop() {
                b.step(1)?;
                let node = &self.nodes[index];
                if !query.owner_bounds.meets(&node.bounds, b)?
                    || !possible(&node.bounds, query.origin, query.direction, b)?
                {
                    continue;
                }
                if let Some([left, right]) = node.children {
                    pending.push(right);
                    pending.push(left);
                } else {
                    for position in node.start..node.end {
                        b.step(1)?;
                        let ordinal = self.source_ordinal(position);
                        let bounds = &self.facets[ordinal];
                        if query.originals[ordinal].id().component == query.owner
                            && possible(bounds, query.origin, query.direction, b)?
                        {
                            candidates.push(ordinal);
                        }
                    }
                }
            }
            self.source_order(candidates, 1, |ordinal| *ordinal, b)
        })
    }
}

struct ComponentBounds {
    bounds: Bounds,
    facets: usize,
}

fn component_bounds(
    originals: &[Facet],
    facet_bounds: &[Bounds],
    b: &mut Budget,
) -> Result<BTreeMap<u32, ComponentBounds>, Failure> {
    let mut result: BTreeMap<u32, ComponentBounds> = BTreeMap::new();
    for (face, bounds) in originals.iter().zip(facet_bounds) {
        // Six bound coordinates plus one cached owner-count update.
        b.step(7)?;
        log_cost(result.len(), b)?;
        result
            .entry(face.id().component)
            .and_modify(|old| {
                old.bounds.include(bounds);
                old.facets += 1;
            })
            .or_insert_with(|| ComponentBounds {
                bounds: bounds.clone(),
                facets: 1,
            });
    }
    Ok(result)
}

struct OccupancyIndex<'a> {
    spatial: &'a FacetBoundsIndex,
    planes: Option<&'a PlaneGroups>,
    caches: &'a mut RayCaches,
}
fn build_plane_groups(
    originals: &[Facet],
    owners: &BTreeMap<u32, ComponentBounds>,
    b: &mut Budget,
) -> Result<Option<PlaneGroups>, Failure> {
    let entry = b.retained_memberships();
    let result = b.with_retained(8, |b| {
        let mut large = BTreeSet::new();
        for (owner, group) in owners {
            b.step(1)?;
            if group.facets > 4 * FacetBoundsIndex::LEAF_SIZE {
                log_cost(large.len(), b)?;
                b.retain(2)?;
                large.insert(*owner);
            }
        }
        if large.is_empty() {
            return Ok(None);
        }
        let result = PlaneGroups::build(originals, &large, b);
        let members = 2 * large.len();
        drop(large);
        b.release(members);
        result.map(Some)
    });
    if result.is_err() {
        b.release(b.retained_memberships() - entry);
    }
    result
}

fn ray_parity<'a>(
    faces: impl Iterator<Item = &'a Facet>,
    p: &Point3,
    direction: &Point3,
    b: &mut Budget,
) -> Result<Option<bool>, Failure> {
    let mut parity = false;
    for face in faces {
        b.step(1)?;
        let denominator = dot(face.normal(), direction);
        let numerator = face.offset() - dot(face.normal(), p);
        if denominator.is_zero() {
            if numerator.is_zero() && face.contains(p, b)?.0 {
                return Ok(None);
            }
            continue;
        }
        let t = numerator / denominator;
        if t <= integer(0) {
            continue;
        }
        let hit = add(p, &scale(direction, &t));
        point(&hit, b)?;
        let (inside, edge) = face.contains(&hit, b)?;
        if !inside {
            continue;
        }
        if edge {
            return Ok(None);
        }
        parity = !parity;
    }
    Ok(Some(parity))
}

fn occupied(
    p: &Point3,
    normal: &Point3,
    originals: &[Facet],
    owners: &BTreeMap<u32, ComponentBounds>,
    index: &mut OccupancyIndex<'_>,
    b: &mut Budget,
) -> Result<bool, Failure> {
    for (owner, group) in owners {
        // Strict exclusion only. Equality keeps all face/edge/point contacts.
        if !group.bounds.contains(p, b)? {
            continue;
        }
        let mut attempt = 0u64;
        loop {
            b.step(1)?;
            attempt = attempt
                .checked_add(1)
                .ok_or_else(|| b.fail(FailureKind::Operations, "ray attempt overflow"))?;
            let n = Rational::from_integer(attempt.into());
            let mut direction = [integer(1), n.clone(), &n * &n];
            let facing = dot(&direction, normal);
            if facing.is_zero() {
                continue;
            }
            if facing.is_negative() {
                direction = scale(&direction, &integer(-1));
            }
            let result = if group.facets <= 4 * FacetBoundsIndex::LEAF_SIZE {
                // A flat scan avoids tree overhead for small original owners.
                ray_parity(
                    originals
                        .iter()
                        .filter(|face| face.id().component == *owner),
                    p,
                    &direction,
                    b,
                )?
            } else {
                let repeated = match index.planes {
                    Some(planes) => {
                        let count = planes.count(*owner, b)?;
                        // Count initialization, one computation per equation and
                        // this lookup against the maximum saved coordinate checks.
                        planes
                            .can_amortize(group.facets, count)
                            .then_some((planes, count))
                    }
                    None => None,
                };
                if let Some((planes, count)) = repeated {
                    let faces = originals
                        .iter()
                        .enumerate()
                        .filter(|(_, face)| face.id().component == *owner);
                    let result = if index.caches.can_reuse(planes, count) {
                        index.caches.parity(
                            CachedRay {
                                groups: planes,
                                owner: *owner,
                                count,
                                origin: p,
                                direction: &direction,
                            },
                            faces,
                            b,
                        )?
                    } else {
                        planes.parity(count, faces, p, &direction, b)?
                    };
                    match result {
                        None => continue,
                        Some(true) => return Ok(true),
                        Some(false) => break,
                    }
                }
                b.with_retained(originals.len(), |b| {
                    let candidates = index.spatial.ray_intersections(
                        p,
                        &direction,
                        originals,
                        *owner,
                        &group.bounds,
                        b,
                    )?;
                    ray_parity(
                        candidates.into_iter().map(|ordinal| &originals[ordinal]),
                        p,
                        &direction,
                        b,
                    )
                })?
            };
            match result {
                None => continue,
                Some(true) => return Ok(true),
                Some(false) => break,
            }
        }
    }
    Ok(false)
}
fn visible(
    face: &Facet,
    sample: &Point2,
    originals: &[Facet],
    owners: &BTreeMap<u32, ComponentBounds>,
    index: &mut OccupancyIndex<'_>,
    coplanar: &[&Facet],
    b: &mut Budget,
) -> Result<bool, Failure> {
    let p = face.lift(sample, b)?;
    if occupied(&p, face.normal(), originals, owners, index, b)? {
        return Ok(false);
    }
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

fn node_cell(
    face: &Facet,
    cell: &Cell,
    all_points: &BTreeSet<Point3>,
    b: &mut Budget,
) -> Result<Cell, Failure> {
    let mut result = Vec::new();
    for i in 0..cell.len() {
        b.state([cell.len(), all_points.len(), result.len(), 0, i, 0, 0, 0]);
        let a = face.lift(&cell[i], b)?;
        let c = face.lift(&cell[(i + 1) % cell.len()], b)?;
        let mut cuts = BTreeSet::new();
        for p in all_points {
            b.state([
                cell.len(),
                all_points.len(),
                result.len(),
                cuts.len(),
                i,
                0,
                0,
                0,
            ]);
            if on_segment(&a, &c, p, b)? {
                b.guard(16 * (result.len() + cuts.len() + cell.len() + 1))?;
                log_cost(cuts.len(), b)?;
                cuts.insert(p.clone());
            }
        }
        let mut cuts: Vec<_> = cuts.into_iter().collect();
        if c < a {
            cuts.reverse();
        }
        for p in cuts.iter().take(cuts.len().saturating_sub(1)) {
            result.push(project(p, face.axis()));
        }
    }
    if result.len() < 3 {
        return Err(b.fail(FailureKind::InvalidWitness, "noding lost cell boundary"));
    }
    Ok(result)
}
fn triangulate(mut cell: Cell, b: &mut Budget) -> Result<Vec<[Point2; 3]>, Failure> {
    let expected = area(&cell, b)?;
    let mut result = Vec::new();
    while cell.len() > 3 {
        b.state([cell.len(), result.len(), 0, 0, 0, 0, 0, 0]);
        b.guard(16 * (cell.len() + result.len() + 1))?;
        let mut ear = None;
        for i in 0..cell.len() {
            b.step(1)?;
            let a = &cell[(i + cell.len() - 1) % cell.len()];
            let c = &cell[i];
            let d = &cell[(i + 1) % cell.len()];
            if orient(a, c, d, b)? <= integer(0) {
                continue;
            }
            let mut occupied = false;
            for p in &cell {
                if p == a || p == c || p == d {
                    continue;
                }
                if orient(a, c, p, b)? >= integer(0)
                    && orient(c, d, p, b)? >= integer(0)
                    && orient(d, a, p, b)? >= integer(0)
                {
                    occupied = true;
                    break;
                }
            }
            if !occupied {
                ear = Some(i);
                break;
            }
        }
        let Some(i) = ear else {
            return Err(b.fail(
                FailureKind::InvalidWitness,
                "no vertex-preserving positive ear",
            ));
        };
        result.push([
            cell[(i + cell.len() - 1) % cell.len()].clone(),
            cell[i].clone(),
            cell[(i + 1) % cell.len()].clone(),
        ]);
        b.step(cell.len() as u64)?;
        cell.remove(i);
    }
    let last = [cell[0].clone(), cell[1].clone(), cell[2].clone()];
    if area(&last, b)? <= integer(0) {
        return Err(b.fail(FailureKind::Degenerate, "degenerate final ear"));
    }
    result.push(last);
    let mut actual = integer(0);
    for triangle in &result {
        actual += area(triangle, b)?;
    }
    if actual != expected {
        return Err(b.fail(FailureKind::InvalidWitness, "triangulation area mismatch"));
    }
    Ok(result)
}
fn protected_facet_link(
    sectors: &[[Point2; 2]],
    boundary: bool,
    b: &mut Budget,
) -> Result<(), Failure> {
    // Protected physical intersections are retained by the reference complex.
    // A single original facet can contribute several disjoint boundary paths at
    // one such point. Each path must be regular, and the final 3D mesh must still
    // have one connected cycle there.
    b.with_retained(24 * sectors.len() + 64, |budget| {
        let mut incident: BTreeMap<Point2, Vec<usize>> = BTreeMap::new();
        for (index, sector) in sectors.iter().enumerate() {
            for endpoint in sector {
                budget.step(1)?;
                log_cost(incident.len(), budget)?;
                incident.entry(endpoint.clone()).or_default().push(index);
            }
        }
        let mut seen = BTreeSet::new();
        for seed in 0..sectors.len() {
            budget.step(1)?;
            log_cost(seen.len(), budget)?;
            if seen.contains(&seed) {
                continue;
            }
            let mut pending = vec![seed];
            let mut component = Vec::new();
            while let Some(index) = pending.pop() {
                budget.step(1)?;
                log_cost(seen.len(), budget)?;
                if !seen.insert(index) {
                    continue;
                }
                component.push(sectors[index].clone());
                for endpoint in &sectors[index] {
                    log_cost(incident.len(), budget)?;
                    for next in &incident[endpoint] {
                        budget.step(1)?;
                        if !seen.contains(next) {
                            pending.push(*next);
                        }
                    }
                }
            }
            connected_link(&component, !boundary, budget)?;
        }
        Ok(())
    })
}

fn facet_stars(
    triangles: &[[Point2; 3]],
    protected: &BTreeSet<Point2>,
    b: &mut Budget,
) -> Result<Vec<ContactStar>, Failure> {
    b.state([triangles.len(), protected.len(), 0, 0, 0, 0, 0, 0]);
    b.guard(64 * triangles.len() + 64)?;
    let mut stars: BTreeMap<Point2, Vec<[Point2; 2]>> = BTreeMap::new();
    let mut incidence: BTreeMap<[Point2; 2], Vec<bool>> = BTreeMap::new();
    for triangle in triangles {
        if area(triangle, b)? <= integer(0) {
            return Err(b.fail(FailureKind::InvalidWitness, "nonpositive facet triangle"));
        }
        for i in 0..3 {
            b.step(3)?;
            log_cost(incidence.len() + stars.len(), b)?;
            let a = &triangle[i];
            let c = &triangle[(i + 1) % 3];
            let d = &triangle[(i + 2) % 3];
            stars
                .entry(a.clone())
                .or_default()
                .push([d.clone(), c.clone()]);
            incidence.entry(ordered_edge(a, c)).or_default().push(a < c);
        }
    }
    let mut boundary = BTreeSet::new();
    for ([a, c], entries) in incidence {
        b.step(1)?;
        match entries.as_slice() {
            [_] => {
                boundary.insert(a);
                boundary.insert(c);
            }
            [first, second] if first != second => {}
            _ => return Err(b.fail(FailureKind::InvalidWitness, "facet edge incidence overlaps")),
        }
    }
    let mut result = Vec::new();
    for (point, sectors) in stars {
        let path = boundary.contains(&point);
        b.step(1)?;
        log_cost(protected.len(), b)?;
        if protected.contains(&point) {
            protected_facet_link(&sectors, path, b)?;
        } else {
            connected_link(&sectors, !path, b)?;
        }
        result.push(ContactStar {
            point,
            sectors,
            boundary_path: path,
        });
    }
    Ok(result)
}

// These records are part of the unchanged 40N+64 original-input reservation.
// Source IDs have two scalar fields plus one set membership; owner IDs have
// one scalar plus one membership. The original 64 header/padding slots stay.
struct InputIdentityMaps {
    ids: BTreeSet<SourceId>,
    owners: BTreeSet<u32>,
}
impl InputIdentityMaps {
    fn release(self, b: &mut Budget) -> usize {
        let members = 3 * self.ids.len() + 2 * self.owners.len();
        // Deallocate actual owning sets before releasing their reservation.
        drop(self);
        b.release(members);
        members
    }
}

// Strict projected-box separation proves that the original convex-domain
// clip returns None. Equality remains eligible for every contact obligation.
// The input segment reservations remain live until retain has dropped records.
fn prune_impossible_barriers(
    face: &Facet,
    bounds: &Bounds,
    barriers: &mut Vec<crate::Segment>,
    b: &mut Budget,
) -> Result<(), Failure> {
    b.with_retained(16, |b| {
        let lower = project(&bounds.lower, face.axis());
        let upper = project(&bounds.upper, face.axis());
        let before = barriers.len();
        // Both coordinate visits execute for every segment; admission precedes
        // mutation so an operation refusal preserves the original vector.
        b.step((2 * before) as u64)?;
        barriers.retain(|[a, c]| {
            let x = a[0] < lower[0] && c[0] < lower[0] || a[0] > upper[0] && c[0] > upper[0];
            let y = a[1] < lower[1] && c[1] < lower[1] || a[1] > upper[1] && c[1] > upper[1];
            !x && !y
        });
        b.release(8 * (before - barriers.len()));
        Ok(())
    })
}

/// Direct original-component adapter. Inputs are transformed original triangles,
/// including all disconnected sheets of each component. Every output triangle
/// retains one original facet and its complete affine field.
pub fn reconstruct(originals: &[Facet], b: &mut Budget) -> Result<Hull, Failure> {
    b.with_phase(BudgetPhase::InputAudit, |b| {
        reconstruct_records(originals, b)
    })
}
fn reconstruct_records(originals: &[Facet], b: &mut Budget) -> Result<Hull, Failure> {
    if originals.is_empty() {
        return Err(b.fail(FailureKind::InvalidInput, "missing original components"));
    }
    b.guard(originals.len())?;
    let mut input_members = Facet::input_memberships(originals, b)?;
    b.retain(input_members)?;
    let mut ids = BTreeSet::new();
    let mut owners = BTreeSet::new();
    for face in originals {
        b.at(face.id());
        b.step(1)?;
        log_cost(ids.len(), b)?;
        if !ids.insert(face.id()) {
            return Err(b.fail(
                FailureKind::InvalidInput,
                "duplicate original facet identity",
            ));
        }
        owners.insert(face.id().component);
    }
    b.with_phase(BudgetPhase::InputAudit, |b| {
        audit_original_owner_ranges(originals, &owners, b)
    })?;
    b.with_phase(BudgetPhase::InputAudit, |b| {
        audit_original_sheets(originals, b)
    })?;
    // One owner map plus the two transient facet boxes. Source audit maps
    // have already been released, and this map releases before final noding.
    let bounds_members = 11 * owners.len() + 32;
    // Both authenticated identity sets have reached their last use. Their
    // original geometry/style records remain admitted for all later phases.
    input_members -= InputIdentityMaps { ids, owners }.release(b);
    b.phase(BudgetPhase::Index);
    b.state([
        input_members,
        bounds_members,
        originals.len(),
        0,
        0,
        0,
        0,
        0,
    ]);
    b.retain(bounds_members)?;
    let facet_bounds_members = FacetBoundsIndex::cell_memberships(originals.len());
    b.retain(facet_bounds_members)?;
    let facet_bounds = b.with_phase(BudgetPhase::Index, |b| {
        FacetBoundsIndex::build_cells(originals, b)
    })?;
    let owner_bounds = b.with_phase(BudgetPhase::Index, |b| {
        component_bounds(originals, &facet_bounds.facets, b)
    })?;
    let interior = b.with_phase(BudgetPhase::InputAudit, |b| {
        strict_interior::InteriorCertificates::build_sections(originals, &owner_bounds, b)
    })?;
    let plane_groups = b.with_phase(BudgetPhase::Index, |b| {
        build_plane_groups(originals, &owner_bounds, b)
    })?;
    let plane_members = plane_groups.as_ref().map_or(0, PlaneGroups::memberships);
    let mut ray_caches = RayCaches::new(b)?;
    let mut occupancy_index = OccupancyIndex {
        spatial: &facet_bounds,
        planes: plane_groups.as_ref(),
        caches: &mut ray_caches,
    };
    let mut regions = Vec::new();
    let mut all_points = BTreeSet::new();
    let mut point_members = 0;
    let mut held = 0usize;
    for (face, face_bounds) in originals.iter().zip(&facet_bounds.facets) {
        b.phase(BudgetPhase::Sections);
        b.state([input_members, point_members, held, 0, 0, 0, 0, 0]);
        b.at(face.id());
        if let Some(certificates) = &interior
            && certificates.strictly_covers(face, face_bounds, b)?
        {
            // Keep the authenticated source in every other facet's queries.
            // Only this wholly interior facet's own empty publication is omitted.
            continue;
        }
        let mut domain = face.domain().to_vec();
        if area(&domain, b)?.is_negative() {
            domain.reverse();
        }
        let rows = convex_sections::Rows::build(face, face_bounds, interior.as_ref(), b)?;
        let (mut barriers, coplanar, sections) = b.with_phase(BudgetPhase::Sections, |b| {
            b.with_retained(16, |local| {
                sections::Frame::collect_rows(
                    face,
                    face_bounds,
                    originals,
                    &owner_bounds,
                    &facet_bounds,
                    &rows,
                    local,
                )
            })
        })?;
        prune_impossible_barriers(face, face_bounds, &mut barriers, b)?;
        let parity_index = sections::ParityIndex::build(&sections, b)?;
        // Transfer barrier ownership to the arrangement's existing input bound.
        // Its coplanar scope below admits those references for classification.
        b.release(8 * barriers.len() + coplanar.len());
        let section_members = sections.memberships();
        let arrangement_result = b.with_phase(BudgetPhase::ArrangementSetup, |b| {
            b.with_retained(coplanar.len(), |budget| {
                let result =
                    Arrangement::build_source(&domain, &barriers, budget, |sample, local| {
                        match local.with_phase_state(
                            BudgetPhase::Occupancy,
                            local.state_values(),
                            |b| match rows.occupied(sample, b)? {
                                Some(false) => match &parity_index {
                                    Some(index) => index.occupied(sample, b),
                                    None => sections.occupied(sample, b),
                                },
                                decisive => Ok(decisive),
                            },
                        )? {
                            Some(true) => Ok(false),
                            Some(false) => local.with_phase_state(
                                BudgetPhase::Fields,
                                local.state_values(),
                                |b| sections::fields_visible(face, sample, &coplanar, b),
                            ),
                            // Keep the independent generic ray policy for a boundary
                            // sample. The accounted callback admits its live scratch.
                            None => local.with_phase_state(
                                BudgetPhase::Occupancy,
                                local.state_values(),
                                |local| {
                                    visible(
                                        face,
                                        sample,
                                        originals,
                                        &owner_bounds,
                                        &mut occupancy_index,
                                        &coplanar,
                                        local,
                                    )
                                },
                            ),
                        }
                    });
                drop(coplanar);
                result
            })
        });
        let parity_members = parity_index.as_ref().map_or(0, |index| index.memberships());
        drop(parity_index);
        b.release(parity_members);
        drop(sections);
        b.release(section_members);
        let row_members = rows.memberships();
        drop(rows);
        b.release(row_members);
        let arrangement = arrangement_result?;
        drop(barriers);
        // The original witness publisher retains no record for an invisible
        // facet. Its authenticated original triangle remains in `originals`.
        if arrangement.cells.is_empty() {
            continue;
        }
        b.phase(BudgetPhase::GlobalPoints);
        b.state([
            point_members,
            held,
            arrangement.cells.len(),
            arrangement.physical.len(),
            arrangement.protected.len(),
            0,
            0,
            0,
        ]);
        let reserve = PackedRegion::raw_memberships(
            &arrangement.cells,
            arrangement.physical.len(),
            arrangement.protected.len(),
        );
        b.retain(reserve)?;
        let region = b.with_phase(BudgetPhase::Packing, |b| PackedRegion::pack(arrangement, b))?;
        b.release(reserve);
        held += region.memberships();
        regions.push((face, region));
    }
    let interior_members = interior
        .as_ref()
        .map_or(0, |certificates| certificates.memberships());
    drop(interior);
    b.release(interior_members);
    drop(owner_bounds);
    drop(facet_bounds);
    drop(plane_groups);
    let ray_members = ray_caches.memberships();
    drop(ray_caches);
    b.release(bounds_members + facet_bounds_members + plane_members + ray_members);
    // The complete noding pool is needed only after all source-plane sections
    // and indexes are dropped. Read precisely the packed cells/protected set;
    // physical-only points do not enlarge the old pool.
    for (face, region) in &regions {
        b.at(face.id());
        b.phase(BudgetPhase::GlobalPoints);
        region.visit_noding_points(b, |p, b| {
            let p = face.lift(p, b)?;
            b.state([point_members, held, all_points.len(), 0, 0, 0, 0, 0]);
            b.retain(8)?;
            log_cost(all_points.len(), b)?;
            if all_points.insert(p) {
                point_members += 8;
            } else {
                b.release(8);
            }
            Ok(())
        })?;
    }
    let mut triangles = Vec::new();
    let mut witnesses = Vec::new();
    for (source, region) in regions {
        b.phase(BudgetPhase::Noding);
        b.state([
            input_members,
            point_members,
            held,
            region.memberships(),
            region.expanded_memberships(),
            0,
            0,
            0,
        ]);
        b.at(source.id());
        let packed_members = region.memberships();
        let expanded_members = region.expanded_memberships();
        b.retain(expanded_members)?;
        let arrangement = region.expand(b)?;
        let _work = arrangement.work;
        b.release(packed_members);
        held -= packed_members;
        let mut planar = Vec::new();
        let mut expected = integer(0);
        for cell in &arrangement.cells {
            expected += area(cell, b)?;
            let noded = b.with_phase(BudgetPhase::Noding, |b| {
                node_cell(source, cell, &all_points, b)
            })?;
            let new = b.with_phase(BudgetPhase::Triangulation, |b| triangulate(noded, b))?;
            b.guard(32 * (planar.len() + new.len() + 1))?;
            planar.extend(new);
        }
        let stars = b.with_phase(BudgetPhase::ContactStars, |b| {
            facet_stars(&planar, &arrangement.protected, b)
        })?;
        // Every exposed protected point must survive in the actual output mesh.
        for p in &arrangement.protected {
            let mut in_region = false;
            for cell in &arrangement.cells {
                if contains_convex(cell, p, b)? {
                    in_region = true;
                    break;
                }
            }
            if in_region && !planar.iter().flatten().any(|q| q == p) {
                return Err(b.fail(
                    FailureKind::InvalidWitness,
                    "lost physical contact endpoint",
                ));
            }
        }
        b.phase(BudgetPhase::Witness);
        b.state([
            expanded_members,
            planar.len(),
            stars.len(),
            arrangement.cells.len(),
            arrangement.physical.len(),
            arrangement.protected.len(),
            0,
            0,
        ]);
        let witness_members = incoming_witness_memberships(
            &arrangement.cells,
            arrangement.physical.len(),
            arrangement.protected.len(),
            planar.len(),
            &stars,
            b,
        )?;
        // Admit final ownership before transferring expanded region records.
        // Raw expansion remains admitted through the complete move/collection.
        let geometry_members = (32 + source.style().len()) * planar.len();
        b.retain(geometry_members + witness_members)?;
        for triangle in &planar {
            if triangles.len() >= b.limits().triangles {
                return Err(b.fail(
                    FailureKind::TriangleLimit,
                    "complete candidate exceeds original triangle cap",
                ));
            }
            let make = |p: &Point2, b: &mut Budget| -> Result<Vertex, Failure> {
                let p = source.lift(p, b)?;
                let uv = source.uv_at(&p, b)?;
                Ok(Vertex { point: p, uv })
            };
            let mut vertices = [
                make(&triangle[0], b)?,
                make(&triangle[1], b)?,
                make(&triangle[2], b)?,
            ];
            if dot(
                &cross(
                    &sub(&vertices[1].point, &vertices[0].point),
                    &sub(&vertices[2].point, &vertices[0].point),
                ),
                source.normal(),
            )
            .is_negative()
            {
                vertices.swap(1, 2);
            }
            triangles.push(Facet::new(
                source.id(),
                vertices,
                source.style().to_owned(),
                b,
            )?);
        }
        let witness = FacetWitness {
            source: source.id(),
            classified_cells: arrangement.cells,
            physical_segments: arrangement.physical,
            protected_points: arrangement.protected.into_iter().collect(),
            output_triangles: planar,
            contact_stars: stars,
            twice_area: expected,
        };
        b.release(expanded_members);
        // The final witness is already retained above. Public standalone verify
        // reserves independently owned input; use the admitted path here.
        b.with_phase(BudgetPhase::Witness, |b| witness.verify_admitted(source, b))?;
        witnesses.push(witness);
    }
    if triangles.is_empty() {
        return Err(b.fail(FailureKind::InvalidWitness, "lost entire original exterior"));
    }
    b.with_phase(BudgetPhase::Topology, |b| {
        audit_closed(triangles.iter(), false, b)
    })?;
    let mut volume = integer(0);
    for face in &triangles {
        b.at(face.id());
        b.step(1)?;
        volume += dot(
            &face.vertices()[0].point,
            &cross(&face.vertices()[1].point, &face.vertices()[2].point),
        ) / integer(6);
    }
    if volume <= integer(0) {
        return Err(b.fail(
            FailureKind::NonManifold,
            "candidate has nonpositive signed volume",
        ));
    }
    b.release(held + point_members + input_members);
    Ok(Hull {
        triangles,
        witnesses,
        volume,
    })
}
fn contains_convex(cell: &[Point2], p: &Point2, b: &mut Budget) -> Result<bool, Failure> {
    for i in 0..cell.len() {
        if orient(&cell[i], &cell[(i + 1) % cell.len()], p, b)?.is_negative() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn incoming_witness_memberships(
    cells: &[Cell],
    physical: usize,
    protected: usize,
    triangles: usize,
    stars: &[ContactStar],
    b: &mut Budget,
) -> Result<usize, Failure> {
    // Two source identity fields, area and five owning vector memberships.
    // Each point includes its containing membership and both coordinates.
    let mut total = 8usize;
    let mut add = |count: usize, width: usize, budget: &mut Budget| -> Result<(), Failure> {
        budget.state([total, count, width, 0, 0, 0, 0, 0]);
        budget.step(1)?;
        total = count
            .checked_mul(width)
            .and_then(|value| total.checked_add(value))
            .ok_or_else(|| {
                budget.fail(FailureKind::Storage, "witness membership count overflow")
            })?;
        budget.guard(total)
    };
    add(cells.len(), 1, b)?;
    for cell in cells {
        add(cell.len(), 3, b)?;
    }
    add(physical, 7, b)?;
    add(protected, 3, b)?;
    add(triangles, 10, b)?;
    add(stars.len(), 6, b)?;
    for star in stars {
        add(star.sectors.len(), 7, b)?;
    }
    Ok(total)
}

impl FacetWitness {
    /// Recheck the actual region, contact intervals and vertex links before publication.
    pub fn verify(&self, source: &Facet, b: &mut Budget) -> Result<(), Failure> {
        b.with_phase(BudgetPhase::Witness, |b| {
            b.at(source.id());
            let members = incoming_witness_memberships(
                &self.classified_cells,
                self.physical_segments.len(),
                self.protected_points.len(),
                self.output_triangles.len(),
                &self.contact_stars,
                b,
            )?;
            b.with_retained(members, |budget| self.verify_admitted(source, budget))
        })
    }

    fn verify_admitted(&self, source: &Facet, b: &mut Budget) -> Result<(), Failure> {
        if self.source != source.id() {
            return Err(b.fail(
                FailureKind::InvalidWitness,
                "witness source identity mismatch",
            ));
        }
        let mut expected = integer(0);
        for cell in &self.classified_cells {
            if area(cell, b)? <= integer(0) {
                return Err(b.fail(FailureKind::InvalidWitness, "nonpositive classified cell"));
            }
            for i in 0..cell.len() {
                if cell[i] == cell[(i + 1) % cell.len()] {
                    return Err(b.fail(FailureKind::InvalidWitness, "duplicate classified corner"));
                }
                for p in cell {
                    if orient(&cell[i], &cell[(i + 1) % cell.len()], p, b)?.is_negative() {
                        return Err(b.fail(
                            FailureKind::InvalidWitness,
                            "classified witness is not convex",
                        ));
                    }
                }
            }
            for p in cell {
                point(p, b)?;
                source.uv_at(&source.lift(p, b)?, b)?;
            }
            expected += area(cell, b)?;
        }
        if expected != self.twice_area {
            return Err(b.fail(
                FailureKind::InvalidWitness,
                "classified witness area mismatch",
            ));
        }
        let mut overlap = integer(0);
        let mut actual = integer(0);
        for (i, triangle) in self.output_triangles.iter().enumerate() {
            for p in triangle {
                point(p, b)?;
                source.uv_at(&source.lift(p, b)?, b)?;
            }
            let measure = area(triangle, b)?;
            if measure <= integer(0) {
                return Err(b.fail(FailureKind::InvalidWitness, "nonpositive witness output"));
            }
            actual += measure;
            for cell in &self.classified_cells {
                overlap += intersection_area(triangle, cell, b)?;
            }
            for other in &self.output_triangles[i + 1..] {
                if !intersection_area(triangle, other, b)?.is_zero() {
                    return Err(b.fail(FailureKind::InvalidWitness, "overlapping output regions"));
                }
            }
        }
        for (i, cell) in self.classified_cells.iter().enumerate() {
            for other in &self.classified_cells[i + 1..] {
                if !intersection_area(cell, other, b)?.is_zero() {
                    return Err(b.fail(
                        FailureKind::InvalidWitness,
                        "overlapping classified regions",
                    ));
                }
            }
        }
        if actual != expected || overlap != expected {
            return Err(b.fail(
                FailureKind::InvalidWitness,
                "incomplete mutual exact region coverage",
            ));
        }
        let stars = b.with_retained(3 * self.protected_points.len(), |budget| {
            let mut protected = BTreeSet::new();
            for point in &self.protected_points {
                budget.step(1)?;
                log_cost(protected.len(), budget)?;
                protected.insert(point.clone());
            }
            facet_stars(&self.output_triangles, &protected, budget)
        })?;
        if stars != self.contact_stars {
            return Err(b.fail(FailureKind::InvalidWitness, "contact-star witness mismatch"));
        }
        for p in &self.protected_points {
            point(p, b)?;
            source.uv_at(&source.lift(p, b)?, b)?;
            for cell in &self.classified_cells {
                if contains_convex(cell, p, b)?
                    && !self.output_triangles.iter().flatten().any(|q| q == p)
                {
                    return Err(b.fail(FailureKind::InvalidWitness, "missing protected point"));
                }
            }
        }
        for segment in &self.physical_segments {
            for p in segment {
                point(p, b)?;
                source.uv_at(&source.lift(p, b)?, b)?;
            }
            for cell in &self.classified_cells {
                if let Some(clipped) = clip_interval(segment, cell, b)? {
                    verify_interval(&clipped, &self.output_triangles, b)?;
                }
            }
        }
        Ok(())
    }
}
fn intersection_area(
    first: &[Point2],
    second: &[Point2],
    b: &mut Budget,
) -> Result<Rational, Failure> {
    b.guard(16 * (first.len() + second.len() + 8))?;
    let mut current = first.to_vec();
    for i in 0..second.len() {
        let a = &second[i];
        let c = &second[(i + 1) % second.len()];
        let mut next = Vec::new();
        for j in 0..current.len() {
            b.step(1)?;
            let p = &current[j];
            let q = &current[(j + 1) % current.len()];
            let x = orient(a, c, p, b)?;
            let y = orient(a, c, q, b)?;
            if x >= integer(0) {
                next.push(p.clone());
            }
            if x.is_negative() != y.is_negative() {
                let t = &x / (&x - &y);
                let hit = add(p, &scale(&sub(q, p), &t));
                point(&hit, b)?;
                next.push(hit);
            }
            b.guard(16 * (first.len() + second.len() + current.len() + next.len() + 8))?;
        }
        current = next;
        if current.len() < 3 {
            return Ok(integer(0));
        }
    }
    Ok(area(&current, b)?.abs())
}
fn clip_interval(
    segment: &[Point2; 2],
    cell: &[Point2],
    b: &mut Budget,
) -> Result<Option<[Point2; 2]>, Failure> {
    let [a, c] = segment;
    let mut low = integer(0);
    let mut high = integer(1);
    for i in 0..cell.len() {
        let x = orient(&cell[i], &cell[(i + 1) % cell.len()], a, b)?;
        let y = orient(&cell[i], &cell[(i + 1) % cell.len()], c, b)?;
        if x.is_negative() && y.is_negative() {
            return Ok(None);
        }
        if x.is_negative() {
            low = low.max(&x / (&x - &y));
        } else if y.is_negative() {
            high = high.min(&x / (&x - &y));
        }
    }
    if low > high {
        return Ok(None);
    }
    let delta = sub(c, a);
    Ok(Some([
        add(a, &scale(&delta, &low)),
        add(a, &scale(&delta, &high)),
    ]))
}
fn verify_interval(
    segment: &[Point2; 2],
    triangles: &[[Point2; 3]],
    b: &mut Budget,
) -> Result<(), Failure> {
    let [a, c] = segment;
    if a == c {
        if triangles.iter().flatten().any(|p| p == a) {
            return Ok(());
        }
        return Err(b.fail(
            FailureKind::InvalidWitness,
            "lost isolated physical contact",
        ));
    }
    let axis = usize::from(a[0] == c[0]);
    let lo = a[axis].clone().min(c[axis].clone());
    let hi = a[axis].clone().max(c[axis].clone());
    let mut intervals = Vec::new();
    b.guard(16 * (triangles.len() + 1))?;
    for triangle in triangles {
        for i in 0..3 {
            let p = &triangle[i];
            let q = &triangle[(i + 1) % 3];
            if orient(a, c, p, b)?.is_zero() && orient(a, c, q, b)?.is_zero() {
                intervals.push([
                    p[axis].clone().min(q[axis].clone()),
                    p[axis].clone().max(q[axis].clone()),
                ]);
            }
        }
    }
    // Charged insertion sort retains deterministic ordering without uncharged comparisons.
    for i in 1..intervals.len() {
        let mut j = i;
        while j > 0 {
            b.step(1)?;
            if intervals[j - 1] <= intervals[j] {
                break;
            }
            intervals.swap(j - 1, j);
            j -= 1;
        }
    }
    let mut end = lo;
    for [left, right] in intervals {
        b.step(1)?;
        if right < end {
            continue;
        }
        if left > end {
            break;
        }
        end = end.max(right);
        if end >= hi {
            return Ok(());
        }
    }
    Err(b.fail(
        FailureKind::InvalidWitness,
        "physical segment lacks complete output-edge coverage",
    ))
}

fn audit_original_sheets(faces: &[Facet], b: &mut Budget) -> Result<(), Failure> {
    for (i, first) in faces.iter().enumerate() {
        for second in &faces[i + 1..] {
            if first.id().component != second.id().component {
                continue;
            }
            b.step(1)?;
            let separated = (0..3).any(|k| {
                let a = first.vertices().each_ref().map(|v| v.point[k].clone());
                let c = second.vertices().each_ref().map(|v| v.point[k].clone());
                a.iter().max() < c.iter().min() || c.iter().max() < a.iter().min()
            });
            if separated {
                continue;
            }
            let parallel = cross(first.normal(), second.normal())
                .iter()
                .all(Zero::is_zero);
            if parallel {
                if !first.distance(&second.vertices()[0].point).is_zero() {
                    continue;
                }
                let mut a = first.domain().to_vec();
                let mut c = second
                    .vertices()
                    .iter()
                    .map(|v| project(&v.point, first.axis()))
                    .collect::<Vec<_>>();
                if area(&a, b)?.is_negative() {
                    a.reverse();
                }
                if area(&c, b)?.is_negative() {
                    c.reverse();
                }
                if !intersection_area(&a, &c, b)?.is_zero() {
                    return Err(b.fail(
                        FailureKind::NonManifold,
                        "overlapping coplanar source sheets",
                    ));
                }
                continue;
            }
            for (a, c) in [(first, second), (second, first)] {
                for edge in 0..3 {
                    let p = &a.vertices()[edge].point;
                    let q = &a.vertices()[(edge + 1) % 3].point;
                    let x = c.distance(p);
                    let y = c.distance(q);
                    let hit = if x.is_zero() {
                        Some(p.clone())
                    } else if (&x * &y).is_negative() {
                        Some(add(p, &scale(&sub(q, p), &(&x / (&x - &y)))))
                    } else {
                        None
                    };
                    if let Some(hit) = hit {
                        point(&hit, b)?;
                        if c.contains(&hit, b)?.0
                            && !(first.vertices().iter().any(|v| v.point == hit)
                                && second.vertices().iter().any(|v| v.point == hit))
                        {
                            return Err(b.fail(
                                FailureKind::NonManifold,
                                "source sheets cross away from shared vertices",
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_overlap_facet_keeps_each_regular_local_boundary_path() -> Result<(), Failure> {
        // Frozen analytic overlap (0..2)^3 union (1..3)^3. Original facet9
        // on component0's y=2 face projects to x+z>=2 in [0,2]^2.
        // Removing x>1,z>1 leaves these two positive regions meeting at (1,1).
        let p = |x, y| [integer(x), integer(y)];
        let contact = p(1, 1);
        let triangles = [
            [p(0, 2), contact.clone(), p(1, 2)],
            [contact.clone(), p(2, 0), p(2, 1)],
        ];
        let mut b = Budget::new(crate::Limits::default())?;
        let protected = BTreeSet::from([contact.clone()]);
        let stars = facet_stars(&triangles, &protected, &mut b)?;
        let star = stars
            .iter()
            .find(|s| s.point == contact)
            .ok_or_else(|| b.fail(FailureKind::InvalidWitness, "missing regression contact"))?;
        assert_eq!(star.sectors.len(), 2);
        assert!(star.boundary_path);
        assert_eq!(
            area(&triangles[0], &mut b)? + area(&triangles[1], &mut b)?,
            integer(2)
        );
        assert_eq!(
            facet_stars(&triangles, &BTreeSet::new(), &mut b)
                .err()
                .map(|e| e.kind),
            Some(FailureKind::UncertifiedContact)
        );
        // Complete 3D links retain the original single-cycle requirement.
        assert_eq!(
            connected_link(&star.sectors, true, &mut b)
                .err()
                .map(|e| e.kind),
            Some(FailureKind::UncertifiedContact)
        );
        Ok(())
    }
    fn tetrahedron(owner: u32, first_facet: u32, reflected: bool) -> Result<Vec<Facet>, Failure> {
        let mut setup = Budget::new(crate::Limits::default())?;
        let sign = if reflected { -1 } else { 1 };
        let points = [[0, 0, 0], [sign, 0, 0], [0, sign, 0], [0, 0, sign]].map(|p| p.map(integer));
        [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]
            .into_iter()
            .enumerate()
            .map(|(i, mut indices)| {
                if reflected {
                    indices.swap(1, 2);
                }
                Facet::new(
                    SourceId {
                        component: owner,
                        facet: first_facet + i as u32,
                    },
                    indices.map(|j| Vertex {
                        point: points[j].clone(),
                        uv: [integer(0), integer(0)],
                    }),
                    "t".into(),
                    &mut setup,
                )
            })
            .collect()
    }

    #[test]
    fn original_topology_streams_owner_maps_under_the_same_storage_guard() -> Result<(), Failure> {
        let mut faces = Vec::new();
        let owners = (0..20).collect::<BTreeSet<_>>();
        for owner in &owners {
            faces.extend(tetrahedron(*owner, 0, false)?);
        }
        let input_members =
            40 * faces.len() + 64 + faces.iter().map(|f| f.style().len()).sum::<usize>();
        let limits = crate::Limits {
            storage_memberships: 4000,
            ..crate::Limits::default()
        };
        let mut combined = Budget::new(limits)?;
        combined.retain(input_members)?;
        assert_eq!(
            audit_closed(faces.iter(), true, &mut combined)
                .err()
                .map(|f| f.kind),
            Some(FailureKind::Storage)
        );
        assert_eq!(combined.peak_admitted_memberships(), input_members);
        assert_eq!(combined.retained_memberships(), input_members);
        assert!(combined.peak_memberships() > limits.storage_memberships);

        let mut streamed = Budget::new(limits)?;
        streamed.retain(input_members)?;
        audit_original_components(&faces, &owners, &mut streamed)?;
        assert!(streamed.peak_memberships() <= limits.storage_memberships);
        assert_eq!(
            streamed.peak_admitted_memberships(),
            streamed.peak_memberships()
        );
        assert_eq!(streamed.retained_memberships(), input_members);

        // The last component is still audited and can refuse without leaking the group reservation.
        faces.pop();
        let mut open = Budget::new(limits)?;
        open.retain(input_members)?;
        assert_eq!(
            audit_original_components(&faces, &owners, &mut open)
                .err()
                .map(|f| f.kind),
            Some(FailureKind::NonManifold)
        );
        assert_eq!(open.retained_memberships(), input_members);
        Ok(())
    }

    #[test]
    fn owner_streaming_keeps_all_sheets_of_one_component_in_one_link_audit() -> Result<(), Failure>
    {
        let mut faces = tetrahedron(0, 0, false)?;
        faces.extend(tetrahedron(0, 4, true)?);
        let owners = BTreeSet::from([0]);
        let mut b = Budget::new(crate::Limits::default())?;
        assert_eq!(
            audit_original_components(&faces, &owners, &mut b)
                .err()
                .map(|f| f.kind),
            Some(FailureKind::UncertifiedContact)
        );
        assert_eq!(b.retained_memberships(), 0);
        Ok(())
    }
    #[test]
    fn closed_component_bounds_retain_face_edge_and_point_equalities() -> Result<(), Failure> {
        let bounds = Bounds {
            lower: [integer(0), integer(0), integer(0)],
            upper: [integer(1), integer(1), integer(1)],
        };
        let mut b = Budget::new(crate::Limits::default())?;
        for point in [[1, 0, 0], [1, 1, 0], [1, 1, 1]] {
            assert!(bounds.contains(&point.map(integer), &mut b)?);
        }
        assert!(!bounds.contains(&[integer(2), integer(0), integer(0)], &mut b)?);
        for lower in [[1, 0, 0], [1, 1, 0], [1, 1, 1]] {
            let other = Bounds {
                lower: lower.map(integer),
                upper: lower.map(|v| integer(v + 1)),
            };
            assert!(bounds.meets(&other, &mut b)?);
        }
        let disjoint = Bounds {
            lower: [integer(2), integer(0), integer(0)],
            upper: [integer(3), integer(1), integer(1)],
        };
        assert!(!bounds.meets(&disjoint, &mut b)?);
        Ok(())
    }
    #[test]
    fn facet_bounds_index_matches_independent_closed_pairs_and_guards_scratch()
    -> Result<(), Failure> {
        let mut setup = Budget::new(crate::Limits::default())?;
        let mut faces = Vec::new();
        for ordinal in 0..65 {
            let base = [ordinal % 17 - 8, (ordinal / 17) % 3, ordinal % 5 - 2];
            let vertices = [[0, 0, 0], [1, 0, 0], [0, 1, 1]].map(|offset| Vertex {
                point: std::array::from_fn(|k| integer(base[k] + offset[k])),
                uv: [integer(0), integer(0)],
            });
            faces.push(Facet::new(
                SourceId {
                    component: 0,
                    facet: ordinal as u32,
                },
                vertices,
                String::new(),
                &mut setup,
            )?);
        }
        for count in [1, 7, 8, 9, 16, 17, 31, 32, 33, 49, 64, 65] {
            let members = FacetBoundsIndex::memberships(count);
            let scratch = FacetBoundsIndex::stack_members(count);
            let mut b = Budget::new(crate::Limits {
                storage_memberships: members + count + scratch,
                ..crate::Limits::default()
            })?;
            b.with_retained(members, |b| {
                let index = FacetBoundsIndex::build(&faces[..count], b)?;
                assert!(index.nodes.len() <= FacetBoundsIndex::node_limit(count));
                for (first, query) in faces[..count].iter().zip(&index.facets) {
                    let actual = b.with_retained(count, |b| index.candidates(query, b))?;
                    // Independent strict-separation oracle compares every corner
                    // pair, rather than using cached extrema or tree nodes.
                    let expected = faces[..count]
                        .iter()
                        .enumerate()
                        .filter_map(|(i, second)| {
                            let separated = (0..3).any(|k| {
                                first.vertices().iter().all(|a| {
                                    second.vertices().iter().all(|c| a.point[k] < c.point[k])
                                }) || second.vertices().iter().all(|c| {
                                    first.vertices().iter().all(|a| c.point[k] < a.point[k])
                                })
                            });
                            (!separated).then_some(i)
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(actual, expected);
                }
                Ok(())
            })?;
            assert_eq!(b.retained_memberships(), 0);
            assert_eq!(b.peak_admitted_memberships(), members + count + scratch);
        }
        let count = faces.len();
        let members = FacetBoundsIndex::memberships(count);
        let scratch = FacetBoundsIndex::stack_members(count);
        let mut short = Budget::new(crate::Limits {
            storage_memberships: members + count + scratch - 1,
            ..crate::Limits::default()
        })?;
        let result = short.with_retained(members, |b| {
            let index = FacetBoundsIndex::build(&faces, b)?;
            b.with_retained(count, |b| index.candidates(&index.facets[0], b))
        });
        assert_eq!(result.err().map(|e| e.kind), Some(FailureKind::Storage));
        assert_eq!(short.retained_memberships(), 0);
        assert!(short.peak_admitted_memberships() <= short.limits().storage_memberships);
        Ok(())
    }
    #[test]
    fn forward_ray_candidates_keep_boundary_ambiguities_and_source_order() -> Result<(), Failure> {
        let mut setup = Budget::new(crate::Limits::default())?;
        let mut faces = Vec::new();
        for ordinal in 0..65 {
            let base = [ordinal % 17 - 8, (ordinal / 17) % 3, ordinal % 5 - 2];
            let vertices = [[0, 0, 0], [1, 0, 0], [0, 1, 1]].map(|offset| Vertex {
                point: std::array::from_fn(|k| integer(base[k] + offset[k])),
                uv: [integer(0), integer(0)],
            });
            faces.push(Facet::new(
                SourceId {
                    component: ordinal as u32 % 3,
                    facet: ordinal as u32,
                },
                vertices,
                String::new(),
                &mut setup,
            )?);
        }
        let members = FacetBoundsIndex::memberships(faces.len()) + 11 * 3 + 32;
        let mut b = Budget::new(crate::Limits::default())?;
        b.with_retained(members, |b| {
            let index = FacetBoundsIndex::build(&faces, b)?;
            let owners = component_bounds(&faces, &index.facets, b)?;
            for origin in [[0, 0, 0], [3, 1, 2], [-8, 0, -2]] {
                for direction in [[1, 2, 3], [-1, -2, -3], [1, 0, -1], [0, 1, 0]] {
                    let origin = origin.map(integer);
                    let direction = direction.map(integer);
                    for (owner, group) in &owners {
                        assert_eq!(
                            group.facets,
                            faces.iter().filter(|f| f.id().component == *owner).count()
                        );
                        let actual = b.with_retained(faces.len(), |b| {
                            index.ray_candidates(
                                &origin,
                                &direction,
                                &faces,
                                *owner,
                                &group.bounds,
                                b,
                            )
                        })?;
                        // Independent corner predicate: every vertex must be behind
                        // the origin on one ray axis before that facet can disappear.
                        let expected = faces
                            .iter()
                            .enumerate()
                            .filter_map(|(i, face)| {
                                if face.id().component != *owner {
                                    return None;
                                }
                                let impossible = (0..3).any(|k| {
                                    let below =
                                        face.vertices().iter().all(|v| v.point[k] < origin[k]);
                                    let above =
                                        face.vertices().iter().all(|v| v.point[k] > origin[k]);
                                    if direction[k].is_positive() {
                                        below
                                    } else if direction[k].is_negative() {
                                        above
                                    } else {
                                        below || above
                                    }
                                });
                                (!impossible).then_some(i)
                            })
                            .collect::<Vec<_>>();
                        assert_eq!(actual, expected);
                        let mut flat = Budget::new(crate::Limits::default())?;
                        let expected_parity = ray_parity(
                            faces.iter().filter(|f| f.id().component == *owner),
                            &origin,
                            &direction,
                            &mut flat,
                        )?;
                        let actual_parity = ray_parity(
                            actual.into_iter().map(|i| &faces[i]),
                            &origin,
                            &direction,
                            b,
                        )?;
                        assert_eq!(actual_parity, expected_parity);
                    }
                }
            }
            Ok(())
        })?;
        assert_eq!(b.retained_memberships(), 0);
        let mut short = Budget::new(crate::Limits {
            storage_memberships: members
                + faces.len()
                + FacetBoundsIndex::stack_members(faces.len())
                - 1,
            ..crate::Limits::default()
        })?;
        let result = short.with_retained(members, |b| {
            let index = FacetBoundsIndex::build(&faces, b)?;
            let owners = component_bounds(&faces, &index.facets, b)?;
            let group = owners
                .get(&0)
                .ok_or_else(|| b.fail(FailureKind::InvalidWitness, "missing ray oracle owner"))?;
            b.with_retained(faces.len(), |b| {
                index.ray_candidates(
                    &[integer(0), integer(0), integer(0)],
                    &[integer(1), integer(2), integer(3)],
                    &faces,
                    0,
                    &group.bounds,
                    b,
                )
            })
        });
        assert_eq!(result.err().map(|e| e.kind), Some(FailureKind::Storage));
        assert_eq!(short.retained_memberships(), 0);
        assert!(short.peak_admitted_memberships() <= short.limits().storage_memberships);
        Ok(())
    }
    fn independent_closed_ray_box(face: &Facet, origin: &Point3, direction: &Point3) -> bool {
        let mut entries = Vec::new();
        let mut exits = Vec::new();
        for k in 0..3 {
            let mut lower = face.vertices()[0].point[k].clone();
            let mut upper = lower.clone();
            for vertex in &face.vertices()[1..] {
                lower = lower.min(vertex.point[k].clone());
                upper = upper.max(vertex.point[k].clone());
            }
            if direction[k].is_zero() {
                if origin[k] < lower || origin[k] > upper {
                    return false;
                }
            } else {
                let a = (lower - &origin[k]) / &direction[k];
                let c = (upper - &origin[k]) / &direction[k];
                let (entry, exit) = if direction[k].is_positive() {
                    (a, c)
                } else {
                    (c, a)
                };
                if exit < integer(0) {
                    return false;
                }
                entries.push(entry);
                exits.push(exit);
            }
        }
        // Independent all-pairs interval theorem, rather than the production
        // running max/min clipping or any cached node/leaf extrema.
        entries
            .iter()
            .all(|entry| exits.iter().all(|exit| entry <= exit))
    }
    #[test]
    fn closed_ray_intervals_reject_skew_boxes_and_preserve_ordered_exact_parity()
    -> Result<(), Failure> {
        let origin = [integer(0), integer(0), integer(0)];
        let mut b = Budget::new(crate::Limits::default())?;
        for (direction, lower, upper, expected) in [
            ([1, 1, 1], [1, 3, 1], [2, 4, 2], false),
            ([1, 1, 1], [1, 2, 1], [2, 3, 2], true),
            ([-1, -1, -1], [-2, -3, -2], [-1, -2, -1], true),
            ([1, 0, 1], [1, 0, 1], [2, 0, 2], true),
            ([1, 0, 1], [1, 1, 1], [2, 2, 2], false),
        ] {
            let bounds = Bounds {
                lower: lower.map(integer),
                upper: upper.map(integer),
            };
            let direction = direction.map(integer);
            assert_eq!(
                b.with_retained(32, |b| bounds.ray_intersects(&origin, &direction, b))?,
                expected
            );
        }
        let skew = Bounds {
            lower: [1, 3, 1].map(integer),
            upper: [2, 4, 2].map(integer),
        };
        assert!(skew.ray_may_meet(&origin, &[1, 1, 1].map(integer), &mut b)?);
        assert!(!skew.ray_intersects(&origin, &[1, 1, 1].map(integer), &mut b)?);
        let mut faces = Vec::new();
        for ordinal in 0..65 {
            let base = [ordinal % 17 - 8, (ordinal / 17) % 3, ordinal % 5 - 2];
            let vertices = [[0, 0, 0], [1, 0, 0], [0, 1, 1]].map(|offset| Vertex {
                point: std::array::from_fn(|k| integer(base[k] + offset[k])),
                uv: [integer(0), integer(0)],
            });
            faces.push(Facet::new(
                SourceId {
                    component: ordinal as u32 % 3,
                    facet: ordinal as u32,
                },
                vertices,
                String::new(),
                &mut b,
            )?);
        }
        let members = FacetBoundsIndex::memberships(faces.len()) + 11 * 3 + 32;
        b.with_retained(members, |b| {
            let index = FacetBoundsIndex::build(&faces, b)?;
            let owners = component_bounds(&faces, &index.facets, b)?;
            for origin in [[0, 0, 0], [3, 1, 2], [-8, 0, -2]] {
                for direction in [[1, 2, 3], [-1, -2, -3], [1, 0, -1], [0, 1, 0], [0, 0, 0]] {
                    let origin = origin.map(integer);
                    let direction = direction.map(integer);
                    for (owner, group) in &owners {
                        let actual = b.with_retained(faces.len(), |b| {
                            index.ray_intersections(
                                &origin,
                                &direction,
                                &faces,
                                *owner,
                                &group.bounds,
                                b,
                            )
                        })?;
                        let expected = faces
                            .iter()
                            .enumerate()
                            .filter_map(|(i, f)| {
                                (f.id().component == *owner
                                    && independent_closed_ray_box(f, &origin, &direction))
                                .then_some(i)
                            })
                            .collect::<Vec<_>>();
                        assert_eq!(actual, expected);
                        let mut flat = Budget::new(crate::Limits::default())?;
                        let old = ray_parity(
                            faces.iter().filter(|f| f.id().component == *owner),
                            &origin,
                            &direction,
                            &mut flat,
                        )?;
                        let new = ray_parity(
                            actual.into_iter().map(|i| &faces[i]),
                            &origin,
                            &direction,
                            b,
                        )?;
                        assert_eq!(new, old);
                    }
                }
            }
            Ok(())
        })?;
        assert_eq!(b.retained_memberships(), 0);
        let mut short = Budget::new(crate::Limits {
            storage_memberships: members
                + faces.len()
                + FacetBoundsIndex::stack_members(faces.len())
                - 1,
            ..crate::Limits::default()
        })?;
        let result = short.with_retained(members, |b| {
            let index = FacetBoundsIndex::build(&faces, b)?;
            let owners = component_bounds(&faces, &index.facets, b)?;
            let group = owners.get(&0).ok_or_else(|| {
                b.fail(FailureKind::InvalidWitness, "missing interval oracle owner")
            })?;
            b.with_retained(faces.len(), |b| {
                index.ray_intersections(
                    &[0, 0, 0].map(integer),
                    &[1, 2, 3].map(integer),
                    &faces,
                    0,
                    &group.bounds,
                    b,
                )
            })
        });
        assert_eq!(result.err().map(|e| e.kind), Some(FailureKind::Storage));
        assert_eq!(short.retained_memberships(), 0);
        assert!(short.peak_admitted_memberships() <= short.limits().storage_memberships);
        Ok(())
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod identity_tests;

#[cfg(test)]
#[path = "owner_range_tests.rs"]
mod owner_range_tests;

#[cfg(test)]
#[path = "proved_visit_tests.rs"]
mod proved_visit_tests;

#[path = "spatial_index.rs"]
mod spatial_index;

#[path = "strict_interior.rs"]
mod strict_interior;

#[cfg(test)]
#[path = "source_work_tests.rs"]
mod source_work_tests;

#[path = "convex_sections.rs"]
mod convex_sections;

#[cfg(test)]
#[path = "convex_section_tests.rs"]
mod convex_section_tests;

#[path = "cell_index.rs"]
mod cell_index;

#[cfg(test)]
#[path = "cell_index_tests.rs"]
mod cell_index_tests;
