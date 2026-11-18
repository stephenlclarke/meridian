//! Exact half-plane geometry for one original facet and certified convex owners.
use super::strict_interior::InteriorCertificates;
use super::{Bounds, log_cost};
use crate::{Budget, Facet, Failure, Point2, Rational};
use num_traits::{Signed, Zero};
use std::collections::BTreeMap;

struct Equation {
    normal: Point2,
    constant: Rational,
}
struct Row {
    equations: Vec<Equation>,
    outside: bool,
}
pub(super) struct Rows {
    owners: BTreeMap<u32, Row>,
    members: usize,
}
impl Rows {
    pub(super) fn build(
        face: &Facet,
        finite: &Bounds,
        certificates: Option<&InteriorCertificates<'_>>,
        b: &mut Budget,
    ) -> Result<Self, Failure> {
        let entry = b.retained_memberships();
        let result = b.with_retained(32, |b| Self::build_admitted(face, finite, certificates, b));
        if result.is_err() {
            b.release(b.retained_memberships() - entry);
        }
        result
    }
    fn build_admitted(
        face: &Facet,
        finite: &Bounds,
        certificates: Option<&InteriorCertificates<'_>>,
        b: &mut Budget,
    ) -> Result<Self, Failure> {
        b.retain(16)?;
        let mut result = Self {
            owners: BTreeMap::new(),
            members: 16,
        };
        let Some(certificates) = certificates else {
            return Ok(result);
        };
        // The reviewed exact own-support proof also avoids constructing a
        // plane row whose outward section is already known to be empty.
        let empty_own = certificates.source_outward_empty(face, b)?;
        // The dominant-axis choice performs two magnitude comparisons.
        b.step(2)?;
        let axis = face.axis();
        let axes: [usize; 2] = match axis {
            0 => [1, 2],
            1 => [0, 2],
            _ => [0, 1],
        };
        let source = &face.normal()[axis];
        for (owner, bounds, planes) in certificates.section_owners() {
            b.state([
                result.members,
                result.owners.len(),
                planes.len(),
                0,
                axis,
                0,
                0,
                0,
            ]);
            b.step(1)?;
            if !finite.meets(bounds, b)? {
                continue;
            }
            log_cost(result.owners.len(), b)?;
            b.retain(16)?;
            result.members += 16;
            let mut row = Row {
                equations: Vec::new(),
                outside: false,
            };
            if empty_own && owner == face.id().component {
                row.outside = true;
                result.owners.insert(owner, row);
                continue;
            }
            for (ordinal, plane) in planes.iter().enumerate() {
                b.state([
                    result.members,
                    result.owners.len(),
                    planes.len(),
                    ordinal,
                    axis,
                    row.equations.len(),
                    32,
                    0,
                ]);
                // One exact plane projection and one parallel-plane predicate.
                b.step(2)?;
                let mut normal = axes.map(|k| {
                    &plane.normal()[k] * source - &plane.normal()[axis] * &face.normal()[k]
                });
                let mut constant = &plane.normal()[axis] * face.offset() - plane.offset() * source;
                if source.is_negative() {
                    normal = normal.map(|n| -n);
                    constant = -constant;
                }
                if normal.iter().all(Zero::is_zero) {
                    b.step(1)?;
                    // A coincident parallel support is satisfied strictly on
                    // the outward side only when its normal opposes the source.
                    if constant.is_positive()
                        || constant.is_zero()
                            && plane.normal()[axis].is_positive() == source.is_positive()
                    {
                        row.outside = true;
                        break;
                    }
                    continue;
                }
                b.step(1)?;
                b.retain(8)?;
                result.members += 8;
                row.equations.push(Equation { normal, constant });
            }
            if row.outside {
                let transient = 8 * row.equations.len();
                drop(std::mem::take(&mut row.equations));
                b.release(transient);
                result.members -= transient;
            }
            result.owners.insert(owner, row);
        }
        Ok(result)
    }
    pub(super) fn contains(&self, owner: u32, b: &mut Budget) -> Result<bool, Failure> {
        log_cost(self.owners.len(), b)?;
        Ok(self.owners.contains_key(&owner))
    }
    pub(super) fn memberships(&self) -> usize {
        self.members
    }
    // Every value is computed for this sample. Only exact source-plane geometry
    // is retained; no occupancy, intersection or ray answer is cached.
    pub(super) fn occupied(
        &self,
        sample: &Point2,
        b: &mut Budget,
    ) -> Result<Option<bool>, Failure> {
        b.with_retained(32, |b| {
            for (owner, row) in self.owners.values().enumerate() {
                b.state([
                    self.members,
                    self.owners.len(),
                    row.equations.len(),
                    owner,
                    0,
                    32,
                    0,
                    0,
                ]);
                b.step(1)?;
                if row.outside {
                    continue;
                }
                let mut boundary = false;
                let mut outside = false;
                for (ordinal, equation) in row.equations.iter().enumerate() {
                    b.state([
                        self.members,
                        self.owners.len(),
                        row.equations.len(),
                        owner,
                        ordinal,
                        32,
                        0,
                        0,
                    ]);
                    b.step(1)?;
                    let distance = &equation.normal[0] * &sample[0]
                        + &equation.normal[1] * &sample[1]
                        + &equation.constant;
                    if distance.is_positive() {
                        outside = true;
                        break;
                    }
                    boundary |= distance.is_zero();
                }
                if !outside {
                    if boundary {
                        return Ok(None);
                    }
                    return Ok(Some(true));
                }
            }
            Ok(Some(false))
        })
    }
}
