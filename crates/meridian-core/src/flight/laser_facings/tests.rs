//! Authored regressions. Existing G1/Dell owners execute after the whole-source join review.
use super::{LaserFacing, World};
use crate::docking::{Basis3, RigidPose, Vec3};
use crate::{FireError, FlightInput, ModuleAction, ModuleError, ModuleKind, PulseContact};

fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("unexpected public refusal: {error:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("expected real item or contact absent"))
}
fn apply(w: &mut World, action: ModuleAction) -> u64 {
    let tick = w.snapshot().tick;
    let review = ok(w.review_module(action, tick));
    let prepared = ok(w.prepare_module(review, tick));
    let item = prepared.item();
    let candidate = ok(World::from_save_bytes(prepared.bytes()));
    assert_eq!(prepared.commit(), item);
    assert_eq!(
        w.modular_equipment_snapshot(),
        candidate.modular_equipment_snapshot()
    );
    assert_eq!(w.credits_tenths(), candidate.credits_tenths());
    item
}
fn funded() -> World {
    let w = crate::flight::equipment::tests::earned_world(2101);
    assert_eq!(w.credits_tenths(), Some(85349));
    assert!(!some(w.xyzzy_snapshot()).active);
    w
}
fn paid(w: &mut World, facing: LaserFacing) -> u64 {
    apply(
        w,
        ModuleAction::PurchaseLaser {
            kind: ModuleKind::PulseLaser,
            facing,
        },
    )
}
fn cold(w: &World) -> World {
    let loaded = ok(World::from_save_bytes(&ok(w.save_bytes())));
    assert_eq!(
        w.modular_equipment_snapshot(),
        loaded.modular_equipment_snapshot()
    );
    assert_eq!(w.combat_snapshot(), loaded.combat_snapshot());
    for facing in LaserFacing::ALL {
        assert_eq!(w.laser_snapshot(facing), loaded.laser_snapshot(facing));
    }
    loaded
}
fn idle(w: &mut World, count: usize) {
    for _ in 0..count {
        ok(w.step(FlightInput::default()));
    }
}
fn launch(w: &mut World) {
    ok(w.request_launch(w.snapshot().tick));
    idle(w, 1);
    crate::flight::missiles::tests::stop(w);
}
fn turn(w: &mut World, x: f64, z: f64) {
    let p = w.snapshot().position.map(f64::from);
    crate::flight::practice::tests::aim_at(w, Vec3::new(p[0] + x, p[1], p[2] + z));
}
fn rear_starter() -> World {
    let mut w = ok(World::new_commander(2101));
    let item = some(w.front_laser_snapshot()).fitted_item_id;
    apply(&mut w, ModuleAction::Store { item });
    apply(
        &mut w,
        ModuleAction::InstallLaser {
            item,
            facing: LaserFacing::Rear,
            replacing: None,
        },
    );
    assert!(w.front_laser_snapshot().is_none());
    assert_eq!(
        some(w.laser_snapshot(LaserFacing::Rear)).fitted_item_id,
        item
    );
    w
}

#[test]
fn original_four_axis_contract_rotates_with_body_without_changing_emitter_or_proxy_geometry() {
    let body = RigidPose {
        position: Vec3::new(4.0, 7.0, 9.0),
        orientation: Basis3 {
            x: Vec3::new(0.0, 0.0, -1.0),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(1.0, 0.0, 0.0),
        },
    };
    for (facing, expected) in [
        (LaserFacing::Front, Vec3::new(1.0, 0.0, 0.0)),
        (LaserFacing::Left, Vec3::new(0.0, 0.0, 1.0)),
        (LaserFacing::Right, Vec3::new(0.0, 0.0, -1.0)),
        (LaserFacing::Rear, Vec3::new(-1.0, 0.0, 0.0)),
    ] {
        let ray = facing.ray(body);
        assert_eq!(ray.position, body.position);
        assert_eq!(ray.orientation.z, expected);
        assert!(ray.orientation.is_valid_rotation());
    }
    assert_eq!(LaserFacing::Front.ray(body), body);
}

#[test]
fn actual_paid_three_mounts_debit_one_wallet_and_store_move_refit_preserve_item_cold() {
    let mut w = funded();
    let left = paid(&mut w, LaserFacing::Left);
    assert_eq!(w.credits_tenths(), Some(70849));
    let right = paid(&mut w, LaserFacing::Right);
    assert_eq!(w.credits_tenths(), Some(56349));
    let rear = paid(&mut w, LaserFacing::Rear);
    assert_eq!(w.credits_tenths(), Some(41849));
    assert!(left != right && right != rear && left != rear);
    let bank = w.credits_tenths();
    apply(&mut w, ModuleAction::Store { item: left });
    assert!(w.laser_snapshot(LaserFacing::Left).is_none());
    apply(
        &mut w,
        ModuleAction::InstallLaser {
            item: rear,
            facing: LaserFacing::Left,
            replacing: None,
        },
    );
    assert!(w.laser_snapshot(LaserFacing::Rear).is_none());
    assert_eq!(
        some(w.laser_snapshot(LaserFacing::Left)).fitted_item_id,
        rear
    );
    apply(
        &mut w,
        ModuleAction::InstallLaser {
            item: left,
            facing: LaserFacing::Rear,
            replacing: None,
        },
    );
    assert_eq!(
        some(w.laser_snapshot(LaserFacing::Rear)).fitted_item_id,
        left
    );
    assert_eq!(w.credits_tenths(), bank);
    ok(w.request_flight_view(crate::FlightView::Rear, w.snapshot().tick));
    let loaded = cold(&w);
    assert_eq!(loaded.selected_laser_facing(), LaserFacing::Rear);
    assert_eq!(some(loaded.selected_laser_snapshot()).fitted_item_id, left);
}

#[test]
fn foreign_item_nonlaser_slot_and_stale_occupant_refuse_without_wallet_or_save_change() {
    let mut w = funded();
    let rear = paid(&mut w, LaserFacing::Rear);
    let foreign = some(ok(World::new_commander(9991)).front_laser_snapshot()).fitted_item_id;
    for (action, expected) in [
        (
            ModuleAction::InstallLaser {
                item: foreign,
                facing: LaserFacing::Left,
                replacing: None,
            },
            ModuleError::MissingItem,
        ),
        (
            ModuleAction::InstallLaser {
                item: 2106,
                facing: LaserFacing::Left,
                replacing: None,
            },
            ModuleError::NotForSale,
        ),
        (
            ModuleAction::InstallLaser {
                item: rear,
                facing: LaserFacing::Left,
                replacing: Some(rear),
            },
            ModuleError::StaleOccupant,
        ),
        (
            ModuleAction::PurchaseLaser {
                kind: ModuleKind::ExtraEnergyUnit,
                facing: LaserFacing::Right,
            },
            ModuleError::NotForSale,
        ),
    ] {
        let before = ok(w.save_bytes());
        assert_eq!(w.review_module(action, w.snapshot().tick), Err(expected));
        assert_eq!(ok(w.save_bytes()), before);
    }
}

#[test]
fn unfitted_nonfront_refuses_but_selected_clear_miss_pays_shared_pool_and_blocks_front_cadence() {
    let mut w = funded();
    paid(&mut w, LaserFacing::Left);
    launch(&mut w);
    let before = ok(w.save_bytes());
    assert_eq!(
        w.request_fire_laser(LaserFacing::Rear, w.snapshot().tick),
        Err(FireError::NoFittedLaser(LaserFacing::Rear))
    );
    assert_eq!(ok(w.save_bytes()), before);
    let opening = some(w.flight_resources_snapshot());
    let shot = ok(w.request_fire_laser(LaserFacing::Left, w.snapshot().tick));
    assert_eq!(shot.contact, PulseContact::Clear);
    assert_eq!(shot.direction, Vec3::new(-1.0, 0.0, 0.0));
    assert_eq!(
        some(w.flight_resources_snapshot()).energy_hundredths,
        opening.energy_hundredths - 100
    );
    assert_eq!(
        some(w.flight_resources_snapshot()).heat_fifths,
        opening.heat_fifths + 50
    );
    let paid_save = ok(w.save_bytes());
    assert_eq!(
        w.request_fire_front_laser(w.snapshot().tick),
        Err(FireError::CoolingDown)
    );
    assert_eq!(ok(w.save_bytes()), paid_save);
    idle(&mut w, 18);
    ok(w.request_fire_front_laser(w.snapshot().tick));
    cold(&w);
}

#[test]
fn all_three_nonfront_commands_hit_original_station_wall_and_cold_replay_keeps_body_pose() {
    let mut w = funded();
    for facing in [LaserFacing::Left, LaserFacing::Right, LaserFacing::Rear] {
        paid(&mut w, facing);
    }
    launch(&mut w);
    // Center of original SH028 native trim quad 482/483, not the braking waypoint.
    // At speed1 the old threshold pilot coasted ~0.8445m while braking, beyond x70.12.
    crate::flight::missiles::tests::fly_axis(&mut w, 0, (68.12 + 70.12) / 2.0, 0.25);
    for (facing, x, z) in [
        (LaserFacing::Left, -10000.0, 0.0),
        (LaserFacing::Right, 10000.0, 0.0),
        (LaserFacing::Rear, 0.0, 10000.0),
    ] {
        turn(&mut w, x, z);
        let body = w.snapshot();
        let ray = facing.ray(some(crate::flight::flight_pose(
            body.position,
            body.orientation,
        )));
        // Independent source quad: x68.12..70.12, y+-48.384, z136.32.
        let distance = (136.32 - ray.position.z) / ray.orientation.z.z;
        let wall_x = ray.position.x + distance * ray.orientation.z.x;
        let wall_y = ray.position.y + distance * ray.orientation.z.y;
        assert!(distance > 0.0 && distance < crate::PULSE_RANGE);
        assert!(
            (68.12..70.12).contains(&wall_x) && wall_y.abs() < 48.384,
            "real ray must cross the independently pinned native front trim: {ray:?}"
        );
        let shot = ok(w.request_fire_laser(facing, body.tick));
        assert!(matches!(shot.contact, PulseContact::StationBlocked { .. }));
        assert!(shot.direction.z < -0.999999);
        assert!(
            (shot.end.z - 136.32).abs() < 1e-6,
            "independent original SH028 wall plane; ray={ray:?} shot={shot:?}"
        );
        let hit = some(some(w.combat_snapshot()).last_ordinary_impact);
        assert_eq!(hit.item, some(w.laser_snapshot(facing)).fitted_item_id);
        assert_eq!(hit.point, shot.end);
        assert_eq!(w.snapshot().orientation, body.orientation);
        cold(&w);
        idle(&mut w, 18);
    }
}

#[test]
fn actual_rear_starter_hits_real_practice_target_and_retains_selected_item_emission_cold() {
    let mut w = rear_starter();
    crate::flight::practice::tests::consent(&mut w);
    launch(&mut w);
    let beacons = some(some(w.practice_snapshot()).beacons);
    turn(&mut w, 0.0, -10000.0);
    ok(w.request_flight_view(crate::FlightView::Rear, w.snapshot().tick));
    let shot = ok(w.request_fire_selected_laser(w.snapshot().tick));
    ok(w.request_flight_view(crate::FlightView::Front, w.snapshot().tick));
    assert!(
        matches!(shot.contact, PulseContact::PracticeBeacon { id, first_hit: true, .. } if id == beacons[1].id)
    );
    assert_eq!(some(w.practice_snapshot()).first_hits, 1);
    cold(&w);
}

#[test]
fn rear_station_occlusion_does_not_award_the_real_practice_target_behind_the_wall() {
    let mut w = rear_starter();
    crate::flight::practice::tests::consent(&mut w);
    launch(&mut w);
    let target = some(some(w.practice_snapshot()).beacons)[2].pose.position;
    crate::flight::missiles::tests::fly_axis(&mut w, 0, 1000.0, 4.0);
    crate::flight::missiles::tests::fly_axis(&mut w, 2, -1000.0, 4.0);
    // Source faces384/387 share y0 edge (105.632,0,-112)..(96.8,0,-76.16).
    // Aim through its midpoint; the old speed4 brake overshot x90 by ~15m into a real opening.
    let wall = Vec3::new((105.632 + 96.8) / 2.0, 0.0, (-112.0 - 76.16) / 2.0);
    let z = f64::from(w.snapshot().position[2]);
    let x = wall.x + (target.x - wall.x) * (z - wall.z) / (target.z - wall.z);
    crate::flight::missiles::tests::fly_axis(&mut w, 0, x, 0.25);
    let p = w.snapshot().position.map(f64::from);
    crate::flight::practice::tests::aim_at(
        &mut w,
        Vec3::new(2.0 * p[0] - target.x, p[1], 2.0 * p[2] - target.z),
    );
    let snapshot = w.snapshot();
    let ray = LaserFacing::Rear.ray(some(crate::flight::flight_pose(
        snapshot.position,
        snapshot.orientation,
    )));
    // Independent plane through that source edge: 35.84(x-105.632)+8.832(z+112)=0.
    let d = ray.orientation.z;
    let wall_distance = -(35.84 * (ray.position.x - 105.632) + 8.832 * (ray.position.z + 112.0))
        / (35.84 * d.x + 8.832 * d.z);
    let wall_z = ray.position.z + wall_distance * d.z;
    assert!(
        wall_distance > 0.0 && (-112.0..-76.16).contains(&wall_z),
        "source back-wall crossing: {ray:?}"
    );
    let target_distance = (target.z - ray.position.z) / d.z;
    assert!(wall_distance < target_distance && target_distance < crate::PULSE_RANGE);
    assert!((ray.position.x + target_distance * d.x - target.x).abs() < crate::PRACTICE_HALF_SIDE);
    assert!((ray.position.y + target_distance * d.y - target.y).abs() < crate::PRACTICE_HALF_SIDE);
    let shot = ok(w.request_fire_laser(LaserFacing::Rear, snapshot.tick));
    assert!(
        matches!(shot.contact, PulseContact::StationBlocked { .. }),
        "source back wall must occlude actual beacon: {shot:?}"
    );
    assert_eq!(some(w.practice_snapshot()).first_hits, 0);
    cold(&w);
}

#[test]
fn right_mount_hits_the_actual_native_attacker_with_real_shield_damage_and_cold_custody() {
    let mut w = ok(World::new_commander(12001));
    let item = some(w.front_laser_snapshot()).fitted_item_id;
    apply(&mut w, ModuleAction::Store { item });
    apply(
        &mut w,
        ModuleAction::InstallLaser {
            item,
            facing: LaserFacing::Right,
            replacing: None,
        },
    );
    ok(w.request_station_service(crate::station_services::Command::BuyFood, w.snapshot().tick));
    launch(&mut w);
    crate::flight::missiles::tests::fly_axis(&mut w, 0, -4500.0, 34.3);
    let mut attacker = None;
    for _ in 0..24000 {
        idle(&mut w, 1);
        if let Some(hit) = some(w.traffic_snapshot()).last_player_hit
            && hit.world_tick == w.snapshot().tick
            && hit.visit == some(w.snapshot().navigation).visit
        {
            attacker = Some(hit.attacker);
            break;
        }
    }
    let attacker = some(attacker);
    let balance = w.credits_tenths();
    let food = w.commodity_quantity(crate::station_services::CommodityId::Food);
    for _ in 0..1200 {
        let target = some(
            some(w.traffic_snapshot())
                .actors
                .into_iter()
                .find(|a| a.id == attacker),
        );
        let p = w.snapshot();
        let body = some(crate::flight::flight_pose(p.position, p.orientation));
        let surface = some(ok(
            some(w.station_data.as_ref()).player_planar_aimpoint(target.pose, body.position)
        ));
        let desired = (surface.x - body.position.x).atan2(surface.z - body.position.z)
            - core::f64::consts::FRAC_PI_2;
        let current = body.orientation.z.x.atan2(body.orientation.z.z);
        let error = (desired - current).sin().atan2((desired - current).cos());
        if w.traffic_beam_contact(LaserFacing::Right.ray(body), crate::PULSE_RANGE)
            .is_some_and(|hit| hit.id == attacker)
        {
            let shot = ok(w.request_fire_laser(LaserFacing::Right, p.tick));
            assert!(matches!(shot.contact, PulseContact::Npc { id, .. } if id == attacker));
            assert_eq!(shot.fitted_item_id, item);
            let after = some(
                some(w.traffic_snapshot())
                    .actors
                    .into_iter()
                    .find(|a| a.id == attacker),
            );
            assert!(
                after.front_shield_hundredths + after.rear_shield_hundredths
                    < target.front_shield_hundredths + target.rear_shield_hundredths
            );
            assert!(after.energy_hundredths > 0);
            assert_eq!(w.credits_tenths(), balance);
            assert_eq!(
                w.commodity_quantity(crate::station_services::CommodityId::Food),
                food
            );
            cold(&w);
            return;
        }
        ok(w.step(FlightInput {
            yaw: (error / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32,
            ..FlightInput::default()
        }));
    }
    panic!("right-facing real pilot did not reach the actual native attacker's surface");
}

#[test]
fn actual_rear_practice_hit_retains_item_facing_and_history_through_retry_cold_continue() {
    let mut w = rear_starter();
    crate::flight::practice::tests::consent(&mut w);
    launch(&mut w);
    let beacons = some(some(w.practice_snapshot()).beacons);
    turn(&mut w, 0.0, -10000.0);
    ok(w.request_flight_view(crate::FlightView::Rear, w.snapshot().tick));
    let shot = ok(w.request_fire_selected_laser(w.snapshot().tick));
    assert!(
        matches!(shot.contact, PulseContact::PracticeBeacon { id, first_hit: true, .. } if id == beacons[1].id)
    );
    assert_eq!(
        shot.fitted_item_id,
        some(w.laser_snapshot(LaserFacing::Rear)).fitted_item_id
    );
    crate::flight::missiles::tests::dock(&mut w);
    let review = ok(w.review_practice_closure(w.snapshot().tick));
    ok(w.prepare_practice_closure(review, w.snapshot().tick)).commit();
    let first = some(w.practice_ledger_snapshot());
    assert_eq!(first.attempts.len(), 1);
    assert_eq!(first.attempts[0].first_hits, 1);
    assert_eq!(
        first.attempts[0].outcome,
        Some(crate::PracticeOutcome::Abandoned)
    );
    crate::flight::practice::tests::consent(&mut w);
    let expected = some(w.practice_ledger_snapshot());
    assert_eq!(expected.attempts.len(), 2);
    assert_eq!(expected.attempts[0], first.attempts[0]);
    let loaded = cold(&w);
    assert_eq!(some(loaded.practice_ledger_snapshot()), expected);
    assert_eq!(loaded.practice_snapshot(), w.practice_snapshot());
    assert_eq!(loaded.flight_view(), crate::FlightView::Rear);
}
