//! One stable exact cell grouping; original bounds remain the query authority.
use super::{Bounds, BoundsNode, FacetBoundsIndex};
use crate::{Budget, Facet, Failure, FailureKind, integer};
use num_traits::{ToPrimitive, Zero};

const MIN_FACETS: usize = 32 * FacetBoundsIndex::LEAF_SIZE;
const CELLS: usize = 512;

impl FacetBoundsIndex {
    pub(super) fn cell_memberships(count: usize) -> usize {
        Self::memberships(count) + if count >= MIN_FACETS { count + 16 } else { 0 }
    }
    pub(super) fn build_cells(originals: &[Facet], b: &mut Budget) -> Result<Self, Failure> {
        if originals.len() < MIN_FACETS {
            return Self::build(originals, b);
        }
        let mut index = Self {
            facets: originals
                .iter()
                .map(|face| Bounds::for_facet(face, b))
                .collect::<Result<_, _>>()?,
            nodes: Vec::with_capacity(Self::node_limit(originals.len())),
            root: 0,
            order: None,
        };
        // Returned order plus marker are covered by cell_memberships. All
        // counting keys, cursors and rational temporary values are scoped here.
        index.order = Some(b.with_retained(originals.len() + CELLS + 48, |b| index.cell_order(b))?);
        index.root = b.with_retained(Self::stack_members(originals.len()), |b| {
            index.cell_range(originals, 0, originals.len(), 0, b)
        })?;
        Ok(index)
    }
    fn cell_order(&self, b: &mut Budget) -> Result<Vec<usize>, Failure> {
        let count = self.facets.len();
        b.step(6)?;
        let mut world = self.facets[0].clone();
        for bounds in &self.facets[1..] {
            b.step(6)?;
            world.include(bounds);
        }
        b.step(3)?;
        let widths: [_; 3] = std::array::from_fn(|axis| &world.upper[axis] - &world.lower[axis]);
        b.step(CELLS as u64)?;
        let mut bins = [0usize; CELLS];
        let mut keys = Vec::with_capacity(count);
        for (ordinal, bounds) in self.facets.iter().enumerate() {
            b.state([count, ordinal, keys.len(), CELLS, 0, 0, 0, 0]);
            b.step(1)?;
            let mut key = 0;
            for (axis, width) in widths.iter().enumerate() {
                // Exact coordinate quantization affects grouping only. Every
                // resulting node gets original bounds, never cell boundaries.
                b.step(1)?;
                let digit = if width.is_zero() {
                    0
                } else {
                    let relative =
                        &bounds.lower[axis] + &bounds.upper[axis] - integer(2) * &world.lower[axis];
                    let scaled = relative * integer(4) / width;
                    b.step(1)?;
                    scaled
                        .to_integer()
                        .to_usize()
                        .ok_or_else(|| {
                            b.fail(
                                FailureKind::InvalidWitness,
                                "invalid spatial cell coordinate",
                            )
                        })?
                        .min(7)
                };
                for bit in 0..3 {
                    b.step(1)?;
                    key |= ((digit >> bit) & 1) << (3 * bit + axis);
                }
            }
            b.step(2)?;
            keys.push(key);
            bins[key] += 1;
        }
        let mut offset = 0;
        for bin in &mut bins {
            b.step(1)?;
            let size = *bin;
            *bin = offset;
            offset += size;
        }
        b.step((count + 1) as u64)?;
        let mut order = vec![0; count + 1];
        for (ordinal, key) in keys.into_iter().enumerate() {
            b.state([count, ordinal, key, bins[key], CELLS, 0, 0, 0]);
            b.step(2)?;
            order[bins[key]] = ordinal;
            bins[key] += 1;
        }
        // The marker is outside every node range. It distinguishes this exact
        // permutation from the frozen recursively sorted test-only prototype.
        b.step(1)?;
        order[count] = count;
        Ok(order)
    }
    pub(super) fn cell_source_order<T: Copy>(
        &self,
        mut records: Vec<T>,
        fields: usize,
        key: impl Fn(&T) -> usize,
        b: &mut Budget,
    ) -> Result<Vec<T>, Failure> {
        if records.len() < 2 {
            if let Some(record) = records.first() {
                b.step(1)?;
                if key(record) >= self.facets.len() {
                    return Err(b.fail(
                        FailureKind::InvalidWitness,
                        "cell candidate ordinal outside source",
                    ));
                }
            }
            return Ok(records);
        }
        let words = self.facets.len().div_ceil(64);
        b.with_retained(2 * words + 2 * fields + 16, |b| {
            b.step((2 * words) as u64)?;
            let mut bits = vec![0u64; words];
            let mut prefix = vec![0usize; words];
            for record in &records {
                let ordinal = key(record);
                b.state([
                    self.facets.len(),
                    records.len(),
                    words,
                    ordinal,
                    fields,
                    0,
                    0,
                    0,
                ]);
                b.step(3)?;
                if ordinal >= self.facets.len() {
                    return Err(b.fail(
                        FailureKind::InvalidWitness,
                        "cell candidate ordinal outside source",
                    ));
                }
                let mask = 1u64 << (ordinal % 64);
                if bits[ordinal / 64] & mask != 0 {
                    return Err(b.fail(
                        FailureKind::InvalidWitness,
                        "duplicate cell candidate ordinal",
                    ));
                }
                bits[ordinal / 64] |= mask;
            }
            let mut rank = 0;
            for (word, value) in bits.iter().enumerate() {
                b.step(1)?;
                prefix[word] = rank;
                rank += value.count_ones() as usize;
            }
            for position in 0..records.len() {
                loop {
                    let ordinal = key(&records[position]);
                    b.state([
                        self.facets.len(),
                        records.len(),
                        position,
                        ordinal,
                        fields,
                        words,
                        0,
                        0,
                    ]);
                    b.step(1)?;
                    let below = (1u64 << (ordinal % 64)) - 1;
                    let target =
                        prefix[ordinal / 64] + (bits[ordinal / 64] & below).count_ones() as usize;
                    if target == position {
                        break;
                    }
                    b.step(2)?;
                    records.swap(position, target);
                }
            }
            Ok(records)
        })
    }
    fn cell_range(
        &mut self,
        originals: &[Facet],
        start: usize,
        end: usize,
        depth: usize,
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
            let middle = start + (end - start) / 2;
            let left = self.cell_range(originals, start, middle, depth + 1, b)?;
            let right = self.cell_range(originals, middle, end, depth + 1, b)?;
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
}
