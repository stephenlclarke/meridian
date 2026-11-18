//! Spatial node grouping changes only necessary-condition query work.
#[cfg(test)]
use super::{Bounds, BoundsNode};
use super::{FacetBoundsIndex, log_cost};
#[cfg(test)]
use crate::Facet;
use crate::{Budget, Failure};
#[cfg(test)]
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[cfg(test)]
const SPATIAL_MIN: usize = 32 * FacetBoundsIndex::LEAF_SIZE;

impl FacetBoundsIndex {
    #[cfg(test)]
    pub(super) fn spatial_memberships(count: usize) -> usize {
        Self::memberships(count) + if count >= SPATIAL_MIN { count + 8 } else { 0 }
    }
    pub(super) fn source_ordinal(&self, position: usize) -> usize {
        self.order
            .as_ref()
            .map_or(position, |order| order[position])
    }
    #[cfg(test)]
    pub(super) fn build_spatial(originals: &[Facet], b: &mut Budget) -> Result<Self, Failure> {
        if originals.len() < SPATIAL_MIN {
            return Self::build(originals, b);
        }
        let mut index = Self {
            facets: originals
                .iter()
                .map(|face| Bounds::for_facet(face, b))
                .collect::<Result<_, _>>()?,
            nodes: Vec::with_capacity(Self::node_limit(originals.len())),
            root: 0,
            order: Some(Vec::new()),
        };
        b.step(originals.len() as u64)?;
        index.order = Some((0..originals.len()).collect());
        index.root = b.with_retained(
            originals.len() + 16 + Self::stack_members(originals.len()),
            |b| {
                b.step(originals.len() as u64)?;
                let mut scratch = vec![0; originals.len()];
                index.spatial_range(originals, 0, originals.len(), 0, &mut scratch, b)
            },
        )?;
        Ok(index)
    }
    #[cfg(test)]
    fn spatial_range(
        &mut self,
        originals: &[Facet],
        start: usize,
        end: usize,
        depth: usize,
        scratch: &mut [usize],
        b: &mut Budget,
    ) -> Result<usize, Failure> {
        b.state([
            originals.len(),
            self.nodes.len(),
            start,
            end,
            depth,
            0,
            0,
            0,
        ]);
        let (bounds, children, owner) = if end - start <= Self::LEAF_SIZE {
            let first = self.source_ordinal(start);
            b.step(6)?;
            let mut bounds = self.facets[first].clone();
            let mut owner = Some(originals[first].id().component);
            for position in start + 1..end {
                let ordinal = self.source_ordinal(position);
                b.step(7)?;
                bounds.include(&self.facets[ordinal]);
                if owner != Some(originals[ordinal].id().component) {
                    owner = None;
                }
            }
            (bounds, None, owner)
        } else {
            self.sort_range(start, end, depth % 3, scratch, b)?;
            let middle = start + (end - start) / 2;
            let left = self.spatial_range(originals, start, middle, depth + 1, scratch, b)?;
            let right = self.spatial_range(originals, middle, end, depth + 1, scratch, b)?;
            b.state([
                originals.len(),
                self.nodes.len(),
                start,
                end,
                depth,
                0,
                0,
                0,
            ]);
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
        let ordinal = self.nodes.len();
        self.nodes.push(BoundsNode {
            bounds,
            start,
            end,
            children,
            owner,
        });
        Ok(ordinal)
    }
    #[cfg(test)]
    fn sort_range(
        &mut self,
        start: usize,
        end: usize,
        axis: usize,
        scratch: &mut [usize],
        b: &mut Budget,
    ) -> Result<(), Failure> {
        let Some(order) = self.order.as_mut() else {
            return Ok(());
        };
        let count = end - start;
        let mut width = 1;
        while width < count {
            for offset in (start..end).step_by(2 * width) {
                let middle = (offset + width).min(end);
                let stop = (offset + 2 * width).min(end);
                let (mut left, mut right) = (offset, middle);
                for output in &mut scratch[offset..stop] {
                    b.state([
                        self.facets.len(),
                        self.nodes.len(),
                        start,
                        end,
                        axis,
                        width,
                        left,
                        right,
                    ]);
                    let from_left = if right == stop {
                        true
                    } else if left == middle {
                        false
                    } else {
                        b.step(1)?;
                        let comparison = self.facets[order[left]].lower[axis]
                            .cmp(&self.facets[order[right]].lower[axis]);
                        if comparison == Ordering::Equal {
                            b.step(1)?;
                            order[left] <= order[right]
                        } else {
                            comparison == Ordering::Less
                        }
                    };
                    b.step(1)?;
                    *output = if from_left {
                        let ordinal = order[left];
                        left += 1;
                        ordinal
                    } else {
                        let ordinal = order[right];
                        right += 1;
                        ordinal
                    };
                }
            }
            for position in start..end {
                b.step(1)?;
                order[position] = scratch[position];
            }
            width *= 2;
        }
        Ok(())
    }
    pub(super) fn source_order<T: Copy>(
        &self,
        records: Vec<T>,
        fields: usize,
        key: impl Fn(&T) -> usize,
        b: &mut Budget,
    ) -> Result<Vec<T>, Failure> {
        // Cell ordering appends one admitted mode marker after all N source
        // ordinals. Legacy/contiguous builders and their exact prices stay intact.
        #[cfg(test)]
        if self
            .order
            .as_ref()
            .is_some_and(|order| order.len() == self.facets.len() + 1)
        {
            return self.cell_source_order(records, fields, key, b);
        }
        if self.order.is_none() {
            return Ok(records);
        }
        let entry = b.retained_memberships();
        let count = records.len();
        b.state([count, 0, fields, 0, 0, 0, 0, 0]);
        let result = b.with_retained(16, |b| {
            let mut sorted = BTreeMap::new();
            let mut held = 0;
            for record in records {
                let ordinal = key(&record);
                b.state([count, sorted.len(), fields, ordinal, held, 0, 0, 0]);
                // Key, complete copied payload, and map membership.
                b.retain(fields + 2)?;
                held += fields + 2;
                log_cost(sorted.len(), b)?;
                sorted.insert(ordinal, record);
            }
            let mut result = Vec::new();
            for (_, record) in sorted {
                b.step(1)?;
                result.push(record);
            }
            b.release(held);
            Ok(result)
        });
        if result.is_err() {
            b.release(b.retained_memberships() - entry);
        }
        result
    }
}

#[cfg(test)]
#[path = "spatial_index_tests.rs"]
mod tests;
