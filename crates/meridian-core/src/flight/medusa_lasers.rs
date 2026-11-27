//! The three integral Naval mounts belong to the first ordinary Fleet Medusa issue.
use super::{FlightView, FrontLaserSnapshot, OrdinaryWeapon, World};
use crate::docking::{Basis3, RigidPose, Vec3};

pub(super) const INTEGRAL: [FlightView; 3] =
    [FlightView::Left, FlightView::Right, FlightView::Rear];

impl FlightView {
    pub(super) fn integral_tag(self, base: u8) -> u8 {
        match self {
            Self::Left => base,
            Self::Right => base + 1,
            Self::Rear => base + 2,
            Self::Front => u8::MAX,
        }
    }
    pub(super) fn laser_ray(self, body: RigidPose) -> RigidPose {
        let neg = |v: Vec3| Vec3::new(-v.x, -v.y, -v.z);
        let b = body.orientation;
        let (x, z) = match self {
            Self::Front => (b.x, b.z),
            Self::Left => (b.z, neg(b.x)),
            Self::Right => (neg(b.z), b.x),
            Self::Rear => (neg(b.x), neg(b.z)),
        };
        RigidPose {
            position: body.position,
            orientation: Basis3 { x, y: b.y, z },
        }
    }
}
impl World {
    pub fn selected_laser_view(&self) -> FlightView {
        if self.xyzzy_medusa_active() {
            self.flight_view()
        } else {
            FlightView::Front
        }
    }
    pub(super) fn integral_laser(&self, facing: FlightView) -> Option<u64> {
        let index = INTEGRAL.iter().position(|v| *v == facing)? as u64;
        let (dock, _, revision) = self.xyzzy_medusa_issue()?;
        // All older ship inventory, ammunition and EMP IDs use offsets through 400011.
        let offset = revision.checked_mul(3)?.checked_add(500_011 + index)?;
        let owner = dock.owner().seed();
        owner
            .checked_add(offset)
            .or_else(|| owner.checked_sub(offset))
    }
    pub(super) fn selected_fit(&self) -> Option<(u64, OrdinaryWeapon)> {
        let facing = self.selected_laser_view();
        if facing == FlightView::Front {
            self.fitted_front_laser()
        } else {
            Some((
                self.integral_laser(facing)?,
                OrdinaryWeapon::FrontNavalLaser,
            ))
        }
    }
    pub fn selected_laser_snapshot(&self) -> Option<FrontLaserSnapshot> {
        let (fitted_item_id, weapon) = self.selected_fit()?;
        Some(FrontLaserSnapshot {
            fitted_item_id,
            weapon,
            damage_hundredths: weapon.damage_hundredths(),
            range: 6000,
            cooldown_ticks_per_shot: weapon.cooldown_ticks(),
        })
    }
    pub(super) fn integral_view_at(&self, item: u64, tick: u64) -> Option<FlightView> {
        let (_, issue_tick, _) = self.xyzzy_medusa_issue()?;
        if tick <= issue_tick {
            return None;
        }
        INTEGRAL
            .into_iter()
            .find(|v| self.integral_laser(*v) == Some(item))
    }
    pub(super) fn laser_weapon_at_tick(
        &self,
        item: u64,
        tick: u64,
        emission: u64,
    ) -> Option<OrdinaryWeapon> {
        if self.integral_view_at(item, tick).is_some() {
            Some(OrdinaryWeapon::FrontNavalLaser)
        } else {
            self.front_weapon_at_tick(item, tick, emission)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight::practice::tests::{launch, step};
    use crate::{FireError, FlightInput, ModuleAction, XyzzyAction};
    fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|e| panic!("{e:?}"))
    }
    fn issue(w: &mut World) {
        let tick = w.snapshot().tick;
        let review = ok(w.review_xyzzy(XyzzyAction::ActivateMedusa, tick));
        ok(w.prepare_xyzzy(review, tick)).commit();
    }
    fn cold(w: &World) -> World {
        ok(World::from_save_bytes(&ok(w.save_bytes())))
    }
    fn select(w: &mut World, view: FlightView) {
        ok(w.request_flight_view(view, w.snapshot().tick));
    }
    #[test]
    fn sh171_lasers_four_owned_emissions_follow_the_current_body_axes_after_continue() {
        let mut w = ok(World::new_commander(31811));
        issue(&mut w);
        let first = w.xyzzy_medusa_issue();
        let ids = INTEGRAL.map(|f| w.integral_laser(f));
        issue(&mut w);
        let mut w = cold(&w);
        assert_eq!(w.xyzzy_medusa_issue(), first);
        assert_eq!(INTEGRAL.map(|f| w.integral_laser(f)), ids);
        launch(&mut w);
        let mut fired = std::collections::BTreeSet::new();
        for f in [
            FlightView::Rear,
            FlightView::Left,
            FlightView::Right,
            FlightView::Front,
        ] {
            select(&mut w, f);
            let body = super::super::flight_pose(w.state.position, w.state.orientation)
                .unwrap_or_else(|| panic!("missing body"));
            let expected = match f {
                FlightView::Left => Vec3::new(
                    -body.orientation.x.x,
                    -body.orientation.x.y,
                    -body.orientation.x.z,
                ),
                FlightView::Right => body.orientation.x,
                FlightView::Rear => Vec3::new(
                    -body.orientation.z.x,
                    -body.orientation.z.y,
                    -body.orientation.z.z,
                ),
                FlightView::Front => body.orientation.z,
            };
            let fit = w
                .selected_laser_snapshot()
                .unwrap_or_else(|| panic!("missing selected fit"));
            let before = w
                .flight_resources_snapshot()
                .unwrap_or_else(|| panic!("missing bus"));
            let shot = ok(w.request_fire_selected_laser(w.snapshot().tick));
            assert_eq!(fit.weapon, OrdinaryWeapon::FrontNavalLaser);
            assert_eq!(
                (shot.origin, shot.direction, shot.fitted_item_id),
                (body.position, expected, fit.fitted_item_id)
            );
            assert!(fired.insert(shot.fitted_item_id));
            let after = w
                .flight_resources_snapshot()
                .unwrap_or_else(|| panic!("missing bus"));
            assert!(
                after.energy_hundredths < before.energy_hundredths
                    && after.heat_fifths > before.heat_fifths
            );
            assert_eq!(after.cooldown_ticks, fit.cooldown_ticks_per_shot);
            let mut restored = cold(&w);
            assert_eq!(restored.flight_resources_snapshot(), Some(after));
            assert_eq!(restored.selected_laser_snapshot(), Some(fit));
            select(
                &mut restored,
                if f == FlightView::Rear {
                    FlightView::Left
                } else {
                    FlightView::Rear
                },
            );
            assert_eq!(
                restored.request_fire_selected_laser(restored.snapshot().tick),
                Err(FireError::CoolingDown)
            );
            w = restored;
            step(&mut w, after.cooldown_ticks);
        }
        assert_eq!(fired.len(), 4);
    }
    #[test]
    fn sh171_lasers_keep_the_front_item_removable_and_leave_other_ships_unchanged() {
        let mut w = ok(World::new_commander(31812));
        launch(&mut w);
        select(&mut w, FlightView::Right);
        assert_eq!(w.selected_laser_view(), FlightView::Front);
        assert_eq!(w.selected_laser_snapshot(), w.front_laser_snapshot());
        assert!(INTEGRAL.iter().all(|v| w.integral_laser(*v).is_none()));
        let preview = ok(World::new_medusa_preview());
        assert!(preview.selected_laser_snapshot().is_none());
        let mut w = ok(World::new_commander(31813));
        issue(&mut w);
        let front = w
            .front_laser_snapshot()
            .unwrap_or_else(|| panic!("no front laser"));
        let tick = w.snapshot().tick;
        let review = ok(w.review_module(
            ModuleAction::Store {
                item: front.fitted_item_id,
            },
            tick,
        ));
        ok(w.prepare_module(review, tick)).commit();
        let mut w = cold(&w);
        launch(&mut w);
        assert!(w.selected_laser_snapshot().is_none());
        assert_eq!(
            w.request_fire_selected_laser(w.snapshot().tick),
            Err(FireError::NoFittedFrontLaser)
        );
        select(&mut w, FlightView::Left);
        assert!(w.request_fire_front_laser(w.snapshot().tick).is_err());
        let shot = ok(w.request_fire_selected_laser(w.snapshot().tick));
        assert_ne!(shot.fitted_item_id, front.fitted_item_id);
        ok(w.step(FlightInput::default()));
        assert_eq!(
            cold(&w).selected_laser_snapshot(),
            w.selected_laser_snapshot()
        );
    }
}
