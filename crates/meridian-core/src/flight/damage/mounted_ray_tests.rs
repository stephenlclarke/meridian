//! Genuine paid two-mount positive commands followed by coordinated corruption only.
use super::*;
use crate::docking::Vec3;
use crate::{FlightInput, LaserFacing, ModuleAction, ModuleKind, PulseContact};

fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("{error:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("actual receipt/item absent"))
}
fn idle(w: &mut World, n: usize) {
    for _ in 0..n {
        ok(w.step(FlightInput::default()));
    }
}
fn paid_pair() -> (World, u64, u64) {
    // Existing genuine protected G02/finite-market pilot earns every credit, closes
    // its real freight and buys the EMP base. It injects no actor/ammo/target state.
    let mut w = crate::flight::emp::tests::paid();
    assert_eq!(w.credits_tenths(), Some(62649));
    assert!(!some(w.xyzzy_snapshot()).active);
    let mut items = [0; 2];
    for (index, facing) in [LaserFacing::Left, LaserFacing::Right]
        .into_iter()
        .enumerate()
    {
        let tick = w.snapshot().tick;
        let review = ok(w.review_module(
            ModuleAction::PurchaseLaser {
                kind: ModuleKind::PulseLaser,
                facing,
            },
            tick,
        ));
        let prepared = ok(w.prepare_module(review, tick));
        let cold = ok(World::from_save_bytes(prepared.bytes()));
        items[index] = prepared.commit();
        assert!(
            some(w.modular_equipment_snapshot())
                .items
                .iter()
                .any(|item| item.id == items[index]
                    && item.kind == ModuleKind::PulseLaser
                    && matches!(item.provenance, crate::ModuleProvenance::Paid { .. }))
        );
        assert_eq!(
            cold.modular_equipment_snapshot(),
            w.modular_equipment_snapshot()
        );
    }
    assert_eq!(w.credits_tenths(), Some(33649));
    assert_ne!(items[0], items[1]);
    assert_eq!(
        some(w.laser_snapshot(LaserFacing::Left)).fitted_item_id,
        items[0]
    );
    assert_eq!(
        some(w.laser_snapshot(LaserFacing::Right)).fitted_item_id,
        items[1]
    );
    (w, items[0], items[1])
}
fn positive(w: &World) -> World {
    let bytes = ok(w.save_bytes());
    let loaded = ok(World::from_save_bytes(&bytes));
    assert_eq!(loaded.traffic_snapshot(), w.traffic_snapshot());
    assert_eq!(loaded.missile_snapshot(), w.missile_snapshot());
    assert_eq!(
        loaded.flight_resources_snapshot(),
        w.flight_resources_snapshot()
    );
    assert_eq!(
        loaded.modular_equipment_snapshot(),
        w.modular_equipment_snapshot()
    );
    loaded
}
fn relabel_spend(w: &mut World, clock: u64) {
    let event = some(
        Arc::make_mut(&mut w.damage.events)
            .iter_mut()
            .find(|event| {
                event.clock == clock
                    && matches!(
                        event.kind,
                        Kind::FacedLaser(
                            crate::OrdinaryWeapon::FrontPulseLaser,
                            LaserFacing::Right
                        )
                    )
            }),
    );
    event.kind = Kind::FacedLaser(crate::OrdinaryWeapon::FrontPulseLaser, LaserFacing::Left);
}
fn refusal(altered: &World, live: &mut World) {
    let before = ok(live.save_bytes());
    assert!(altered.save_bytes().is_err());
    let bytes = ok(altered.unchecked_test_save());
    assert!(World::from_save_bytes(&bytes).is_err());
    assert!(live.restore_bytes(&bytes).is_err());
    assert_eq!(ok(live.save_bytes()), before);
}

#[test]
fn genuine_paid_right_npc_hit_rejects_coordinated_item_spend_and_receipt_facing_relabel_cold() {
    let (w, left, right) = paid_pair();
    let (mut w, target) = crate::flight::emp::tests::defender(w);
    let balance = w.credits_tenths();
    let food = w.commodity_quantity(crate::station_services::CommodityId::Food);
    let mut emitted = None;
    for _ in 0..1200 {
        let actor = some(
            some(w.traffic_snapshot())
                .actors
                .into_iter()
                .find(|actor| actor.id == target),
        );
        let snapshot = w.snapshot();
        let body = some(crate::flight::flight_pose(
            snapshot.position,
            snapshot.orientation,
        ));
        let point = some(ok(
            some(w.station_data.as_ref()).player_planar_aimpoint(actor.pose, body.position)
        ));
        let desired = (point.x - body.position.x).atan2(point.z - body.position.z)
            - core::f64::consts::FRAC_PI_2;
        let current = body.orientation.z.x.atan2(body.orientation.z.z);
        let error = (desired - current).sin().atan2((desired - current).cos());
        if w.traffic_beam_contact(LaserFacing::Right.ray(body), crate::PULSE_RANGE)
            .is_some_and(|hit| hit.id == target)
        {
            let before = actor.front_shield_hundredths + actor.rear_shield_hundredths;
            let shot = ok(w.request_fire_laser(LaserFacing::Right, snapshot.tick));
            assert_eq!(shot.fitted_item_id, right);
            assert!(matches!(shot.contact,PulseContact::Npc {id,..} if id==target));
            let after = some(
                some(w.traffic_snapshot())
                    .actors
                    .into_iter()
                    .find(|actor| actor.id == target),
            );
            assert!(after.front_shield_hundredths + after.rear_shield_hundredths < before);
            let receipt=some(w.traffic.beam_receipts().iter().find(|beam|matches!(beam.source,crate::traffic::BeamSource::Player {item,..} if item==right)));
            let crate::traffic::BeamSource::Player { emission, .. } = receipt.source else {
                panic!("player receipt")
            };
            let mounted = some(receipt.mounted_emission);
            assert_eq!(mounted.body_pose, body);
            assert_eq!(mounted.facing, LaserFacing::Right);
            emitted = Some((emission, mounted, shot));
            break;
        }
        ok(w.step(FlightInput {
            yaw: (error / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32,
            ..FlightInput::default()
        }));
    }
    let (emission, mounted, shot) = some(emitted);
    assert_eq!(w.credits_tenths(), balance);
    assert_eq!(
        w.commodity_quantity(crate::station_services::CommodityId::Food),
        food
    );
    idle(&mut w, 1); // Historical receipt: a same-tick current-body guard alone cannot pass this case.
    let mut live = positive(&w);
    for change_receipt_facing in [false, true] {
        let mut altered = positive(&w);
        relabel_spend(&mut altered, mounted.physical_tick);
        altered.traffic.corrupt_beams(|beams| {
            let beam=some(beams.iter_mut().find(|beam|matches!(beam.source,crate::traffic::BeamSource::Player {emission:id,..} if id==emission)));
            let crate::traffic::BeamSource::Player {item,..}=&mut beam.source else {panic!("player")};*item=left;
            if change_receipt_facing {some(beam.mounted_emission.as_mut()).facing=LaserFacing::Left;}
            assert_eq!((beam.origin,beam.direction,beam.point),(shot.origin,shot.direction,shot.end));
            assert_eq!(some(beam.mounted_emission).body_pose,mounted.body_pose);
        });
        refusal(&altered, &mut live);
    }
    let mut altered = positive(&w);
    altered.traffic.corrupt_beams(|beams| {
        let beam=some(beams.iter_mut().find(|beam|matches!(beam.source,crate::traffic::BeamSource::Player {emission:id,..} if id==emission)));
        some(beam.mounted_emission.as_mut()).physical_tick+=1;
    });
    refusal(&altered, &mut live);
}

#[test]
fn genuine_paid_right_interception_rejects_coordinated_item_spend_and_receipt_facing_relabel_cold()
{
    let (mut w, left, right) = paid_pair();
    assert_eq!(ok(w.request_refuel(w.snapshot().tick)), 204);
    crate::flight::equipment::tests::journey(&mut w, crate::navigation::SystemId::AVENOR);
    assert_eq!(some(w.snapshot().navigation).fuel_units, 96);
    assert_eq!(w.credits_tenths(), Some(33445));
    crate::flight::practice::tests::consent(&mut w);
    crate::flight::practice::tests::launch(&mut w);
    crate::flight::missiles::tests::stop(&mut w);
    // Far enough that a real owned projectile remains airborne during a public 90-degree turn.
    crate::flight::missiles::tests::fly_axis(&mut w, 2, 5500.0, 34.3);
    let beacon = some(some(w.practice_snapshot()).beacons)[1];
    crate::flight::practice::tests::aim_at(&mut w, beacon.pose.position);
    let origin = crate::flight::missiles::tests::fire(&mut w, beacon.id);
    let p = w.snapshot().position.map(f64::from);
    crate::flight::practice::tests::aim_at(&mut w, Vec3::new(p[0] + 10000.0, p[1], p[2]));
    let mut emitted = None;
    for _ in 0..120 {
        let projectile = some(
            some(w.missile_snapshot())
                .projectiles
                .into_iter()
                .flatten()
                .find(|projectile| projectile.origin.projectile == origin.projectile),
        );
        assert!(
            projectile.ended.is_none(),
            "real owned projectile must still be airborne"
        );
        let snapshot = w.snapshot();
        let body = some(crate::flight::flight_pose(
            snapshot.position,
            snapshot.orientation,
        ));
        if w.missiles
            .nearest_beam_body(
                some(snapshot.navigation).visit,
                LaserFacing::Right.ray(body),
                crate::PULSE_RANGE,
            )
            .is_some_and(|(id, _)| id == origin.projectile)
        {
            let shot = ok(w.request_fire_laser(LaserFacing::Right, snapshot.tick));
            assert!(
                matches!(shot.contact,PulseContact::Projectile {id,..} if id==origin.projectile)
            );
            assert_eq!(shot.fitted_item_id, right);
            let (emission, _, _, mounted) = some(w.missiles.laser_interception_emissions().next());
            assert_eq!(mounted.body_pose, body);
            emitted = Some((emission, mounted, shot));
            break;
        }
        let desired = (projectile.position.x - body.position.x)
            .atan2(projectile.position.z - body.position.z)
            - core::f64::consts::FRAC_PI_2;
        let current = body.orientation.z.x.atan2(body.orientation.z.z);
        let error = (desired - current).sin().atan2((desired - current).cos());
        ok(w.step(FlightInput {
            yaw: (error / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32,
            ..FlightInput::default()
        }));
    }
    let (emission, mounted, shot) = some(emitted);
    assert_eq!(
        some(w.missile_snapshot()).rounds.iter().flatten().count(),
        2
    );
    assert_eq!(some(w.practice_snapshot()).first_hits, 0);
    assert_eq!(w.credits_tenths(), Some(33445));
    idle(&mut w, 1);
    let mut live = positive(&w);
    for change_receipt_facing in [false, true] {
        let mut altered = positive(&w);
        relabel_spend(&mut altered, mounted.physical_tick);
        assert!(
            altered
                .missiles
                .corrupt_intercepting_laser(emission, |retained, record| {
                    retained.fitted_item_id = left;
                    if change_receipt_facing {
                        record.facing = LaserFacing::Left;
                    }
                    assert_eq!(
                        (retained.origin, retained.direction, retained.end),
                        (shot.origin, shot.direction, shot.end)
                    );
                    assert_eq!(record.body_pose, mounted.body_pose);
                })
        );
        refusal(&altered, &mut live);
    }
    let mut altered = positive(&w);
    assert!(
        altered
            .missiles
            .corrupt_intercepting_laser(emission, |_, record| record.physical_tick += 1)
    );
    refusal(&altered, &mut live);
}
