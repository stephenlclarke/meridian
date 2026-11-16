use crate::exact::{add, axis, checked, cross, dot, orient, point, project, scale, sub};
use crate::{Budget, Failure, FailureKind, Point2, Point3, Rational, integer};
use num_traits::{Signed, Zero};
use std::sync::Arc;

/// IDs are assigned before applying component transforms and survive every split.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceId {
    pub component: u32,
    pub facet: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vertex {
    pub point: Point3,
    pub uv: Point2,
}

#[derive(Clone, Debug)]
enum Style {
    Owned(String),
    Shared(Arc<str>),
}
impl Style {
    fn as_str(&self) -> &str {
        match self {
            Self::Owned(s) => s,
            Self::Shared(s) => s,
        }
    }
}
impl PartialEq for Style {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
impl Eq for Style {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facet {
    id: SourceId,
    vertices: [Vertex; 3],
    style: Style,
    normal: Point3,
    offset: Rational,
}

impl Facet {
    pub fn new(
        id: SourceId,
        vertices: [Vertex; 3],
        style: String,
        budget: &mut Budget,
    ) -> Result<Self, Failure> {
        Self::with_style(id, vertices, Style::Owned(style), budget)
    }
    pub(crate) fn with_shared_style(
        id: SourceId,
        vertices: [Vertex; 3],
        style: Arc<str>,
        budget: &mut Budget,
    ) -> Result<Self, Failure> {
        Self::with_style(id, vertices, Style::Shared(style), budget)
    }
    fn with_style(
        id: SourceId,
        vertices: [Vertex; 3],
        style: Style,
        budget: &mut Budget,
    ) -> Result<Self, Failure> {
        budget.at(id);
        budget.guard(32 + style.as_str().len())?;
        for vertex in &vertices {
            point(&vertex.point, budget)?;
            point(&vertex.uv, budget)?;
        }
        budget.step(1)?;
        let normal = cross(
            &sub(&vertices[1].point, &vertices[0].point),
            &sub(&vertices[2].point, &vertices[0].point),
        );
        if normal.iter().all(Zero::is_zero) {
            return Err(budget.fail(FailureKind::Degenerate, "zero-area original triangle"));
        }
        let offset = dot(&normal, &vertices[0].point);
        Ok(Self {
            id,
            vertices,
            style,
            normal,
            offset,
        })
    }
    pub fn id(&self) -> SourceId {
        self.id
    }
    pub fn vertices(&self) -> &[Vertex; 3] {
        &self.vertices
    }
    pub fn style(&self) -> &str {
        self.style.as_str()
    }
    /// Includes original geometry, identity maps and distinct owned style data.
    /// Equal text in separate allocations is deliberately counted separately.
    pub(crate) fn input_memberships(faces: &[Self], b: &mut Budget) -> Result<usize, Failure> {
        b.guard(faces.len())?;
        let mut members = 40 * faces.len() + 64;
        let mut styles: Vec<&Arc<str>> = Vec::new();
        b.guard(members)?;
        for face in faces {
            b.at(face.id());
            b.step(1)?;
            let added = match &face.style {
                Style::Owned(style) => style.len(),
                Style::Shared(style) => {
                    let mut shared = false;
                    for prior in &styles {
                        b.step(1)?;
                        if Arc::ptr_eq(prior, style) {
                            shared = true;
                            break;
                        }
                    }
                    if shared {
                        0
                    } else {
                        // Two reference-count slots and the immutable text.
                        b.guard(members + style.len() + 2 + styles.len() + 1)?;
                        styles.push(style);
                        style.len() + 2
                    }
                }
            };
            members = members
                .checked_add(added)
                .ok_or_else(|| b.fail(FailureKind::Storage, "original style storage overflow"))?;
            // The temporary allocation index releases before identity maps.
            b.guard(members + styles.len())?;
        }
        Ok(members)
    }
    pub(crate) fn normal(&self) -> &Point3 {
        &self.normal
    }
    pub(crate) fn offset(&self) -> &Rational {
        &self.offset
    }
    pub(crate) fn axis(&self) -> usize {
        axis(&self.normal)
    }
    pub(crate) fn domain(&self) -> [Point2; 3] {
        self.vertices
            .each_ref()
            .map(|v| project(&v.point, self.axis()))
    }
    pub(crate) fn distance(&self, p: &Point3) -> Rational {
        dot(&self.normal, p) - &self.offset
    }
    pub fn uv_at(&self, p: &Point3, b: &mut Budget) -> Result<Point2, Failure> {
        point(p, b)?;
        b.step(1)?;
        if !self.distance(p).is_zero() {
            return Err(b.fail(FailureKind::InvalidWitness, "point left source plane"));
        }
        let [a, c, d] = self.domain();
        let q = project(p, self.axis());
        let total = orient(&a, &c, &d, b)?;
        let first = orient(&q, &c, &d, b)? / &total;
        let second = orient(&a, &q, &d, b)? / &total;
        let third = integer(1) - &first - &second;
        if [&first, &second, &third].iter().any(|v| v.is_negative()) {
            return Err(b.fail(FailureKind::InvalidWitness, "point left original triangle"));
        }
        self.affine_uv(p, b)
    }
    pub(crate) fn affine_uv(&self, p: &Point3, b: &mut Budget) -> Result<Point2, Failure> {
        let [a, c, d] = self.domain();
        let q = project(p, self.axis());
        let total = orient(&a, &c, &d, b)?;
        let first = orient(&q, &c, &d, b)? / &total;
        let second = orient(&a, &q, &d, b)? / &total;
        let third = integer(1) - &first - &second;
        let uv = std::array::from_fn(|k| {
            &self.vertices[0].uv[k] * &first
                + &self.vertices[1].uv[k] * &second
                + &self.vertices[2].uv[k] * &third
        });
        point(&uv, b)?;
        Ok(uv)
    }
    pub(crate) fn lift(&self, p: &Point2, b: &mut Budget) -> Result<Point3, Failure> {
        let axis = self.axis();
        let mut result = std::array::from_fn(|_| integer(0));
        let mut next = 0;
        for (i, v) in result.iter_mut().enumerate() {
            if i != axis {
                *v = p[next].clone();
                next += 1;
            }
        }
        result[axis] = (&self.offset - dot(&self.normal, &result)) / &self.normal[axis];
        point(&result, b)?;
        Ok(result)
    }
    pub(crate) fn contains(&self, p: &Point3, b: &mut Budget) -> Result<(bool, bool), Failure> {
        let mut edge = false;
        for i in 0..3 {
            b.step(1)?;
            let sign = dot(
                &cross(
                    &sub(&self.vertices[(i + 1) % 3].point, &self.vertices[i].point),
                    &sub(p, &self.vertices[i].point),
                ),
                &self.normal,
            );
            if sign.is_negative() {
                return Ok((false, false));
            }
            edge |= sign.is_zero();
        }
        Ok((true, edge))
    }
    pub(crate) fn section(
        &self,
        other: &Self,
        b: &mut Budget,
    ) -> Result<(Vec<[Point3; 2]>, bool), Failure> {
        b.guard(48)?;
        let mut distances = Vec::with_capacity(3);
        for v in &other.vertices {
            b.step(1)?;
            distances.push(self.distance(&v.point));
        }
        if distances.iter().all(Zero::is_zero) {
            return Ok((
                (0..3)
                    .map(|i| {
                        [
                            other.vertices[i].point.clone(),
                            other.vertices[(i + 1) % 3].point.clone(),
                        ]
                    })
                    .collect(),
                true,
            ));
        }
        let mut hits = std::collections::BTreeSet::new();
        for i in 0..3 {
            b.step(1)?;
            let j = (i + 1) % 3;
            if distances[i].is_zero() {
                hits.insert(other.vertices[i].point.clone());
            }
            if (&distances[i] * &distances[j]).is_negative() {
                let t = &distances[i] / (&distances[i] - &distances[j]);
                let p = add(
                    &other.vertices[i].point,
                    &scale(&sub(&other.vertices[j].point, &other.vertices[i].point), &t),
                );
                point(&p, b)?;
                hits.insert(p);
            }
        }
        b.step(hits.len() as u64)?;
        match (hits.first(), hits.last()) {
            (Some(a), Some(c)) => Ok((vec![[a.clone(), c.clone()]], false)),
            _ => Ok((Vec::new(), false)),
        }
    }
}

/// Exact source transform. Identity and provenance remain attached to each facet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffineTransform {
    pub linear: [[Rational; 3]; 3],
    pub translation: Point3,
}
impl AffineTransform {
    pub fn apply(&self, face: &Facet, b: &mut Budget) -> Result<Facet, Failure> {
        for row in &self.linear {
            point(row, b)?;
        }
        point(&self.translation, b)?;
        let determinant = dot(&self.linear[0], &cross(&self.linear[1], &self.linear[2]));
        checked(&determinant, b)?;
        // Mirroring reverses orientation, and must reverse the point/UV pairs together.
        if determinant.is_zero() {
            return Err(b.fail(FailureKind::Degenerate, "singular source transform"));
        }
        let mut vertices = face.vertices.clone();
        for vertex in &mut vertices {
            b.step(3)?;
            vertex.point =
                std::array::from_fn(|i| dot(&self.linear[i], &vertex.point) + &self.translation[i]);
        }
        if determinant.is_negative() {
            vertices.swap(1, 2);
        }
        Facet::with_style(face.id, vertices, face.style.clone(), b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_sections_match_independent_signed_contact_enumeration() -> Result<(), Failure> {
        let mut b = Budget::new(crate::Limits::default())?;
        let vertex = |x: i64, y: i64, z: i64| Vertex {
            point: [integer(x), integer(y), integer(z)],
            uv: [integer(0), integer(0)],
        };
        let plane = Facet::new(
            SourceId {
                component: 0,
                facet: 0,
            },
            [vertex(0, 0, 0), vertex(2, 0, 0), vertex(0, 2, 0)],
            String::new(),
            &mut b,
        )?;
        for x in [-2, 0, 3] {
            for y in [-2, 0, 3] {
                for z in [-2, 0, 3] {
                    let corners = [vertex(0, 0, x), vertex(2, 0, y), vertex(0, 2, z)];
                    let other = Facet::new(
                        SourceId {
                            component: 1,
                            facet: 0,
                        },
                        corners.clone(),
                        String::new(),
                        &mut b,
                    )?;
                    let mut expected = std::collections::BTreeSet::new();
                    for corner in &corners {
                        if corner.point[2].is_zero() {
                            expected.insert(corner.point.clone());
                        }
                    }
                    for i in 0..3 {
                        for j in i + 1..3 {
                            let p = &corners[i].point;
                            let q = &corners[j].point;
                            if (&p[2] * &q[2]).is_negative() {
                                let weight = &p[2] / (&p[2] - &q[2]);
                                expected.insert(std::array::from_fn(|k| {
                                    (integer(1) - &weight) * &p[k] + &weight * &q[k]
                                }));
                            }
                        }
                    }
                    let actual = plane.section(&other, &mut b)?;
                    if x == 0 && y == 0 && z == 0 {
                        assert!(actual.1);
                        assert_eq!(actual.0.len(), 3);
                    } else {
                        assert!(!actual.1);
                        match (expected.first(), expected.last()) {
                            (Some(first), Some(last)) => {
                                assert_eq!(actual.0, vec![[first.clone(), last.clone()]])
                            }
                            _ => assert!(actual.0.is_empty()),
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
