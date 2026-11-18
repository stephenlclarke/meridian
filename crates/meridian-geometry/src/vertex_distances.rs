//! Exact original-point distances for one active original source plane.
use super::{JointSection, joint_section_distances, log_cost};
use crate::{Budget, Facet, Failure, FailureKind, Point3, Rational, integer};
use std::collections::{BTreeMap, btree_map::Entry};

const MIN_FACETS: usize = 32 * super::FacetBoundsIndex::LEAF_SIZE;
const ID_BITS: usize = 21;
const ID_MASK: u64 = (1u64 << ID_BITS) - 1;

pub(in crate::certificate) struct VertexDistances<'a> {
    originals: &'a [Facet],
    ids: Vec<u64>,
    values: Vec<Rational>,
    bits: Vec<u64>,
    epochs: Vec<u64>,
    epoch: u64,
    members: usize,
}
pub(in crate::certificate) struct Plane<'q, 'f, 'a> {
    cache: &'q mut VertexDistances<'a>,
    face: &'f Facet,
    epoch: u64,
}
impl<'a> VertexDistances<'a> {
    pub(in crate::certificate) fn build(
        originals: &'a [Facet],
        b: &mut Budget,
    ) -> Result<Option<Self>, Failure> {
        if originals.len() < MIN_FACETS {
            return Ok(None);
        }
        let entry = b.retained_memberships();
        let result = b.with_retained(32, |b| Self::build_admitted(originals, b));
        if result.is_err() {
            b.release(b.retained_memberships() - entry);
        }
        result.map(Some)
    }
    fn build_admitted(originals: &'a [Facet], b: &mut Budget) -> Result<Self, Failure> {
        let count = originals.len();
        b.state([count, 0, 0, 0, count, 0, 32, 0]);
        b.retain(32 + count)?;
        b.step(count as u64)?;
        let mut ids = vec![0u64; count];
        let mut values = Vec::new();
        let mut points: BTreeMap<&'a Point3, usize> = BTreeMap::new();
        for (ordinal, face) in originals.iter().enumerate() {
            b.at(face.id());
            b.step(1)?;
            let mut packed = 0;
            for (vertex, source) in face.vertices().iter().enumerate() {
                b.state([count, ordinal, vertex, points.len(), values.len(), 0, 32, 0]);
                log_cost(points.len(), b)?;
                let id = match points.entry(&source.point) {
                    Entry::Occupied(entry) => *entry.get(),
                    Entry::Vacant(entry) => {
                        // Borrowed key, ID and map membership are transient;
                        // the exact scalar value slot remains after map drop.
                        b.retain(4)?;
                        b.step(1)?;
                        let id = values.len();
                        values.push(integer(0));
                        entry.insert(id);
                        id
                    }
                };
                if id as u64 > ID_MASK {
                    return Err(b.fail(
                        FailureKind::InvalidWitness,
                        "original vertex ID exceeds packed field",
                    ));
                }
                b.step(1)?;
                packed |= (id as u64) << (ID_BITS * vertex);
            }
            b.step(1)?;
            ids[ordinal] = packed;
        }
        let unique = values.len();
        drop(points);
        b.release(3 * unique);
        let words = unique.div_ceil(64);
        b.retain(2 * words)?;
        b.step((2 * words) as u64)?;
        Ok(Self {
            originals,
            ids,
            values,
            bits: vec![0; words],
            epochs: vec![0; words],
            epoch: 0,
            members: 32 + count + unique + 2 * words,
        })
    }
    pub(in crate::certificate) fn memberships(&self) -> usize {
        self.members
    }
    pub(super) fn originals(&self) -> &'a [Facet] {
        self.originals
    }
    pub(in crate::certificate) fn plane<'q, 'f>(
        &'q mut self,
        face: &'f Facet,
        b: &mut Budget,
    ) -> Result<Plane<'q, 'f, 'a>, Failure> {
        let epoch = self.epoch.checked_add(1).ok_or_else(|| {
            b.fail(
                FailureKind::InvalidWitness,
                "source-plane distance generation exhausted",
            )
        })?;
        b.step(1)?;
        b.at(face.id());
        self.epoch = epoch;
        Ok(Plane {
            cache: self,
            face,
            epoch,
        })
    }
}
impl<'a> Plane<'_, '_, 'a> {
    pub(in crate::certificate) fn section(
        &mut self,
        ordinal: usize,
        physical: bool,
        outward: bool,
        b: &mut Budget,
    ) -> Result<JointSection, Failure> {
        let originals: &'a [Facet] = self.cache.originals;
        let other = originals.get(ordinal).ok_or_else(|| {
            b.fail(
                FailureKind::InvalidWitness,
                "distance facet ordinal outside source",
            )
        })?;
        let packed =
            *self.cache.ids.get(ordinal).ok_or_else(|| {
                b.fail(FailureKind::InvalidWitness, "missing original vertex IDs")
            })?;
        // These three fixed metadata fields are read under the caller's actual
        // facet visit, just as existing plane/ray slots use source ordinals.
        let ids: [usize; 3] =
            std::array::from_fn(|vertex| ((packed >> (ID_BITS * vertex)) & ID_MASK) as usize);
        for (vertex, id) in ids.iter().copied().enumerate() {
            if id >= self.cache.values.len() {
                return Err(b.fail(
                    FailureKind::InvalidWitness,
                    "distance vertex ordinal outside source",
                ));
            }
            let word = id / 64;
            let mask = 1u64 << (id % 64);
            b.state([
                originals.len(),
                self.cache.values.len(),
                ordinal,
                vertex,
                id,
                word,
                self.epoch as usize,
                0,
            ]);
            if self.cache.epochs[word] != self.epoch {
                b.step(2)?;
                self.cache.bits[word] = 0;
                self.cache.epochs[word] = self.epoch;
            }
            if self.cache.bits[word] & mask == 0 {
                b.step(1)?;
                self.cache.values[id] = self.face.distance(&other.vertices()[vertex].point);
                self.cache.bits[word] |= mask;
            }
        }
        let face = self.face;
        b.state([
            originals.len(),
            self.cache.values.len(),
            ordinal,
            usize::from(physical) + 2 * usize::from(outward),
            ids[0],
            ids[1],
            ids[2],
            self.epoch as usize,
        ]);
        let distances = ids.map(|id| &self.cache.values[id]);
        joint_section_distances(face, other, physical, outward, distances, b)
    }
}
