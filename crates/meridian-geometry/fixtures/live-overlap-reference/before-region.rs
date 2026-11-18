use crate::{Arrangement, ArrangementWork, Budget, Cell, Failure, FailureKind, Point2, Segment};
use std::collections::BTreeSet;

// Private lossless storage between classification and globally noded publication.
// Ordinals preserve every cell corner and the source sets' lexical iteration order.
pub(crate) struct PackedRegion {
    points: Vec<Point2>,
    cells: Vec<Vec<usize>>,
    physical: Vec<[usize; 2]>,
    protected: BTreeSet<Point2>,
    work: ArrangementWork,
}
pub(crate) struct ExpandedRegion {
    pub cells: Vec<Cell>,
    pub physical: Vec<Segment>,
    pub protected: BTreeSet<Point2>,
    pub work: ArrangementWork,
}
impl PackedRegion {
    pub fn raw_memberships(cells: &[Cell], physical: usize, protected: usize) -> usize {
        16 * (cells.iter().map(Vec::len).sum::<usize>() + physical + protected + 1)
    }
    pub fn memberships(&self) -> usize {
        16 + 3 * self.points.len()
            + self.cells.len()
            + self.cells.iter().map(Vec::len).sum::<usize>()
            + 3 * self.physical.len()
            + 3 * self.protected.len()
    }
    pub fn expanded_memberships(&self) -> usize {
        16 * (self.cells.iter().map(Vec::len).sum::<usize>()
            + self.physical.len()
            + self.protected.len()
            + 1)
    }
    fn intern(
        point: Point2,
        points: &mut Vec<Point2>,
        lookup: &mut Vec<usize>,
        b: &mut Budget,
    ) -> Result<usize, Failure> {
        let (mut low, mut high) = (0, lookup.len());
        while low < high {
            b.state([points.len(), lookup.len(), low, high, 0, 0, 0, 0]);
            b.step(1)?;
            let middle = low + (high - low) / 2;
            match points[lookup[middle]].cmp(&point) {
                std::cmp::Ordering::Equal => return Ok(lookup[middle]),
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
            }
        }
        // Charge each shifted lookup ordinal and the new record. Existing
        // cell/segment ordinals remain stable because point storage only appends.
        b.state([
            points.len(),
            lookup.len(),
            low,
            lookup.len() - low + 1,
            0,
            0,
            0,
            0,
        ]);
        b.step((lookup.len() - low + 1) as u64)?;
        b.retain(4)?;
        let ordinal = points.len();
        points.push(point);
        lookup.insert(low, ordinal);
        Ok(ordinal)
    }
    // The caller retains the full parent raw-region bound until this returns.
    // Each new owned record is additionally admitted before it is allocated.
    pub fn pack(arrangement: Arrangement, b: &mut Budget) -> Result<Self, Failure> {
        let entry = b.retained_memberships();
        let result = Self::pack_records(arrangement, b);
        if result.is_err() {
            b.release(b.retained_memberships() - entry);
        }
        result
    }
    fn pack_records(arrangement: Arrangement, b: &mut Budget) -> Result<Self, Failure> {
        b.step(1)?;
        b.state([
            arrangement.cells.len(),
            arrangement.physical.len(),
            arrangement.protected.len(),
            0,
            0,
            0,
            0,
            0,
        ]);
        b.retain(16 + 3 * arrangement.protected.len())?;
        let mut result = Self {
            points: Vec::new(),
            cells: Vec::new(),
            physical: Vec::new(),
            protected: arrangement.protected,
            work: arrangement.work,
        };
        // Keep the existing protected tree for contact membership. Its lexical
        // points seed a sorted ordinal lookup without rebuilding that tree.
        let mut lookup = Vec::new();
        for point in &result.protected {
            b.step(1)?;
            b.retain(4)?;
            lookup.push(result.points.len());
            result.points.push(point.clone());
        }
        for cell in arrangement.cells {
            b.step(1)?;
            b.retain(1)?;
            let mut ordinals = Vec::new();
            for point in cell {
                b.retain(1)?;
                ordinals.push(Self::intern(point, &mut result.points, &mut lookup, b)?);
            }
            result.cells.push(ordinals);
        }
        for [first, second] in arrangement.physical {
            b.retain(3)?;
            result.physical.push([
                Self::intern(first, &mut result.points, &mut lookup, b)?,
                Self::intern(second, &mut result.points, &mut lookup, b)?,
            ]);
        }
        let lookup_members = lookup.len();
        drop(lookup);
        b.release(lookup_members);
        Ok(result)
    }
    // The caller admits the complete expanded parent bound before cloning.
    // Consuming self drops all packed records before the caller releases them.
    pub fn expand(self, b: &mut Budget) -> Result<ExpandedRegion, Failure> {
        let read = |ordinal: usize, b: &mut Budget| -> Result<Point2, Failure> {
            b.step(1)?;
            self.points.get(ordinal).cloned().ok_or_else(|| {
                b.fail(
                    FailureKind::InvalidWitness,
                    "invalid packed region point ordinal",
                )
            })
        };
        let mut cells = Vec::new();
        for cell in self.cells {
            b.step(1)?;
            let mut points = Vec::new();
            for ordinal in cell {
                points.push(read(ordinal, b)?);
            }
            cells.push(points);
        }
        let mut physical = Vec::new();
        for [first, second] in self.physical {
            physical.push([read(first, b)?, read(second, b)?]);
        }
        Ok(ExpandedRegion {
            cells,
            physical,
            protected: self.protected,
            work: self.work,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, integer};

    fn triangle() -> Arrangement {
        let [a, c, d] = [[0, 0], [2, 0], [0, 2]].map(|p| p.map(integer));
        Arrangement {
            cells: vec![vec![a.clone(), c.clone(), d.clone()]],
            physical: [
                [a.clone(), c.clone()],
                [a.clone(), d.clone()],
                [d.clone(), c.clone()],
                [a.clone(), a.clone()],
                [c.clone(), c.clone()],
                [d.clone(), d.clone()],
            ]
            .into_iter()
            .collect(),
            protected: [a, c, d].into_iter().collect(),
            work: ArrangementWork {
                slabs: 1,
                classified_cells: 1,
                retained_cells: 1,
                ..ArrangementWork::default()
            },
        }
    }
    #[test]
    fn region_round_trip_preserves_every_record_and_guards_transfer_lifetimes()
    -> Result<(), Failure> {
        let original = triangle();
        let raw = PackedRegion::raw_memberships(
            &original.cells,
            original.physical.len(),
            original.protected.len(),
        );
        assert_eq!(raw, 208);
        let packed = 56;
        let lookup = 3;
        let mut b = Budget::new(Limits {
            storage_memberships: raw + packed + lookup,
            ..Limits::default()
        })?;
        b.retain(raw)?;
        let region = PackedRegion::pack(original.clone(), &mut b)?;
        assert_eq!(region.points.len(), 3);
        assert_eq!(region.memberships(), packed);
        assert_eq!(region.expanded_memberships(), raw);
        assert_eq!(b.retained_memberships(), raw + packed);
        b.release(raw);
        b.retain(raw)?;
        let expanded = region.expand(&mut b)?;
        b.release(packed);
        assert_eq!(expanded.cells, original.cells);
        assert_eq!(
            expanded.physical,
            original.physical.iter().cloned().collect::<Vec<_>>()
        );
        assert_eq!(expanded.protected, original.protected);
        assert_eq!(expanded.work, original.work);
        b.release(raw);
        assert_eq!(b.retained_memberships(), 0);
        assert_eq!(b.peak_admitted_memberships(), raw + packed + lookup);

        let mut short = Budget::new(Limits {
            storage_memberships: raw + packed + lookup - 1,
            ..Limits::default()
        })?;
        let result = short.with_retained(raw, |b| PackedRegion::pack(triangle(), b));
        assert_eq!(result.err().map(|e| e.kind), Some(FailureKind::Storage));
        assert_eq!(short.retained_memberships(), 0);
        assert!(short.peak_admitted_memberships() <= short.limits().storage_memberships);

        let mut b = Budget::new(Limits::default())?;
        let mut corrupt = b.with_retained(raw, |b| PackedRegion::pack(triangle(), b))?;
        corrupt.cells[0][0] = usize::MAX;
        let result = b.with_retained(raw, |b| corrupt.expand(b));
        assert_eq!(
            result.err().map(|e| e.kind),
            Some(FailureKind::InvalidWitness)
        );
        b.release(packed);
        assert_eq!(b.retained_memberships(), 0);
        Ok(())
    }
}
