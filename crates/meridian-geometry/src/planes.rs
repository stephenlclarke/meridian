use crate::exact::{add, dot, point, scale};
use crate::{Budget, Facet, Failure, FailureKind, Point3, Rational, integer};
use num_traits::Zero;
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

fn tree_work(count: usize, b: &mut Budget) -> Result<(), Failure> {
    b.step((count.saturating_add(1).ilog2() + 1).into())
}

pub(crate) struct PlaneGroups {
    ordinals: Vec<usize>,
    counts: BTreeMap<u32, usize>,
}
enum PlaneHit {
    Miss,
    Coplanar,
    Hit(Point3),
}
fn compute_hit(
    face: &Facet,
    origin: &Point3,
    direction: &Point3,
    b: &mut Budget,
) -> Result<PlaneHit, Failure> {
    b.step(1)?;
    let denominator = dot(face.normal(), direction);
    let numerator = face.offset() - dot(face.normal(), origin);
    let hit = if denominator.is_zero() {
        if numerator.is_zero() {
            PlaneHit::Coplanar
        } else {
            PlaneHit::Miss
        }
    } else {
        let t = numerator / denominator;
        if t <= integer(0) {
            PlaneHit::Miss
        } else {
            let hit = add(origin, &scale(direction, &t));
            point(&hit, b)?;
            PlaneHit::Hit(hit)
        }
    };
    Ok(hit)
}

impl PlaneGroups {
    pub fn memberships(&self) -> usize {
        8 + self.ordinals.len() + 3 * self.counts.len()
    }
    // The caller admits the temporary large-owner set. Only borrowed immutable
    // source coefficients enter the builder map; persistent data are ordinals.
    pub fn build(
        originals: &[Facet],
        owners: &BTreeSet<u32>,
        b: &mut Budget,
    ) -> Result<Self, Failure> {
        let entry = b.retained_memberships();
        let result = b.with_retained(8, |b| Self::build_records(originals, owners, b));
        if result.is_err() {
            b.release(b.retained_memberships() - entry);
        }
        result
    }
    fn build_records(
        originals: &[Facet],
        owners: &BTreeSet<u32>,
        b: &mut Budget,
    ) -> Result<Self, Failure> {
        b.retain(8 + originals.len())?;
        b.step(originals.len() as u64)?;
        let mut result = Self {
            ordinals: vec![0; originals.len()],
            counts: BTreeMap::new(),
        };
        let mut equations: BTreeMap<(u32, &Point3, &Rational), usize> = BTreeMap::new();
        for (ordinal, face) in originals.iter().enumerate() {
            b.step(1)?;
            tree_work(owners.len(), b)?;
            let owner = face.id().component;
            if !owners.contains(&owner) {
                continue;
            }
            tree_work(equations.len(), b)?;
            let local = match equations.entry((owner, face.normal(), face.offset())) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    b.retain(5)?;
                    tree_work(result.counts.len(), b)?;
                    let count = match result.counts.entry(owner) {
                        Entry::Occupied(count) => count.into_mut(),
                        Entry::Vacant(count) => {
                            b.retain(3)?;
                            count.insert(0)
                        }
                    };
                    b.step(1)?;
                    let local = *count;
                    *count += 1;
                    entry.insert(local);
                    local
                }
            };
            result.ordinals[ordinal] = local;
        }
        let transient = 5 * equations.len();
        drop(equations);
        b.release(transient);
        Ok(result)
    }
    pub fn count(&self, owner: u32, b: &mut Budget) -> Result<usize, Failure> {
        tree_work(self.counts.len(), b)?;
        self.counts
            .get(&owner)
            .copied()
            .ok_or_else(|| b.fail(FailureKind::InvalidWitness, "missing source plane owner"))
    }
    pub fn can_amortize(&self, facets: usize, count: usize) -> bool {
        let lookup = self.counts.len().saturating_add(1).ilog2() as usize + 1;
        2 * count + lookup < 3 * facets.saturating_sub(count)
    }
    pub fn cache_memberships(count: usize) -> usize {
        8 * count + 8
    }
    pub fn parity<'a>(
        &self,
        count: usize,
        faces: impl Iterator<Item = (usize, &'a Facet)>,
        origin: &Point3,
        direction: &Point3,
        b: &mut Budget,
    ) -> Result<Option<bool>, Failure> {
        b.with_retained(Self::cache_memberships(count), |b| {
            b.step(count as u64)?;
            let mut cache: Vec<Option<PlaneHit>> =
                std::iter::repeat_with(|| None).take(count).collect();
            let mut parity = false;
            for (ordinal, face) in faces {
                // The original per-facet visit price stays. Each triangle still
                // decides its own containment, edge ambiguity and parity bit.
                b.step(1)?;
                let local = *self.ordinals.get(ordinal).ok_or_else(|| {
                    b.fail(FailureKind::InvalidWitness, "missing source plane ordinal")
                })?;
                let entry = cache.get_mut(local).ok_or_else(|| {
                    b.fail(FailureKind::InvalidWitness, "invalid source plane ordinal")
                })?;
                let hit = match entry {
                    Some(hit) => hit,
                    None => {
                        let hit = compute_hit(face, origin, direction, b)?;
                        entry.insert(hit)
                    }
                };
                match hit {
                    PlaneHit::Miss => {}
                    PlaneHit::Coplanar => {
                        if face.contains(origin, b)?.0 {
                            return Ok(None);
                        }
                    }
                    PlaneHit::Hit(hit) => {
                        let (inside, edge) = face.contains(hit, b)?;
                        if !inside {
                            continue;
                        }
                        if edge {
                            return Ok(None);
                        }
                        parity = !parity;
                    }
                }
            }
            Ok(Some(parity))
        })
    }
}

pub(crate) struct CachedRay<'a> {
    pub groups: &'a PlaneGroups,
    pub owner: u32,
    pub count: usize,
    pub origin: &'a Point3,
    pub direction: &'a Point3,
}
struct CacheSlot {
    generation: u64,
    hit: PlaneHit,
}
struct OwnerCache {
    generation: u64,
    slots: Vec<CacheSlot>,
}
pub(crate) struct RayCaches {
    owners: BTreeMap<u32, OwnerCache>,
    retained: usize,
}
impl RayCaches {
    pub fn new(b: &mut Budget) -> Result<Self, Failure> {
        b.retain(8)?;
        Ok(Self {
            owners: BTreeMap::new(),
            retained: 8,
        })
    }
    pub fn memberships(&self) -> usize {
        self.retained
    }
    pub fn can_reuse(&self, groups: &PlaneGroups, count: usize) -> bool {
        // Bound the lookup by all possible owner tables, plus the generation
        // update. Keep the original scoped implementation when it is cheaper.
        let lookup = groups.counts.len().saturating_add(1).ilog2() as usize + 1;
        lookup + 1 < count
    }
    pub fn parity<'a>(
        &mut self,
        ray: CachedRay<'_>,
        faces: impl Iterator<Item = (usize, &'a Facet)>,
        b: &mut Budget,
    ) -> Result<Option<bool>, Failure> {
        // Covers new endpoint/hit scratch while a stale validated point remains
        // owned by its slot. Persistent slots include their generation record.
        b.with_retained(8, |b| {
            tree_work(self.owners.len(), b)?;
            let owner = match self.owners.entry(ray.owner) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    b.step(1)?;
                    let members = PlaneGroups::cache_memberships(ray.count) + ray.count;
                    b.retain(members)?;
                    if let Err(error) = b.step(ray.count as u64) {
                        b.release(members);
                        return Err(error);
                    }
                    let slots = std::iter::repeat_with(|| CacheSlot {
                        generation: 0,
                        hit: PlaneHit::Miss,
                    })
                    .take(ray.count)
                    .collect();
                    self.retained += members;
                    entry.insert(OwnerCache {
                        generation: 0,
                        slots,
                    })
                }
            };
            if owner.slots.len() != ray.count {
                return Err(b.fail(FailureKind::InvalidWitness, "changed plane cache owner"));
            }
            b.step(1)?;
            let generation = owner.generation.checked_add(1).ok_or_else(|| {
                b.fail(FailureKind::Operations, "plane cache generation overflow")
            })?;
            owner.generation = generation;
            let mut parity = false;
            for (ordinal, face) in faces {
                b.step(1)?;
                let local = *ray.groups.ordinals.get(ordinal).ok_or_else(|| {
                    b.fail(FailureKind::InvalidWitness, "missing source plane ordinal")
                })?;
                let slot = owner.slots.get_mut(local).ok_or_else(|| {
                    b.fail(FailureKind::InvalidWitness, "invalid source plane ordinal")
                })?;
                if slot.generation != generation {
                    // Publish this generation only after fresh computation and
                    // every original coordinate guard have succeeded.
                    slot.hit = compute_hit(face, ray.origin, ray.direction, b)?;
                    slot.generation = generation;
                }
                match &slot.hit {
                    PlaneHit::Miss => {}
                    PlaneHit::Coplanar => {
                        if face.contains(ray.origin, b)?.0 {
                            return Ok(None);
                        }
                    }
                    PlaneHit::Hit(hit) => {
                        let (inside, edge) = face.contains(hit, b)?;
                        if !inside {
                            continue;
                        }
                        if edge {
                            return Ok(None);
                        }
                        parity = !parity;
                    }
                }
            }
            Ok(Some(parity))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceId, Vertex, fraction};

    fn faces(b: &mut Budget) -> Result<Vec<Facet>, Failure> {
        let first = [[0, 0, 1], [1, 0, 1], [0, 1, 1]];
        let second = [[1, 0, 1], [1, 1, 1], [0, 1, 1]];
        let scaled = [[3, 0, 1], [5, 0, 1], [3, 2, 1]];
        [
            (0, first),
            (1, first),
            (0, second),
            (1, second),
            (0, scaled),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (owner, points))| {
            Facet::new(
                SourceId {
                    component: owner,
                    facet: i as u32,
                },
                points.map(|p| Vertex {
                    point: p.map(integer),
                    uv: [integer(0), integer(0)],
                }),
                String::new(),
                b,
            )
        })
        .collect()
    }
    #[test]
    fn exact_plane_cache_preserves_individual_contacts_and_guarded_lifetimes() -> Result<(), Failure>
    {
        let mut b = Budget::new(Limits::default())?;
        let faces = faces(&mut b)?;
        let owners = [0, 1].into_iter().collect::<BTreeSet<_>>();
        let groups = PlaneGroups::build(&faces, &owners, &mut b)?;
        assert_eq!(groups.count(0, &mut b)?, 2);
        assert_eq!(groups.count(1, &mut b)?, 1);
        assert_eq!(groups.ordinals, vec![0, 0, 0, 0, 1]);
        assert_eq!(groups.memberships(), 19);
        assert_eq!(b.retained_memberships(), 19);
        let quarter = fraction("1", "4", &mut b)?;
        let half = fraction("1", "2", &mut b)?;
        for (origin, direction, expected) in [
            (
                [quarter.clone(), quarter.clone(), integer(0)],
                [0, 0, 1],
                Some(true),
            ),
            ([half.clone(), half, integer(0)], [0, 0, 1], None),
            (
                [quarter.clone(), quarter.clone(), integer(1)],
                [1, 0, 0],
                None,
            ),
            (
                [quarter.clone(), quarter, integer(0)],
                [0, 0, -1],
                Some(false),
            ),
        ] {
            for owner in [0, 1] {
                let count = groups.count(owner, &mut b)?;
                let result = groups.parity(
                    count,
                    faces
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| f.id().component == owner),
                    &origin,
                    &direction.map(integer),
                    &mut b,
                )?;
                assert_eq!(result, expected);
                assert_eq!(b.retained_memberships(), 19);
            }
        }
        let members = groups.memberships();
        drop(groups);
        b.release(members);
        assert_eq!(b.retained_memberships(), 0);
        // Five ordinals, two owner-count records and headers own19 slots.
        // The two-plane attempt owns24 more; builder peak is42.
        for (limit, expected) in [(43, Some(true)), (42, None)] {
            let mut tight = Budget::new(Limits {
                storage_memberships: limit,
                ..Limits::default()
            })?;
            let groups = PlaneGroups::build(&faces, &owners, &mut tight)?;
            let count = groups.count(0, &mut tight)?;
            let result = groups.parity(
                count,
                faces
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.id().component == 0),
                &[
                    fraction("1", "4", &mut b)?,
                    fraction("1", "4", &mut b)?,
                    integer(0),
                ],
                &[0, 0, 1].map(integer),
                &mut tight,
            );
            if let Some(expected) = expected {
                assert_eq!(result?, Some(expected));
            } else {
                assert_eq!(result.err().map(|e| e.kind), Some(FailureKind::Storage));
            }
            let members = groups.memberships();
            drop(groups);
            tight.release(members);
            assert_eq!(tight.retained_memberships(), 0);
            assert!(tight.peak_admitted_memberships() <= limit);
        }
        let mut short = Budget::new(Limits {
            storage_memberships: 41,
            ..Limits::default()
        })?;
        assert_eq!(
            PlaneGroups::build(&faces, &owners, &mut short)
                .err()
                .map(|e| e.kind),
            Some(FailureKind::Storage)
        );
        assert_eq!(short.retained_memberships(), 0);
        Ok(())
    }
    #[test]
    fn reusable_storage_invalidates_hits_and_retains_guards_between_attempts() -> Result<(), Failure>
    {
        let mut b = Budget::new(Limits::default())?;
        let faces = faces(&mut b)?;
        let owners = [0, 1].into_iter().collect::<BTreeSet<_>>();
        let groups = PlaneGroups::build(&faces, &owners, &mut b)?;
        let mut caches = RayCaches::new(&mut b)?;
        let quarter = fraction("1", "4", &mut b)?;
        let half = fraction("1", "2", &mut b)?;
        let origin = [quarter.clone(), quarter.clone(), integer(0)];
        let forward = [0, 0, 1].map(integer);
        let run = |caches: &mut RayCaches, origin: &Point3, direction: &Point3, b: &mut Budget| {
            caches.parity(
                CachedRay {
                    groups: &groups,
                    owner: 0,
                    count: 2,
                    origin,
                    direction,
                },
                faces
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.id().component == 0),
                b,
            )
        };
        let start = b.operations();
        assert_eq!(run(&mut caches, &origin, &forward, &mut b)?, Some(true));
        let first = b.operations() - start;
        let retained = b.retained_memberships();
        let start = b.operations();
        assert_eq!(run(&mut caches, &origin, &forward, &mut b)?, Some(true));
        let second = b.operations() - start;
        assert!(second < first, "first={first} second={second}");
        assert_eq!(b.retained_memberships(), retained);
        assert_eq!(caches.owners.get(&0).map(|o| o.generation), Some(2));
        for (origin, direction, expected) in [
            (
                [integer(10), integer(10), integer(0)],
                [0, 0, 1],
                Some(false),
            ),
            (
                [quarter.clone(), quarter.clone(), integer(0)],
                [0, 0, -1],
                Some(false),
            ),
            (
                [quarter.clone(), quarter.clone(), integer(1)],
                [1, 0, 0],
                None,
            ),
            ([half.clone(), half, integer(0)], [0, 0, 1], None),
        ] {
            assert_eq!(
                run(&mut caches, &origin, &direction.map(integer), &mut b)?,
                expected
            );
            assert_eq!(b.retained_memberships(), retained);
        }
        // A fresh generation must revalidate its new hit, even though this
        // plane previously held a valid point for another ray.
        let bound = num_bigint::BigInt::from(1) << 4095usize;
        let huge = fraction(&bound.to_string(), "1", &mut b)?;
        let result = run(
            &mut caches,
            &origin,
            &[huge, integer(0), integer(1)],
            &mut b,
        );
        assert_eq!(result.err().map(|e| e.kind), Some(FailureKind::Rational));
        assert_eq!(b.retained_memberships(), retained);
        assert_eq!(run(&mut caches, &origin, &forward, &mut b)?, Some(true));
        assert_eq!(
            caches.parity(
                CachedRay {
                    groups: &groups,
                    owner: 1,
                    count: 1,
                    origin: &origin,
                    direction: &forward
                },
                faces
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.id().component == 1),
                &mut b
            )?,
            Some(true)
        );
        assert_eq!(caches.memberships(), 51);
        let cache_members = caches.memberships();
        drop(caches);
        b.release(cache_members);
        let group_members = groups.memberships();
        drop(groups);
        b.release(group_members);
        assert_eq!(b.retained_memberships(), 0);

        for (limit, success) in [(61, true), (60, false)] {
            let mut tight = Budget::new(Limits {
                storage_memberships: limit,
                ..Limits::default()
            })?;
            let groups = PlaneGroups::build(&faces, &owners, &mut tight)?;
            let mut caches = RayCaches::new(&mut tight)?;
            let result = caches.parity(
                CachedRay {
                    groups: &groups,
                    owner: 0,
                    count: 2,
                    origin: &origin,
                    direction: &forward,
                },
                faces
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.id().component == 0),
                &mut tight,
            );
            if success {
                assert_eq!(result?, Some(true));
                assert_eq!(tight.peak_admitted_memberships(), 61);
            } else {
                assert_eq!(result.err().map(|e| e.kind), Some(FailureKind::Storage));
                assert_eq!(caches.memberships(), 8);
            }
            let cache_members = caches.memberships();
            drop(caches);
            tight.release(cache_members);
            let group_members = groups.memberships();
            drop(groups);
            tight.release(group_members);
            assert_eq!(tight.retained_memberships(), 0);
            assert!(tight.peak_admitted_memberships() <= limit);
        }
        // Refusing the one-time slot-initialization charge must also undo
        // its successful memory admission, before any owner table publishes.
        let mut measure = Budget::new(Limits::default())?;
        let groups = PlaneGroups::build(&faces, &owners, &mut measure)?;
        let caches = RayCaches::new(&mut measure)?;
        let spent = measure.operations();
        let cache_members = caches.memberships();
        drop(caches);
        measure.release(cache_members);
        let group_members = groups.memberships();
        drop(groups);
        measure.release(group_members);
        let mut work_short = Budget::new(Limits {
            operations: spent + 3,
            ..Limits::default()
        })?;
        let groups = PlaneGroups::build(&faces, &owners, &mut work_short)?;
        let mut caches = RayCaches::new(&mut work_short)?;
        let result = caches.parity(
            CachedRay {
                groups: &groups,
                owner: 0,
                count: 2,
                origin: &origin,
                direction: &forward,
            },
            faces
                .iter()
                .enumerate()
                .filter(|(_, f)| f.id().component == 0),
            &mut work_short,
        );
        assert_eq!(result.err().map(|e| e.kind), Some(FailureKind::Operations));
        assert!(caches.owners.is_empty());
        assert_eq!(caches.memberships(), 8);
        let cache_members = caches.memberships();
        drop(caches);
        work_short.release(cache_members);
        let group_members = groups.memberships();
        drop(groups);
        work_short.release(group_members);
        assert_eq!(work_short.retained_memberships(), 0);
        Ok(())
    }
}
