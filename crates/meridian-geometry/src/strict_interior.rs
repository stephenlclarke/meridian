//! Strict whole-facet occlusion from audited original convex source boundaries.
use super::{Bounds, ComponentBounds, FacetBoundsIndex, log_cost};
use crate::{Budget, Facet, Failure, Point3, Rational};
use num_traits::{Signed, Zero};
use std::collections::{BTreeMap, BTreeSet};

type Group<'a> = (&'a Bounds, Vec<&'a Facet>);

struct Owner<'a> {
    id: u32,
    bounds: &'a Bounds,
    planes: Vec<&'a Facet>,
}
pub(super) struct InteriorCertificates<'a> {
    owners: Vec<Owner<'a>>,
    members: usize,
}
impl<'a> InteriorCertificates<'a> {
    // Caller has completed both original closed-component and sheet audits.
    pub(super) fn build(
        originals: &'a [Facet],
        owners: &'a BTreeMap<u32, ComponentBounds>,
        b: &mut Budget,
    ) -> Result<Option<Self>, Failure> {
        let entry = b.retained_memberships();
        let result = b.with_retained(16, |b| Self::build_admitted(originals, owners, b));
        if result.is_err() {
            b.release(b.retained_memberships() - entry);
        }
        result
    }
    fn build_admitted(
        originals: &'a [Facet],
        owners: &'a BTreeMap<u32, ComponentBounds>,
        b: &mut Budget,
    ) -> Result<Option<Self>, Failure> {
        let mut groups: BTreeMap<u32, Group<'a>> = BTreeMap::new();
        for (id, owner) in owners {
            b.step(1)?;
            // Use the existing large-owner structural boundary. Small owners
            // retain their original classification without certificate overhead.
            if owner.facets > 4 * FacetBoundsIndex::LEAF_SIZE {
                b.retain(6)?;
                log_cost(groups.len(), b)?;
                groups.insert(*id, (&owner.bounds, Vec::new()));
            }
        }
        if groups.is_empty() {
            return Ok(None);
        }
        for face in originals {
            b.step(1)?;
            log_cost(groups.len(), b)?;
            if let Some((_, faces)) = groups.get_mut(&face.id().component) {
                b.retain(1)?;
                faces.push(face);
            }
        }
        b.retain(16)?;
        let mut result = Self {
            owners: Vec::new(),
            members: 16,
        };
        for (id, (bounds, faces)) in groups {
            b.step(1)?;
            let group_members = 6 + faces.len();
            let planes = Self::certify(&faces, b)?;
            drop(faces);
            b.release(group_members);
            if let Some(planes) = planes {
                // certify admits the returned vector and owner record before
                // transferring refs out of its temporary equation map.
                result.members += 16 + planes.len();
                result.owners.push(Owner { id, bounds, planes });
            }
        }
        if result.owners.is_empty() {
            drop(result);
            b.release(16);
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }
    fn certify(faces: &[&'a Facet], b: &mut Budget) -> Result<Option<Vec<&'a Facet>>, Failure> {
        b.with_retained(32, |b| {
            let mut vertices: BTreeSet<&'a Point3> = BTreeSet::new();
            let mut planes: BTreeMap<(&'a Point3, &'a Rational), &'a Facet> = BTreeMap::new();
            for face in faces {
                let face = *face;
                b.at(face.id());
                for vertex in face.vertices() {
                    b.retain(2)?;
                    log_cost(vertices.len(), b)?;
                    if !vertices.insert(&vertex.point) {
                        b.release(2);
                    }
                }
                b.retain(4)?;
                log_cost(planes.len(), b)?;
                if planes
                    .insert((face.normal(), face.offset()), face)
                    .is_some()
                {
                    b.release(4);
                }
            }
            let vertex_members = 2 * vertices.len();
            let plane_members = 4 * planes.len();
            let mut convex = true;
            'planes: for (ordinal, plane) in planes.values().enumerate() {
                b.at(plane.id());
                for (vertex, point) in vertices.iter().enumerate() {
                    b.state([
                        faces.len(),
                        vertices.len(),
                        planes.len(),
                        ordinal,
                        vertex,
                        0,
                        0,
                        0,
                    ]);
                    b.step(1)?;
                    if plane.distance(point).is_positive() {
                        convex = false;
                        break 'planes;
                    }
                }
            }
            drop(vertices);
            b.release(vertex_members);
            let result = if convex {
                b.retain(16 + planes.len())?;
                b.step(planes.len() as u64)?;
                Some(planes.into_values().collect())
            } else {
                drop(planes);
                None
            };
            // The equation map has actually dropped after reference transfer.
            b.release(plane_members);
            Ok(result)
        })
    }
    // Query the exact coefficient pair, not merely a matching owner ID.
    // Original support references remain sorted by their equation-map keys.
    pub(super) fn source_outward_empty(
        &self,
        face: &Facet,
        b: &mut Budget,
    ) -> Result<bool, Failure> {
        b.with_retained(32, |b| {
            let (mut low, mut high) = (0, self.owners.len());
            while low < high {
                b.step(1)?;
                let middle = low + (high - low) / 2;
                let owner = &self.owners[middle];
                match owner.id.cmp(&face.id().component) {
                    std::cmp::Ordering::Less => low = middle + 1,
                    std::cmp::Ordering::Greater => high = middle,
                    std::cmp::Ordering::Equal => {
                        let (mut first, mut end) = (0, owner.planes.len());
                        while first < end {
                            b.step(1)?;
                            let at = first + (end - first) / 2;
                            let source = owner.planes[at];
                            match (source.normal(), source.offset())
                                .cmp(&(face.normal(), face.offset()))
                            {
                                std::cmp::Ordering::Less => first = at + 1,
                                std::cmp::Ordering::Greater => end = at,
                                std::cmp::Ordering::Equal => return Ok(true),
                            }
                        }
                        return Ok(false);
                    }
                }
            }
            Ok(false)
        })
    }
    pub(super) fn memberships(&self) -> usize {
        self.members
    }
    pub(super) fn strictly_covers(
        &self,
        face: &Facet,
        bounds: &Bounds,
        b: &mut Budget,
    ) -> Result<bool, Failure> {
        b.with_retained(32, |b| self.strictly_covers_admitted(face, bounds, b))
    }
    fn strictly_covers_admitted(
        &self,
        face: &Facet,
        bounds: &Bounds,
        b: &mut Budget,
    ) -> Result<bool, Failure> {
        for owner in &self.owners {
            b.step(1)?;
            if owner.id == face.id().component {
                continue;
            }
            // Necessary strict box containment; equality cannot be interior.
            b.step(1)?;
            if !(0..3).all(|k| {
                owner.bounds.lower[k] < bounds.lower[k] && bounds.upper[k] < owner.bounds.upper[k]
            }) {
                continue;
            }
            let mut inside = true;
            'planes: for (ordinal, plane) in owner.planes.iter().enumerate() {
                for (vertex, point) in face.vertices().iter().enumerate() {
                    b.state([
                        self.owners.len(),
                        owner.id as usize,
                        owner.planes.len(),
                        ordinal,
                        vertex,
                        0,
                        0,
                        0,
                    ]);
                    b.step(1)?;
                    let distance = plane.distance(&point.point);
                    if distance.is_zero() || distance.is_positive() {
                        inside = false;
                        break 'planes;
                    }
                }
            }
            if inside {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
#[path = "strict_interior_tests.rs"]
mod tests;
