use crate::exact::{add, area, cross, dot, orient, point, project, scale, sub};
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
fn audit_closed(faces: &[Facet], per_component: bool, b: &mut Budget) -> Result<(), Failure> {
    b.guard(96 * faces.len() + 64)?;
    let mut edges: BTreeMap<(u32, [Point3; 2]), Vec<bool>> = BTreeMap::new();
    let mut stars: BTreeMap<(u32, Point3), Vec<[Point3; 2]>> = BTreeMap::new();
    for face in faces {
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
fn occupied(
    p: &Point3,
    normal: &Point3,
    originals: &[Facet],
    owners: &BTreeSet<u32>,
    b: &mut Budget,
) -> Result<bool, Failure> {
    for owner in owners {
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
            let mut parity = false;
            let mut ambiguous = false;
            for face in originals.iter().filter(|v| v.id().component == *owner) {
                b.step(1)?;
                let denominator = dot(face.normal(), &direction);
                let numerator = face.offset() - dot(face.normal(), p);
                if denominator.is_zero() {
                    if numerator.is_zero() && face.contains(p, b)?.0 {
                        ambiguous = true;
                        break;
                    }
                    continue;
                }
                let t = numerator / denominator;
                if t <= integer(0) {
                    continue;
                }
                let hit = add(p, &scale(&direction, &t));
                point(&hit, b)?;
                let (inside, edge) = face.contains(&hit, b)?;
                if !inside {
                    continue;
                }
                if edge {
                    ambiguous = true;
                    break;
                }
                parity = !parity;
            }
            if ambiguous {
                continue;
            }
            if parity {
                return Ok(true);
            }
            break;
        }
    }
    Ok(false)
}
fn visible(
    face: &Facet,
    sample: &Point2,
    originals: &[Facet],
    owners: &BTreeSet<u32>,
    b: &mut Budget,
) -> Result<bool, Failure> {
    let p = face.lift(sample, b)?;
    if occupied(&p, face.normal(), originals, owners, b)? {
        return Ok(false);
    }
    let mut winner = face.id();
    for other in originals {
        b.step(1)?;
        if cross(face.normal(), other.normal())
            .iter()
            .any(|v| !v.is_zero())
            || !other.distance(&p).is_zero()
            || dot(face.normal(), other.normal()) <= integer(0)
        {
            continue;
        }
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
        let a = face.lift(&cell[i], b)?;
        let c = face.lift(&cell[(i + 1) % cell.len()], b)?;
        let mut cuts = BTreeSet::new();
        for p in all_points {
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
fn facet_stars(triangles: &[[Point2; 3]], b: &mut Budget) -> Result<Vec<ContactStar>, Failure> {
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
        connected_link(&sectors, !path, b)?;
        result.push(ContactStar {
            point,
            sectors,
            boundary_path: path,
        });
    }
    Ok(result)
}

/// Direct original-component adapter. Inputs are transformed original triangles,
/// including all disconnected sheets of each component. Every output triangle
/// retains one original facet and its complete affine field.
pub fn reconstruct(originals: &[Facet], b: &mut Budget) -> Result<Hull, Failure> {
    if originals.is_empty() {
        return Err(b.fail(FailureKind::InvalidInput, "missing original components"));
    }
    b.guard(originals.len())?;
    let input_members =
        40 * originals.len() + 64 + originals.iter().map(|f| f.style().len()).sum::<usize>();
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
    audit_closed(originals, true, b)?;
    audit_original_sheets(originals, b)?;
    let mut regions = Vec::new();
    let mut all_points = BTreeSet::new();
    let mut held = 0usize;
    for face in originals {
        b.at(face.id());
        let mut domain = face.domain().to_vec();
        if area(&domain, b)?.is_negative() {
            domain.reverse();
        }
        let mut barriers = Vec::new();
        for other in originals {
            b.step(1)?;
            if face.id() == other.id() {
                continue;
            }
            let (segments, _) = face.section(other, b)?;
            b.guard(8 * (barriers.len() + segments.len() + 1))?;
            barriers.extend(
                segments
                    .into_iter()
                    .map(|[a, c]| [project(&a, face.axis()), project(&c, face.axis())]),
            );
        }
        let arrangement = Arrangement::build(&domain, &barriers, b, |sample, budget| {
            visible(face, sample, originals, &owners, budget)
        })?;
        let members = arrangement.cells.iter().map(Vec::len).sum::<usize>();
        let reserve = 16 * (members + arrangement.protected.len() + arrangement.physical.len() + 1);
        b.retain(reserve)?;
        held += reserve;
        for p in arrangement
            .cells
            .iter()
            .flatten()
            .chain(arrangement.protected.iter())
        {
            let p = face.lift(p, b)?;
            b.guard(8 * (all_points.len() + 1))?;
            log_cost(all_points.len(), b)?;
            all_points.insert(p);
        }
        regions.push((face, arrangement));
    }
    let point_members = 8 * all_points.len();
    b.retain(point_members)?;
    let mut triangles = Vec::new();
    let mut witnesses = Vec::new();
    for (source, arrangement) in &regions {
        b.at(source.id());
        let mut planar = Vec::new();
        let mut expected = integer(0);
        for cell in &arrangement.cells {
            expected += area(cell, b)?;
            let noded = node_cell(source, cell, &all_points, b)?;
            let new = triangulate(noded, b)?;
            b.guard(32 * (planar.len() + new.len() + 1))?;
            planar.extend(new);
        }
        let stars = facet_stars(&planar, b)?;
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
        let witness_members = incoming_witness_memberships(
            &arrangement.cells,
            arrangement.physical.len(),
            arrangement.protected.len(),
            planar.len(),
            &stars,
            b,
        )?;
        // Output facets and the complete final witness own separate records.
        // Arrangement copies stay in `held` until the complete hull audit ends.
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
            classified_cells: arrangement.cells.clone(),
            physical_segments: arrangement.physical.iter().cloned().collect(),
            protected_points: arrangement.protected.iter().cloned().collect(),
            output_triangles: planar,
            contact_stars: stars,
            twice_area: expected,
        };
        // The final witness is already retained above. Public standalone verify
        // reserves independently owned input; use the admitted path here.
        witness.verify_admitted(source, b)?;
        witnesses.push(witness);
    }
    if triangles.is_empty() {
        return Err(b.fail(FailureKind::InvalidWitness, "lost entire original exterior"));
    }
    audit_closed(&triangles, false, b)?;
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
        if facet_stars(&self.output_triangles, b)? != self.contact_stars {
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
