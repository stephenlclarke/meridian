//! Real native actions create the wreck source. Direct edits appear only in a corruption test.
use super::{
    assert_native_endpoint, cold, finish_npc, fire_npc, ready, ready_foreign, refuses, trader,
    trader_pilot,
};
use crate::flight::equipment::tests::{earned_from, journey};
use crate::flight::missiles::tests::{fly_axis, stop};
use crate::flight::practice::tests::{consent, launch, step};
use crate::navigation::{BASIC_SPOOL_TICKS, SystemId};
use crate::station_services::MarketSite;
use crate::traffic::{
    BeamSource, NpcId, NpcPhase, Population, TrafficBeam, WreckDamageSource, WreckManifest,
    WreckManifestError,
};
use crate::{
    AmmunitionAction, FireError, MissileContact, MissileError, ModuleAction, ModuleKind,
    OrdinaryWeapon, PulseContact, PulseShot, World,
};

fn paid_wreck() -> (World, WreckManifest) {
    let mut w = ready(4);
    let source = trader(&w);
    let (commodity, quantity) = source.cargo.unwrap();
    assert_eq!(
        w.wreck_manifest(source.id),
        Err(WreckManifestError::SourceAlive)
    );
    let opening_cargo = w.commodity_quantity(commodity);
    let opening_cash = w.credits_tenths();
    let opening_market = w
        .station_markets
        .as_ref()
        .unwrap()
        .commodity_quote(MarketSite::Avenor, commodity);
    for _ in 0..7 {
        let origin = fire_npc(&mut w);
        let projectile = finish_npc(&mut w, origin.projectile);
        assert_eq!(projectile.contact, Some(MissileContact::Npc(source.id)));
        assert_native_endpoint(&w, projectile);
        if trader(&w).phase == NpcPhase::Destroyed {
            let wreck = w.wreck_manifest(source.id).unwrap();
            assert_eq!(
                (wreck.source, wreck.hull_id, wreck.commodity, wreck.quantity),
                (source.id, "SH025", commodity, quantity)
            );
            assert_eq!(wreck.cause, WreckDamageSource::Missile(origin.projectile));
            assert_eq!(wreck.impact_point, projectile.position);
            assert_eq!(wreck.pose, trader(&w).pose);
            assert_eq!(wreck.destroyed_at_tick, projectile.last_tick);
            assert!(wreck.paid_at_tick < wreck.destroyed_at_tick);
            assert!(wreck.destroyed_at_local_tick > 0);
            assert_eq!(wreck.planned_lot_quantities().collect::<Vec<_>>(), [5, 4]);
            assert_eq!(trader(&w).cargo, None);
            assert_eq!(w.commodity_quantity(commodity), opening_cargo);
            assert_eq!(w.credits_tenths(), opening_cash);
            assert_eq!(
                w.station_markets
                    .as_ref()
                    .unwrap()
                    .commodity_quote(MarketSite::Avenor, commodity),
                opening_market
            );
            return (w, wreck);
        }
        assert_eq!(
            w.wreck_manifest(source.id),
            Err(WreckManifestError::SourceAlive)
        );
        while w.missile_snapshot().unwrap().cooldown_ticks > 0 {
            step(&mut w, 1);
        }
        w = cold(&w);
    }
    panic!("real paid native hits did not destroy trader");
}

#[test]
fn actual_fatal_missile_settles_one_paid_manifest_and_cold_readout_never_issues_loot() {
    let (mut w, wreck) = paid_wreck();
    let before = w.save_bytes().unwrap();
    assert_eq!(w.wreck_manifest(wreck.source), Ok(wreck));
    assert_eq!(w.wreck_manifest(wreck.source), Ok(wreck));
    assert_eq!(w.save_bytes().unwrap(), before);
    let tick = w.snapshot().tick;
    assert_eq!(
        w.request_npc_missile_lock(Some(wreck.source), tick),
        Err(MissileError::NoLiveLock)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let mut loaded = cold(&w);
    assert_eq!(loaded.wreck_manifest(wreck.source), Ok(wreck));
    // Current-only saves retain the same earned missile and trader custody history.
    // Older format refusal is covered by the two retained literal fixture tests.
    assert!(w.traffic.beam_receipts().is_empty());
    let current_bytes = w.save_bytes().unwrap();
    assert_eq!(
        u16::from_le_bytes([current_bytes[8], current_bytes[9]]),
        crate::save::SAVE_VERSION
    );
    assert_eq!(crate::save::SAVE_VERSION, 17);
    let current = World::from_save_bytes(&current_bytes).unwrap();
    assert_eq!(current.wreck_manifest(wreck.source), Ok(wreck));
    assert_eq!(current.traffic_snapshot(), w.traffic_snapshot());
    assert_eq!(current.credits_tenths(), w.credits_tenths());
    assert!(current.mining_belt_snapshot().is_none());
    assert_eq!(cold(&current).wreck_manifest(wreck.source), Ok(wreck));
    // A real jump removes the local source from the current World query. There was never a pod.
    let tick = loaded.snapshot().tick;
    loaded.request_jump(SystemId::RENAS, tick).unwrap();
    for _ in 0..BASIC_SPOOL_TICKS {
        step(&mut loaded, 1);
    }
    assert_eq!(
        loaded.wreck_manifest(wreck.source),
        Err(WreckManifestError::OutsideSystem)
    );
    assert_eq!(
        loaded.commodity_quantity(wreck.commodity),
        w.commodity_quantity(wreck.commodity)
    );
    let loaded = cold(&loaded);
    assert_eq!(
        loaded.wreck_manifest(wreck.source),
        Err(WreckManifestError::OutsideSystem)
    );
}

#[test]
fn actual_live_sold_foreign_and_not_yet_admitted_traders_have_no_wreck_manifest() {
    let w = ready(0);
    let local = trader(&w).id;
    let foreign = trader(&ready_foreign()).id;
    let fresh = World::new_commander(12005).unwrap();
    assert_eq!(
        World::new().wreck_manifest(local),
        Err(WreckManifestError::NoCommander)
    );
    assert_eq!(
        fresh.wreck_manifest(local),
        Err(WreckManifestError::UnknownActor)
    );
    let mut w = w;
    let before = w.save_bytes().unwrap();
    assert_eq!(
        w.wreck_manifest(local),
        Err(WreckManifestError::SourceAlive)
    );
    assert_eq!(
        w.wreck_manifest(foreign),
        Err(WreckManifestError::ForeignActor)
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    for _ in 0..24000 {
        if trader(&w).phase == NpcPhase::Complete {
            let before = w.save_bytes().unwrap();
            assert_eq!(
                w.wreck_manifest(local),
                Err(WreckManifestError::ManifestSold)
            );
            assert_eq!(w.save_bytes().unwrap(), before);
            assert_eq!(
                cold(&w).wreck_manifest(local),
                Err(WreckManifestError::ManifestSold)
            );
            return;
        }
        step(&mut w, 1);
    }
    panic!("actual trader did not complete native entry and sale");
}

#[test]
fn a_changed_terminal_native_contact_cannot_relocate_the_paid_manifest_or_replace_a_valid_world() {
    let (w, wreck) = paid_wreck();
    refuses(&w, |changed| {
        changed.traffic.corrupt_missile_hits(|hits| {
            let terminal = hits
                .iter_mut()
                .rfind(|hit| hit.victim == wreck.source)
                .unwrap();
            terminal.victim_pose.position.x += 1.0;
        });
        assert_eq!(
            changed.wreck_manifest(wreck.source),
            Err(WreckManifestError::MissingTerminalContact)
        );
    });
    assert_eq!(w.wreck_manifest(wreck.source), Ok(wreck));
}

fn paid_beam_world() -> (World, u64) {
    let mut world = World::new_commander(12005).unwrap();
    // Use the same protected public funding and actual closure as the existing Beam test.
    consent(&mut world);
    let mut world = earned_from(world);
    if world.snapshot().navigation.unwrap().fuel_units < 68 {
        world.request_refuel(world.snapshot().tick).unwrap();
    }
    journey(&mut world, SystemId::AVENOR);
    let tick = world.snapshot().tick;
    let closure = world.review_practice_closure(tick).unwrap();
    world
        .prepare_practice_closure(closure, tick)
        .unwrap()
        .commit();
    let cash = world.credits_tenths().unwrap();
    let review = world
        .review_module(
            ModuleAction::Purchase {
                kind: ModuleKind::BeamLaser,
            },
            tick,
        )
        .unwrap();
    let item = world.prepare_module(review, tick).unwrap().commit();
    assert_eq!(world.credits_tenths(), Some(cash - 47000));
    let fit = world.front_laser_snapshot().unwrap();
    assert_eq!(
        (fit.fitted_item_id, fit.weapon, fit.cooldown_ticks_per_shot),
        (item, OrdinaryWeapon::FrontBeamLaser, 6)
    );
    for _ in 0..3 {
        let review = world
            .review_ammunition(AmmunitionAction::Purchase, tick)
            .unwrap();
        world.prepare_ammunition(review, tick).unwrap().commit();
    }
    launch(&mut world);
    stop(&mut world);
    fly_axis(&mut world, 0, -600.0, 10.0);
    for _ in 0..1000 {
        if world
            .traffic_snapshot()
            .is_some_and(|s| s.actors.iter().any(|a| a.population == Population::Trader))
        {
            return (world, item);
        }
        step(&mut world, 1);
    }
    panic!("actual trader did not enter the system");
}

fn step_toward_native_surface(world: &mut World) {
    let target = trader(world);
    let flight = world.snapshot();
    let player = crate::flight::flight_pose(flight.position, flight.orientation).unwrap();
    let mut input = trader_pilot(world);
    if let Some(point) = world
        .station_data
        .as_ref()
        .unwrap()
        .player_planar_aimpoint(target.pose, player.position)
        .unwrap()
    {
        let desired = (point.x - player.position.x).atan2(point.z - player.position.z);
        let yaw = player.orientation.z.x.atan2(player.orientation.z.z);
        let delta = desired - yaw;
        let error = delta.sin().atan2(delta.cos());
        input.yaw = (error / (0.65 / 60.0)).clamp(-1.0, 1.0) as f32;
    }
    world.step(input).unwrap();
}

fn weaken_trader_with_missiles(world: &mut World, source: NpcId) {
    let mut missile_hits = 0;
    for _ in 0..6 {
        // Leave at least three 1500-point Beam contacts even if the missile hits bare hull.
        if trader(world).energy_hundredths <= 20_500 {
            break;
        }
        let origin = fire_npc(world);
        if missile_hits == 0 {
            let lesson = world.practice.unwrap();
            assert!(lesson.reception_is_terminal());
            assert!(!lesson.missile_owner_matches(origin.owner));
            assert_eq!(lesson.pending_missiles, 0);

            // Apply the same first homing step as finish_npc and save while the missile is live.
            world.step(trader_pilot(world)).unwrap();
            assert!(
                world
                    .missile_projectiles()
                    .any(|p| { p.origin.projectile == origin.projectile && p.ended.is_none() })
            );
            assert_eq!(world.practice.unwrap().pending_missiles, 0);
            cold(world);
        }
        let impact = finish_npc(world, origin.projectile);
        assert_eq!(impact.contact, Some(MissileContact::Npc(source)));
        assert_native_endpoint(world, impact);
        missile_hits += 1;
        assert_eq!(
            world.wreck_manifest(source),
            Err(WreckManifestError::SourceAlive)
        );
        while world.missile_snapshot().unwrap().cooldown_ticks > 0 {
            step_toward_native_surface(world);
        }
    }
    assert!(missile_hits > 0);
}

fn finish_trader_with_beam(
    world: &mut World,
    source: NpcId,
    item: u64,
) -> (PulseShot, TrafficBeam) {
    let mut beam_hits = 0;
    for _ in 0..12000 {
        let flight = world.snapshot();
        let pose = crate::flight::flight_pose(flight.position, flight.orientation).unwrap();
        let ready = world.flight_resources_snapshot().unwrap().cooldown_ticks == 0;
        if ready
            && matches!(world.pulse_scene(pose), Ok((_, PulseContact::Npc { id, .. }, _)) if id == source)
        {
            match world.request_fire_front_laser(flight.tick) {
                Ok(shot) => {
                    beam_hits += 1;
                    assert_eq!(shot.fitted_item_id, item);
                    let beam = world.traffic_snapshot().unwrap().last_beam.unwrap();
                    assert_eq!(beam.weapon, OrdinaryWeapon::FrontBeamLaser);
                    assert_eq!(world.flight_resources_snapshot().unwrap().cooldown_ticks, 6);
                    if trader(world).phase == NpcPhase::Destroyed {
                        assert!(
                            beam_hits > 1,
                            "expected a rapid Beam history before the fatal shot"
                        );
                        return (shot, beam);
                    }
                    assert_eq!(
                        world.wreck_manifest(source),
                        Err(WreckManifestError::SourceAlive)
                    );
                }
                Err(FireError::Overheated) => {}
                Err(error) => panic!("real fitted Beam refused: {error:?}"),
            }
        }
        step_toward_native_surface(world);
    }
    panic!("actual rapid Beam never settled the trader after {beam_hits} Beam contacts");
}

#[test]
fn mixed_missile_and_rapid_beam_history_keeps_the_actual_fatal_paid_laser_in_custody() {
    let (mut world, item) = paid_beam_world();
    let source = trader(&world);
    let native_manifest = source.cargo.unwrap();
    let cargo = world.commodity_quantity(native_manifest.0);
    let cash = world.credits_tenths();
    weaken_trader_with_missiles(&mut world, source.id);
    let (shot, beam) = finish_trader_with_beam(&mut world, source.id, item);
    let wreck = world.wreck_manifest(source.id).unwrap();
    assert_eq!(
        wreck.cause,
        WreckDamageSource::Laser {
            weapon: OrdinaryWeapon::FrontBeamLaser,
            source: beam.source,
        }
    );
    assert!(matches!(beam.source, BeamSource::Player { item: paid_item, .. } if paid_item == item));
    assert_eq!(wreck.destroyed_at_tick, shot.tick);
    assert_eq!(wreck.destroyed_at_local_tick, beam.local_tick);
    assert_eq!(wreck.pose, beam.victim_pose);
    assert_eq!(wreck.impact_point, shot.end);
    assert_eq!((wreck.commodity, wreck.quantity), native_manifest);
    assert_eq!(world.commodity_quantity(wreck.commodity), cargo);
    assert_eq!(world.credits_tenths(), cash);
    let loaded = cold(&world);
    assert_eq!(loaded.wreck_manifest(source.id), Ok(wreck));
    assert_eq!(
        loaded.flight_resources_snapshot(),
        world.flight_resources_snapshot()
    );
}
