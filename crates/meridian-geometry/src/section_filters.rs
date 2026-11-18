//! Strict original-support separation removes only constraints whose finite clip is empty.
use super::{Bounds, FacetBoundsIndex, OutwardPlane, log_cost};
use crate::certificate::convex_sections::Rows;
use crate::{Budget, Facet, Failure, Segment};
use std::collections::BTreeMap;

impl FacetBoundsIndex {
    pub(super) fn physical_separating(
        &self,
        finite: &Bounds,
        originals: &[Facet],
        rows: &Rows,
        b: &mut Budget,
    ) -> Result<Vec<usize>, Failure> {
        b.with_retained(Self::stack_members(self.facets.len()), |b| {
            let mut pending = vec![(self.root, false)];
            let mut result = Vec::new();
            while let Some((index, known)) = pending.pop() {
                let node = &self.nodes[index];
                b.state([
                    self.facets.len(),
                    index,
                    node.start,
                    node.end,
                    usize::from(known),
                    result.len(),
                    0,
                    0,
                ]);
                b.step(1)?;
                if !finite.meets(&node.bounds, b)? {
                    continue;
                }
                let mut known = known;
                if !known && let Some(owner) = node.owner {
                    known = true;
                    if rows.separated(owner, b)? {
                        continue;
                    }
                }
                if let Some([left, right]) = node.children {
                    pending.push((right, known));
                    pending.push((left, known));
                } else {
                    for position in node.start..node.end {
                        b.step(1)?;
                        let ordinal = self.source_ordinal(position);
                        if finite.meets(&self.facets[ordinal], b)?
                            && (known || !rows.separated(originals[ordinal].id().component, b)?)
                        {
                            result.push(ordinal);
                        }
                    }
                }
            }
            self.source_order(result, 1, |ordinal| *ordinal, b)
        })
    }
    pub(super) fn collection_separating(
        &self,
        face: &Facet,
        finite: &Bounds,
        owners: &BTreeMap<u32, Vec<Segment>>,
        originals: &[Facet],
        rows: &Rows,
        b: &mut Budget,
    ) -> Result<Vec<(usize, bool, bool)>, Failure> {
        b.with_retained(Self::stack_members(self.facets.len()) + 8, |b| {
            let plane = OutwardPlane::new(face, b)?;
            // Five fields per pending node fit the existing eight-membership
            // per-depth bound. Carry both exclusions to the descendants.
            let mut pending = vec![(self.root, true, true, false, false)];
            let mut result = Vec::new();
            while let Some((
                index,
                physical_possible,
                outward_possible,
                owner_known,
                physical_known,
            )) = pending.pop()
            {
                let node = &self.nodes[index];
                b.state([
                    self.facets.len(),
                    index,
                    node.start,
                    node.end,
                    usize::from(physical_possible),
                    usize::from(outward_possible),
                    usize::from(owner_known),
                    usize::from(physical_known),
                ]);
                b.step(1)?;
                let mut physical_possible = physical_possible && finite.meets(&node.bounds, b)?;
                let mut physical_known = physical_known;
                if physical_possible
                    && !physical_known
                    && let Some(owner) = node.owner
                {
                    physical_known = true;
                    if rows.separated(owner, b)? {
                        physical_possible = false;
                    }
                }
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
                    pending.push((
                        right,
                        physical_possible,
                        outward_possible,
                        owner_known,
                        physical_known,
                    ));
                    pending.push((
                        left,
                        physical_possible,
                        outward_possible,
                        owner_known,
                        physical_known,
                    ));
                } else {
                    for position in node.start..node.end {
                        b.step(1)?;
                        let ordinal = self.source_ordinal(position);
                        let bounds = &self.facets[ordinal];
                        let physical = physical_possible
                            && finite.meets(bounds, b)?
                            && (physical_known
                                || !rows.separated(originals[ordinal].id().component, b)?);
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
            self.source_order(result, 3, |record| record.0, b)
        })
    }
}
