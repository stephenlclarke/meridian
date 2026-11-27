//! Pilot sight in the same viewport and facing as the projected flight geometry.
use super::{Vertex, flight_height, line};
use meridian_core::{FlightMode, OrdinaryWeapon, Snapshot, World, equipment::WorkTargetId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Sight {
    Pulse,
    Beam,
    Military,
    Naval,
    Mining,
    Cutter,
    Scoop,
}

impl Sight {
    pub(super) fn capture(world: &World, flight: Snapshot) -> Option<Self> {
        if flight.mode != FlightMode::Flight
            || flight.navigation?.travel.is_some()
            || world.combat_snapshot().is_some_and(|c| c.player.destroyed)
            || world.selected_laser_view() != world.flight_view()
        {
            return None;
        }
        use crate::app::FlightTrigger;
        match FlightTrigger::current(world) {
            FlightTrigger::MountedLaser => Self::laser(world.selected_laser_snapshot()?.weapon),
            FlightTrigger::Manual(WorkTargetId::Wreck(_)) => {
                world.fitted_salvage_cutter().map(|_| Self::Cutter)
            }
            FlightTrigger::Manual(WorkTargetId::Ore(_)) => {
                world.fitted_fuel_scoops().map(|_| Self::Scoop)
            }
            FlightTrigger::Manual(WorkTargetId::Asteroid(_)) => {
                world.fitted_mining_tool().map(|_| Self::Mining)
            }
            FlightTrigger::OccupiedReview => None,
        }
    }

    fn laser(weapon: OrdinaryWeapon) -> Option<Self> {
        Some(match weapon {
            OrdinaryWeapon::FrontPulseLaser => Self::Pulse,
            OrdinaryWeapon::FrontBeamLaser => Self::Beam,
            OrdinaryWeapon::FrontMilitaryLaser => Self::Military,
            OrdinaryWeapon::FrontNavalLaser => Self::Naval,
            OrdinaryWeapon::FrontMiningLaser => Self::Mining,
            OrdinaryWeapon::StandardMissile => return None,
        })
    }

    pub(super) fn draw(self, width: u32, height: u32, out: &mut Vec<Vertex>) {
        if width < 48 || height < 48 {
            return;
        }
        let size = [width as f32, height as f32];
        let centre = [size[0] * 0.5, flight_height(width, height) * 0.5];
        // One arm in logical pixels. Rotate it about the bore, keeping the contact visible.
        let (color, strokes): (_, &[[f32; 4]]) = match self {
            Self::Pulse => ([0.5, 1.0, 0.6], &[[5., 0., 13., 0.]]),
            Self::Beam => ([0.35, 0.85, 1.0], &[[6., -2., 14., -2.], [6., 2., 14., 2.]]),
            Self::Military => ([1.0, 0.6, 0.35], &[[6., 0., 13., -4.], [6., 0., 13., 4.]]),
            Self::Naval => ([0.85, 0.6, 1.0], &[[6., 0., 15., 0.], [11., -3., 11., 3.]]),
            Self::Mining => ([1.0, 0.85, 0.25], &[[6., 0., 0., 6.], [9., 0., 14., 0.]]),
            Self::Cutter => ([1.0, 0.45, 0.3], &[[5., 5., 12., 5.], [12., 5., 12., 1.]]),
            Self::Scoop => ([0.55, 0.9, 0.9], &[[5., 7., 10., 7.], [10., 7., 10., 12.]]),
        };
        for turn in 0..4 {
            let point = |x: f32, y: f32| {
                let [dx, dy] = match turn {
                    0 => [x, y],
                    1 => [-y, x],
                    2 => [-x, -y],
                    _ => [y, -x],
                };
                [centre[0] + dx, centre[1] + dy]
            };
            for &[x, y, u, v] in strokes {
                line(out, point(x, y), point(u, v), color, size);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_core::FlightView;
    fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
        value.unwrap_or_else(|error| panic!("{error:?}"))
    }

    #[test]
    fn empty_front_mount_clears_the_sight_and_is_named_on_the_status_card() {
        let mut world = ok(World::new_commander(7413));
        let fit = world
            .front_laser_snapshot()
            .unwrap_or_else(|| panic!("starter laser"));
        let review = ok(world.review_module(
            meridian_core::ModuleAction::Store {
                item: fit.fitted_item_id,
            },
            0,
        ));
        ok(world.prepare_module(review, 0)).commit();
        ok(world.request_launch(0));
        ok(world.step(Default::default()));
        assert_eq!(Sight::capture(&world, world.snapshot()), None);
        assert!(
            crate::help::Panel::flight_status(&world)
                .lines
                .iter()
                .any(|line| line == "FRONT LASER MOUNT EMPTY")
        );
    }

    #[test]
    fn front_sight_follows_the_fitted_mining_laser_and_selected_salvage_cutter() {
        use meridian_core::{ModuleAction, XyzzyAction};
        let mut miner = ok(World::new_commander(7412));
        let grant = ok(miner.review_xyzzy(XyzzyAction::Enter, 0));
        ok(miner.prepare_xyzzy(grant, 0)).commit();
        let purchase = ok(miner.review_module(ModuleAction::PurchaseMiningLaser, 0));
        ok(miner.prepare_module(purchase, 0)).commit();
        ok(miner.request_launch(0));
        ok(miner.step(Default::default()));
        assert_eq!(
            Sight::capture(&miner, miner.snapshot()),
            Some(Sight::Mining)
        );

        let mut salvage = ok(World::from_save_bytes(include_bytes!(
            "../../../../meridian-core/tests/fixtures/occupied-cut-held-v17.bin"
        )));
        let count = salvage.scanner_targets().len();
        for _ in 0..count {
            ok(salvage.cycle_scanner_target(salvage.snapshot().tick, [640, 240]));
            if salvage.selected_disabled_salvage_target().is_some() {
                assert!(!salvage.pilot_operates_selected_tool());
                assert_eq!(
                    Sight::capture(&salvage, salvage.snapshot()),
                    Sight::laser(
                        salvage
                            .selected_laser_snapshot()
                            .unwrap_or_else(|| panic!("fitted laser"))
                            .weapon
                    )
                );
                ok(salvage.toggle_scanner_assistance());
                assert!(salvage.pilot_operates_selected_tool());
                assert_eq!(
                    Sight::capture(&salvage, salvage.snapshot()),
                    Some(Sight::Cutter)
                );
                salvage.cancel_targeting();
                assert_ne!(
                    Sight::capture(&salvage, salvage.snapshot()),
                    Some(Sight::Cutter)
                );
                return;
            }
        }
        panic!("disabled contact missing from the retained scanner fixture");
    }
    #[test]
    fn ordinary_mounts_project_the_committed_ray_to_the_sight_centre() {
        use crate::mesh::{apply_flight_view, player_pose};
        for medusa in [false, true] {
            let mut world =
                World::new_commander(7411).unwrap_or_else(|error| panic!("commander: {error:?}"));
            if medusa {
                let review = world
                    .review_xyzzy(meridian_core::XyzzyAction::ActivateMedusa, 0)
                    .unwrap_or_else(|error| panic!("review: {error:?}"));
                world
                    .prepare_xyzzy(review, 0)
                    .unwrap_or_else(|error| panic!("issue: {error:?}"))
                    .commit();
            }
            assert_eq!(Sight::capture(&world, world.snapshot()), None);
            world
                .request_launch(world.snapshot().tick)
                .unwrap_or_else(|error| panic!("launch: {error:?}"));
            for _ in 0..600 {
                world
                    .step(Default::default())
                    .unwrap_or_else(|error| panic!("step: {error:?}"));
                if world.snapshot().mode == FlightMode::Flight {
                    break;
                }
            }
            for view in [
                FlightView::Front,
                FlightView::Left,
                FlightView::Right,
                FlightView::Rear,
            ] {
                world
                    .request_flight_view(view, world.snapshot().tick)
                    .unwrap_or_else(|error| panic!("view: {error:?}"));
                let snapshot = world.snapshot();
                let sight = Sight::capture(&world, snapshot);
                if view != world.selected_laser_view() {
                    assert_eq!(sight, None);
                    continue;
                }
                let fit = world
                    .selected_laser_snapshot()
                    .unwrap_or_else(|| panic!("missing fitted laser"));
                assert_eq!(sight, Sight::laser(fit.weapon));
                let shot = world
                    .request_fire_selected_laser(snapshot.tick)
                    .unwrap_or_else(|error| panic!("fire: {error:?}"));
                let eye = player_pose(&snapshot);
                assert_eq!(shot.origin, eye.position);
                let local = eye.orientation.inverse_coordinates(shot.direction);
                let mut ray = [Vertex {
                    position: [local.x as f32, local.y as f32, local.z as f32],
                    color: [0.; 3],
                }];
                apply_flight_view(view, &mut ray);
                let [x, y, z] = ray[0].position;
                assert!(x.abs() < 1e-5 && y.abs() < 1e-5 && z > 0.999);
                for _ in 0..fit.cooldown_ticks_per_shot {
                    world
                        .step(Default::default())
                        .unwrap_or_else(|error| panic!("cooldown: {error:?}"));
                }
            }
        }
    }
    #[test]
    fn all_fitted_tool_sights_have_distinct_geometry_centred_in_the_scene() {
        for [w, h] in [[640, 360], [1280, 720], [831, 617], [240, 180]] {
            let mut signatures = Vec::new();
            for sight in [
                Sight::Pulse,
                Sight::Beam,
                Sight::Military,
                Sight::Naval,
                Sight::Mining,
                Sight::Cutter,
                Sight::Scoop,
            ] {
                let mut vertices = Vec::new();
                sight.draw(w, h, &mut vertices);
                let points: Vec<_> = vertices.iter().map(|v| v.position).collect();
                assert!(!points.is_empty());
                assert!(!signatures.contains(&points));
                for (axis, centre) in [0., 1. - flight_height(w, h) / h as f32]
                    .into_iter()
                    .enumerate()
                {
                    let min = points.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
                    let max = points
                        .iter()
                        .map(|p| p[axis])
                        .fold(f32::NEG_INFINITY, f32::max);
                    assert!(((max + min) * 0.5 - centre).abs() < 1e-5);
                }
                signatures.push(points);
            }
        }
    }
}
