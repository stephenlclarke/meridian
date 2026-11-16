//! Sight is rebuilt from actual native station triangles and retained World-owned body origins.
use super::{Sample, SightBody, World, length, sub, unit};
use crate::docking::{RigidPose, Vec3};
use crate::flight::practice::lifecycle::lessons;

impl World {
    pub(super) fn flight_reception_sample(&self, visit: u64) -> Option<Sample> {
        let data = self.station_data.as_ref()?;
        if self.state.station != Some(data.station_pose()) {
            return None;
        }
        let mut flight = self.state;
        flight.station = None;
        flight.navigation = None;
        let mut bodies = [None; 9];
        for p in self
            .missile_projectiles()
            .filter(|p| p.ended.is_none() && p.origin.owner.visit == visit)
        {
            let ordinal = (p.origin.projectile.entity() - 5) as u8;
            let slot = usize::from(ordinal) % 9;
            if bodies[slot].is_some() {
                return None;
            }
            bodies[slot] = Some(SightBody {
                ordinal,
                position: p.position,
            });
        }
        Some(Sample {
            flight,
            visit,
            physical_tick: self.combat.physical_tick(),
            bodies,
        })
    }
    pub(super) fn flight_sample_sight_clear(&self, sample: Sample) -> bool {
        let Some(pose) = sample.pose() else {
            return false;
        };
        let Some(direction) = unit(pose.orientation.z) else {
            return false;
        };
        let Some(data) = self.station_data.as_ref() else {
            return false;
        };
        let Ok(station) = data.beam_block_distance(
            data.station_pose(),
            pose.position,
            direction,
            sample.range(),
        ) else {
            return false;
        };
        let limit = station.unwrap_or(sample.range());
        let ray = RigidPose {
            position: pose.position,
            orientation: crate::docking::Basis3 {
                z: direction,
                ..pose.orientation
            },
        };
        for body in sample.bodies.into_iter().flatten() {
            if self
                .retained_flight_body_distance(usize::from(body.ordinal), body.position, ray, limit)
                .is_some_and(|d| d < limit)
            {
                return false;
            }
        }
        !lessons(self.practice, &self.practice_history).any(|lesson| {
            lesson.flight_sight_blocked(
                sample.flight.tick,
                sample.visit,
                ray,
                sample.range(),
                station,
            )
        })
    }
    pub(super) fn flight_sample_bodies_valid(&self, sample: Sample, end_boundary: bool) -> bool {
        for p in self.missile_projectiles() {
            let required = p.origin.owner.visit == sample.visit
                && (p.origin.launched_tick < sample.flight.tick
                    || (p.origin.launched_tick == sample.flight.tick && !end_boundary))
                && (p.ended.is_none() || p.last_tick > sample.flight.tick);
            if required
                && !sample
                    .bodies
                    .iter()
                    .flatten()
                    .any(|b| u64::from(b.ordinal) + 5 == p.origin.projectile.entity())
            {
                return false;
            }
        }
        for (i, stored) in sample.bodies.into_iter().enumerate() {
            let projectile = stored.and_then(|b| {
                self.missile_projectiles()
                    .find(|p| u64::from(b.ordinal) + 5 == p.origin.projectile.entity())
            });
            let at_visit = projectile.filter(|p| {
                p.origin.owner.visit == sample.visit && p.origin.launched_tick <= sample.flight.tick
            });
            // External firing/interception may follow the final endpoint at that same tick.
            // At the first endpoint all commands at that tick precede this atomic step.
            let required = at_visit.is_some_and(|p| {
                (p.origin.launched_tick < sample.flight.tick || !end_boundary)
                    && (p.ended.is_none() || p.last_tick > sample.flight.tick)
            });
            if required && stored.is_none() {
                return false;
            }
            let Some(body) = stored else {
                continue;
            };
            let Some(p) = at_visit else {
                return false;
            };
            let elapsed = sample.flight.tick.saturating_sub(p.origin.launched_tick);
            let travel_bound = elapsed.min(455) as f64 * (crate::MISSILE_SPEED / 60.0) + 1e-7;
            if usize::from(body.ordinal) % 9 != i
                || !finite(body.position)
                || (p.ended.is_some()
                    && (p.last_tick < sample.flight.tick
                        || (!end_boundary && p.last_tick == sample.flight.tick)))
                || length(sub(body.position, p.origin.muzzle)) > travel_bound
            {
                return false;
            }
        }
        true
    }
}
fn finite(v: Vec3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

#[cfg(test)]
mod paid_body_tests {
    use super::*;
    use crate::flight::ammunition::AmmunitionAction;
    use crate::flight::missiles::tests::{fire, stop};
    use crate::flight::practice::tests::{consent, launch, step};

    #[test]
    fn fourth_actual_projectile_is_required_in_retained_flight_sight() {
        let mut w = World::new_commander(8501).unwrap_or_else(|e| panic!("{e:?}"));
        for _ in 0..2 {
            let review = w
                .review_ammunition(AmmunitionAction::Purchase, 0)
                .unwrap_or_else(|e| panic!("{e:?}"));
            w.prepare_ammunition(review, 0)
                .unwrap_or_else(|e| panic!("{e:?}"))
                .commit();
        }
        consent(&mut w);
        launch(&mut w);
        stop(&mut w);
        let target = w
            .practice_snapshot()
            .and_then(|p| p.beacons)
            .unwrap_or_else(|| panic!("no real beacons"))[1]
            .id;
        for _ in 0..3 {
            fire(&mut w, target);
            for _ in 0..100 {
                step(&mut w, 1);
            }
        }
        let fourth = fire(&mut w, target);
        assert_eq!(fourth.projectile.entity(), 8);
        let mut sample = w
            .flight_reception_sample(fourth.owner.visit)
            .unwrap_or_else(|| panic!("no actual flight sample"));
        assert!(sample.bodies[3].is_some());
        assert!(w.flight_sample_bodies_valid(sample, false));
        sample.bodies[3] = None;
        assert!(!w.flight_sample_bodies_valid(sample, false));
    }
}
