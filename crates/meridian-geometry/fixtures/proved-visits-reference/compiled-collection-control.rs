impl FacetBoundsIndex {
    fn owner_inheritance_reference(
        &self,
        face: &Facet,
        finite: &Bounds,
        owners: &BTreeMap<u32, Vec<Segment>>,
        originals: &[Facet],
        b: &mut Budget,
    ) -> Result<Vec<(usize, bool, bool)>, Failure> {
        b.with_retained(Self::stack_members(self.facets.len()) + 8, |b| {
            let plane = OutwardPlane::new(face, b)?;
            // Three fields per pending node fit the existing eight-membership
            // per-depth bound. Carry both exclusions to the descendants.
            let mut pending = vec![(self.root, true, true)];
            let mut result = Vec::new();
            while let Some((index, physical_possible, outward_possible)) = pending.pop() {
                b.step(1)?;
                let node = &self.nodes[index];
                let physical_possible = physical_possible && finite.meets(&node.bounds, b)?;
                // A uniform owner is exact metadata about this source range,
                // including unordered inputs. Excluded owners cannot supply an
                // outward section; physical cuts remain independently eligible.
                let owner_allowed = if outward_possible {
                    if let Some(owner) = node.owner {
                        log_cost(owners.len(), b)?;
                        owners.contains_key(&owner)
                    } else {
                        true
                    }
                } else {
                    false
                };
                let outward_possible =
                    outward_possible && owner_allowed && plane.meets(&node.bounds, b)?;
                if !physical_possible && !outward_possible {
                    continue;
                }
                if let Some([left, right]) = node.children {
                    pending.push((right, physical_possible, outward_possible));
                    pending.push((left, physical_possible, outward_possible));
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
