//! Real paid/public command journeys. No pose, balance, tool or save injection supplies success.
use super::*;
use crate::flight::missiles::tests::{dock, fly_axis, stop};
use crate::flight::practice::tests::{aim_at, launch};
use crate::{FlightInput, ModuleAction, MotionRefusal, PulseContact};
fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn some<T>(v: Option<T>) -> T {
    v.unwrap_or_else(|| panic!("missing actual state"))
}
fn steps(w: &mut World, n: usize) {
    for _ in 0..n {
        ok(w.step(FlightInput::default()));
    }
}
fn module(w: &mut World, action: ModuleAction) -> u64 {
    let tick = w.snapshot().tick;
    let review = ok(w.review_module(action, tick));
    ok(w.prepare_module(review, tick)).commit()
}
fn paid(seed: u64) -> (World, u64) {
    let mut w = crate::flight::equipment::tests::earned_world(seed);
    if some(w.snapshot().navigation).fuel_units < 68 {
        let t = w.snapshot().tick;
        ok(w.request_refuel(t));
    }
    crate::flight::equipment::tests::journey(&mut w, SystemId::AVENOR);
    let cash = some(w.credits_tenths());
    let item = module(&mut w, ModuleAction::PurchaseMiningLaser);
    assert_eq!(some(w.credits_tenths()), cash - 33000);
    let fit = some(w.fitted_mining_tool());
    assert_eq!((fit.item, fit.damage, fit.range), (item, 50, 6000));
    if some(w.snapshot().navigation).fuel_units < 68 {
        let t = w.snapshot().tick;
        ok(w.request_refuel(t));
    }
    (w, item)
}
fn approach(w: &mut World) {
    launch(w);
    stop(w);
    let body = some(w.mining_belt_snapshot()).bodies[0];
    aim_at(w, body.position);
    for _ in 0..1800 {
        let p = some(flight_pose(w.snapshot().position, w.snapshot().orientation)).position;
        let d = crate::docking::Vec3::new(
            body.position.x - p.x,
            body.position.y - p.y,
            body.position.z - p.z,
        );
        if d.dot(d).sqrt() < 4200.0 {
            stop(w);
            assert_eq!(
                some(some(w.mining_belt_snapshot()).front_contact).asteroid,
                body.id
            );
            return;
        }
        ok(w.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        }));
        assert_eq!(w.snapshot().motion_refusal, None);
    }
    panic!("paid pilot never reached native work surface");
}
fn work(w: &mut World) -> MiningShot {
    let tool = ok(w.mining_tool_authority());
    let t = w.snapshot().tick;
    ok(w.request_mining(tool, t))
}
fn cold(w: &World) -> World {
    let loaded = ok(World::from_save_bytes(&ok(w.save_bytes())));
    assert_eq!(loaded.mining_belt_snapshot(), w.mining_belt_snapshot());
    assert_eq!(
        loaded.flight_resources_snapshot(),
        w.flight_resources_snapshot()
    );
    assert_eq!(loaded.credits_tenths(), w.credits_tenths());
    assert_eq!(loaded.service_snapshot(), w.service_snapshot());
    assert_eq!(
        loaded
            .fitted_mining_tool()
            .map(|t| (t.owner, t.hull, t.item, t.account_revision)),
        w.fitted_mining_tool()
            .map(|t| (t.owner, t.hull, t.item, t.account_revision))
    );
    loaded
}

#[test]
fn paid_scoops_take_one_physical_fragment_once_and_sell_it_after_cold_continue() {
    let (mut world, _) = paid(12005);
    module(&mut world, ModuleAction::PurchaseFuelScoops);
    approach(&mut world);
    let cash = world.credits_tenths();
    work(&mut world);
    steps(&mut world, 18);
    let shot = work(&mut world);
    let t = world.snapshot().tick;
    let before = ok(world.save_bytes());
    assert_eq!(
        world.request_collect_mined_ore(t - 1),
        Err(MiningError::StaleCommand)
    );
    assert_eq!(
        world.request_collect_mined_ore(t),
        Err(MiningError::NoEligibleOreBody)
    );
    assert_eq!(ok(world.save_bytes()), before);

    // Real flight controls only. Approach the drifting source horizontally, then rotate
    // the lower-forward aperture towards the fragment while stopped.
    for _ in 0..2200 {
        let ore = some(world.mining_collection_target(shot.contact.asteroid));
        let player = some(flight_pose(
            world.snapshot().position,
            world.snapshot().orientation,
        ));
        let horizontal =
            (ore.position.x - player.position.x).hypot(ore.position.z - player.position.z);
        let speed = world.snapshot().speed;
        if horizontal < 65.0 && speed == 0.0 {
            break;
        }
        let desired = ((2.0 * 9.1 * (horizontal - 58.0).max(0.0) / 18.0).sqrt() as f32).min(30.0);
        ok(world.step(FlightInput {
            throttle: ((desired - speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
            ..FlightInput::default()
        }));
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    stop(&mut world);
    let mut collected = false;
    for _ in 0..100 {
        let tick = world.snapshot().tick;
        match world.request_collect_mined_ore(tick) {
            Ok(()) => {
                collected = true;
                break;
            }
            Err(MiningError::NoEligibleOreBody) => {}
            Err(error) => panic!("unexpected actual scoop refusal: {error:?}"),
        }
        ok(world.step(FlightInput {
            pitch: 1.0,
            ..FlightInput::default()
        }));
    }
    assert!(
        collected,
        "physical controls never reached the lower-forward fragment"
    );
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(world.commodity_quantity(shot.commodity), Some(1));
    assert!(some(world.mining_belt_snapshot()).pending_ore.is_empty());
    let mut loaded = cold(&world);
    assert_eq!(loaded.commodity_quantity(shot.commodity), Some(1));
    let before = ok(loaded.save_bytes());
    let tick = loaded.snapshot().tick;
    assert_eq!(
        loaded.request_collect_mined_ore(tick),
        Err(MiningError::NoEligibleOreBody)
    );
    assert_eq!(ok(loaded.save_bytes()), before);
    // Route away from the empty source with public controls before a normal station sale.
    // Returning the pitch to horizontal avoids a vertical drift on the dock helper.
    let current = some(flight_pose(
        loaded.snapshot().position,
        loaded.snapshot().orientation,
    ));
    let pitch = current.orientation.z.y.asin();
    let ticks = (pitch.abs() / (0.7 / 60.0)).ceil() as u16;
    if ticks != 0 {
        for _ in 0..ticks {
            ok(loaded.step(FlightInput {
                pitch: -(pitch / (0.7 / 60.0) / f64::from(ticks)) as f32,
                ..FlightInput::default()
            }));
        }
    }
    fly_axis(&mut loaded, 0, 0.0, 10.0);
    dock(&mut loaded);
    let tick = loaded.snapshot().tick;
    ok(loaded.request_station_service(
        crate::station_services::Command::SellCommodity(shot.commodity),
        tick,
    ));
    assert_eq!(loaded.commodity_quantity(shot.commodity), Some(0));
    assert!(some(loaded.credits_tenths()) > some(cash));
    let _ = cold(&loaded);
}

#[test]
fn explicit_max_owns_stored_mining_tool_and_reversible_exchange_preserves_shared_pools() {
    use crate::{ModuleKind, ModuleSlot, XyzzyAction};
    let mut world = ok(World::new_commander(12005));
    let review = ok(world.review_xyzzy(XyzzyAction::Enter, 0));
    ok(world.prepare_xyzzy(review, 0)).commit();
    let items = some(world.modular_equipment_snapshot()).items;
    let mining = some(
        items
            .iter()
            .find(|i| i.kind == ModuleKind::MiningLaser)
            .copied(),
    );
    let naval = some(
        items
            .iter()
            .find(|i| i.kind == ModuleKind::NavalLaser)
            .copied(),
    );
    assert_eq!(mining.slot, None);
    assert_eq!(naval.slot, Some(ModuleSlot::FrontLaser));
    use crate::station_services::{CargoUnit, Command, CommodityId};
    for commodity in CommodityId::ALL
        .into_iter()
        .filter(|id| id.unit() == CargoUnit::Tonne && id.departure_legal_weight() == 0)
    {
        loop {
            let service = some(world.service_snapshot());
            if service.used_tonnes == service.hold_capacity_tonnes
                || some(world.commodity_quote(commodity)).stock == 0
            {
                break;
            }
            let tick = world.snapshot().tick;
            ok(world.request_station_service(Command::BuyCommodity(commodity), tick));
        }
    }
    let full = some(world.service_snapshot());
    assert_eq!(full.used_tonnes, full.hold_capacity_tonnes);
    let unchanged = ok(world.save_bytes());
    assert_eq!(
        world.prepare_mining_intake_account(CommodityId::Alloys),
        Err(MiningError::HoldFull)
    );
    assert!(
        world
            .prepare_mining_intake_account(CommodityId::Gold)
            .is_ok()
    );
    assert_eq!(ok(world.save_bytes()), unchanged);
    launch(&mut world);
    stop(&mut world);
    let target = some(world.mining_belt_snapshot()).bodies[0].id;
    ok(world.cycle_target_filter());
    let tick = world.snapshot().tick;
    ok(world.cycle_targeting_with_work(
        tick,
        crate::TargetingViewport {
            size: [800, 600],
            front: Vec::new(),
        },
        &[crate::equipment::WorkTargetId::Asteroid(target)],
    ));
    let cash = world.credits_tenths();
    let pools = world.flight_resources_snapshot();
    ok(world.select_auto_mining_tool());
    assert_eq!(some(world.fitted_mining_tool()).item, mining.id);
    assert_eq!(world.credits_tenths(), cash);
    assert_eq!(world.flight_resources_snapshot(), pools);
    let before = ok(world.save_bytes());
    assert_eq!(world.auto_mine_tick(target), "Cargo hold full");
    assert_eq!(ok(world.save_bytes()), before);
    let mut cold_world = cold(&world);
    world.cancel_targeting();
    assert_eq!(world.auto_mine_tick(target), "Target released");
    assert_eq!(ok(world.save_bytes()), before);
    ok(world.restore_auto_mining_tool());
    ok(cold_world.restore_auto_mining_tool());
    for restored in [&mut world, &mut cold_world] {
        let fit = some(restored.modular_equipment_snapshot());
        assert_eq!(
            some(fit.items.iter().find(|item| item.id == naval.id)).slot,
            Some(ModuleSlot::FrontLaser)
        );
        assert_eq!(
            some(fit.items.iter().find(|item| item.id == mining.id)).slot,
            None
        );
        let before = ok(restored.save_bytes());
        ok(restored.restore_auto_mining_tool());
        assert_eq!(ok(restored.save_bytes()), before);
        assert_eq!(restored.flight_resources_snapshot(), pools);
        assert_eq!(restored.credits_tenths(), cash);
    }
    let _ = cold(&world);
}
#[test]
fn genuinely_paid_tool_applies_original_partial_damage_then_releases_pending_ore_without_cargo_or_cash_credit()
 {
    let (mut w, item) = paid(12005);
    approach(&mut w);
    let cash = w.credits_tenths();
    let cargo = w.service_snapshot();
    let opening = w.raw_service_projection().map(|s| s.revision);
    let first = work(&mut w);
    assert_eq!(
        (first.item_id, first.extracted_units, first.remaining_energy),
        (item, 0, 50.0)
    );
    assert!(some(w.mining_belt_snapshot()).pending_ore.is_empty());
    let fresh = ok(w.mining_tool_authority());
    let tick = w.snapshot().tick;
    let before = ok(w.save_bytes());
    assert_eq!(w.request_mining(fresh, tick), Err(MiningError::Resources));
    assert_eq!(ok(w.save_bytes()), before);
    let resources = some(w.flight_resources_snapshot());
    assert_eq!(
        (
            resources.energy_hundredths,
            resources.heat_fifths,
            resources.cooldown_ticks
        ),
        (37900, 50, 18)
    );
    let mut loaded = cold(&w);
    steps(&mut w, 18);
    steps(&mut loaded, 18);
    let second = work(&mut w);
    let copy = work(&mut loaded);
    assert_eq!(second, copy);
    assert_eq!((second.extracted_units, second.remaining_energy), (1, 0.0));
    let pending = some(w.mining_belt_snapshot()).pending_ore;
    assert_eq!(pending.len(), 1);
    assert_eq!(
        (
            pending[0].id.asteroid(),
            pending[0].id.fragment(),
            pending[0].quantity
        ),
        (first.contact.asteroid, 0, 1)
    );
    assert_eq!(w.service_snapshot(), cargo);
    assert_eq!(w.credits_tenths(), cash);
    assert_eq!(w.raw_service_projection().map(|s| s.revision), opening);
    assert_eq!(some(w.flight_resources_snapshot()).heat_fifths, 82);
    let _continued = cold(&w);
}
#[test]
fn actual_lease_scope_and_emission_refuse_cross_continue_stale_pause_and_failed_restore_without_a_stroke()
 {
    let (mut w, _) = paid(12005);
    approach(&mut w);
    let bytes = ok(w.save_bytes());
    let tool = ok(w.mining_tool_authority());
    let mut other = ok(World::from_save_bytes(&bytes));
    let t = other.snapshot().tick;
    assert_eq!(
        other.request_mining(tool, t),
        Err(MiningError::NoCompatibleFittedTool)
    );
    let before = ok(other.save_bytes());
    let tool = ok(other.mining_tool_authority());
    assert_eq!(
        other.request_mining(tool, t - 1),
        Err(MiningError::StaleCommand)
    );
    assert_eq!(ok(other.save_bytes()), before);
    let stale = ok(w.mining_tool_authority());
    work(&mut w);
    steps(&mut w, 18);
    let before = ok(w.save_bytes());
    let t = w.snapshot().tick;
    assert_eq!(
        w.request_mining(stale, t),
        Err(MiningError::NoCompatibleFittedTool)
    );
    assert_eq!(ok(w.save_bytes()), before);
    let tool = ok(w.mining_tool_authority());
    ok(w.request_pause(true, t));
    let paused = ok(w.save_bytes());
    assert_eq!(w.request_mining(tool, t), Err(MiningError::Paused));
    assert_eq!(ok(w.save_bytes()), paused);
    assert!(w.restore_bytes(&bytes[..bytes.len() - 1]).is_err());
    assert_eq!(ok(w.save_bytes()), paused);
}
#[test]
fn no_scoops_is_an_honest_no_transfer_and_no_market_sale_source_after_actual_paid_depletion() {
    let (mut w, item) = paid(12005);
    approach(&mut w);
    work(&mut w);
    steps(&mut w, 18);
    let shot = work(&mut w);
    let before = ok(w.save_bytes());
    let t = w.snapshot().tick;
    assert_eq!(w.request_collect_mined_ore(t), Err(MiningError::NoScoops));
    assert_eq!(ok(w.save_bytes()), before);
    fly_axis(&mut w, 0, 0.0, 10.0);
    dock(&mut w);
    assert_eq!(w.commodity_quantity(shot.commodity), Some(0));
    let before = ok(w.save_bytes());
    let t = w.snapshot().tick;
    assert_eq!(
        w.request_station_service(
            crate::station_services::Command::SellCommodity(shot.commodity),
            t
        ),
        Err(crate::MissionError::Service(
            crate::station_services::Error::NoSellableCargo
        ))
    );
    assert_eq!(ok(w.save_bytes()), before);
    assert_eq!(some(w.fitted_mining_tool()).item, item);
    let _continued = cold(&w);
}
#[test]
fn paid_store_refit_invalidates_capability_but_never_resets_retained_work_or_shared_energy() {
    let (mut w, item) = paid(12005);
    approach(&mut w);
    work(&mut w);
    fly_axis(&mut w, 0, 0.0, 10.0);
    dock(&mut w);
    let old = ok(w.mining_tool_authority());
    let before = some(w.mining_belt_snapshot()).bodies;
    module(&mut w, ModuleAction::Store { item });
    assert!(w.mining_tool_authority().is_err());
    module(
        &mut w,
        ModuleAction::Install {
            item,
            replacing: None,
        },
    );
    assert_eq!(some(w.mining_belt_snapshot()).bodies, before);
    approach(&mut w);
    let t = w.snapshot().tick;
    let bytes = ok(w.save_bytes());
    assert_eq!(
        w.request_mining(old, t),
        Err(MiningError::NoCompatibleFittedTool)
    );
    assert_eq!(ok(w.save_bytes()), bytes);
    assert_eq!(work(&mut w).extracted_units, 1);
    let _continued = cold(&w);
}
#[test]
fn actual_jump_expires_uncollected_ore_and_cold_return_never_releases_the_same_body_twice() {
    let (mut w, _) = paid(12005);
    approach(&mut w);
    work(&mut w);
    steps(&mut w, 18);
    work(&mut w);
    let mut w = cold(&w);
    assert_eq!(some(w.mining_belt_snapshot()).pending_ore.len(), 1);
    let tick = w.snapshot().tick;
    ok(w.request_jump(SystemId::RENAS, tick));
    steps(&mut w, crate::navigation::BASIC_SPOOL_TICKS as usize);
    assert!(w.mining_belt_snapshot().is_none());
    let mut w = cold(&w);
    dock(&mut w);
    if some(w.snapshot().navigation).fuel_units < 34 {
        let t = w.snapshot().tick;
        ok(w.request_refuel(t));
    }
    crate::flight::equipment::tests::journey(&mut w, SystemId::AVENOR);
    let belt = some(w.mining_belt_snapshot());
    assert_eq!(belt.bodies[0].remaining_energy, 0.0);
    assert!(belt.pending_ore.is_empty());
    assert_eq!(w.mining.work_count(), 2);
    let _continued = cold(&w);
}
#[test]
fn unchanged_pulse_and_real_swept_hull_obey_the_visible_source_rock_without_mining_it() {
    let mut w = ok(World::new_commander(12005));
    approach(&mut w);
    let before = some(w.mining_belt_snapshot()).bodies;
    let tick = w.snapshot().tick;
    let shot = ok(w.request_fire_front_pulse(tick));
    assert!(matches!(shot.contact, PulseContact::AsteroidBlocked { .. }));
    assert_eq!(some(w.mining_belt_snapshot()).bodies, before);
    assert_eq!(w.mining.work_count(), 0);
    let mut collided = false;
    for _ in 0..900 {
        let before = w.snapshot();
        ok(w.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        }));
        if w.snapshot().motion_refusal == Some(MotionRefusal::AsteroidContact) {
            assert_eq!(w.snapshot().position, before.position);
            assert_eq!(w.snapshot().speed, 0.0);
            collided = true;
            break;
        }
    }
    assert!(collided);
    let _continued = cold(&w);
}
#[test]
fn all_provider_rock_envelopes_clear_the_actual_celestial_scene() {
    let w = ok(World::new_commander(12005));
    let scene = some(w.celestial_snapshot());
    for rock in some(w.mining_belt_snapshot()).bodies {
        let p = [
            rock.position.x as f32,
            rock.position.y as f32,
            rock.position.z as f32,
        ];
        for body in [scene.planet, scene.sun] {
            assert!(!body.blocks_motion(p, p, rock.bounding_radius()));
        }
    }
}
#[test]
fn explicit_old_fifteen_save_refuses_before_mining_state_changes() {
    // Codec compatibility fixture only; no gameplay yield, item, pose or money is injected.
    let mut fresh = ok(World::new_commander(12005));
    let old = ok(crate::flight::persistence::unsupported_version_test_input(
        &fresh, 15,
    ));
    assert_eq!(u16::from_le_bytes([old[8], old[9]]), 15);
    assert!(matches!(
        World::from_save_bytes(&old),
        Err(crate::save::SaveError::UnsupportedVersion)
    ));
    let before = ok(fresh.save_bytes());
    assert_eq!(
        fresh.restore_bytes(&old),
        Err(crate::save::SaveError::UnsupportedVersion)
    );
    assert_eq!(ok(fresh.save_bytes()), before);
}
#[test]
fn original_work_has_no_invented_slow_speed_gate_and_never_changes_player_motion() {
    let (mut w, _) = paid(12005);
    approach(&mut w);
    for _ in 0..90 {
        ok(w.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        }));
    }
    let motion = w.snapshot();
    assert!(motion.speed > 2.0);
    let shot = work(&mut w);
    assert_eq!(shot.remaining_energy, 50.0);
    assert_eq!(w.snapshot(), motion);
    let _continued = cold(&w);
}

#[test]
fn paid_mining_round_trips_after_a_public_change_to_born_energy_fit() {
    let (mut world, tool) = paid(12005);
    let inventory = some(world.modular_equipment_snapshot());
    let energy = inventory
        .items
        .iter()
        .find(|item| item.kind == crate::ModuleKind::BasicEnergySystem)
        .unwrap_or_else(|| panic!("missing born energy system"));
    let energy = energy.id;
    assert_eq!(
        module(&mut world, ModuleAction::Store { item: energy }),
        energy
    );
    approach(&mut world);
    assert_eq!(work(&mut world).item_id, tool);
    let loaded = cold(&world);
    assert_eq!(
        loaded.modular_equipment_snapshot(),
        world.modular_equipment_snapshot()
    );
}

#[test]
fn secret_outfit_preserves_paid_mining_work_custody_and_refuses_inflight_reactivation() {
    let (mut w, item) = paid(15603);
    let tick = w.snapshot().tick;
    let review = ok(w.review_xyzzy(crate::XyzzyAction::Enter, tick));
    ok(w.prepare_xyzzy(review, tick)).commit();
    let naval = some(w.front_laser_snapshot()).fitted_item_id;
    module(
        &mut w,
        ModuleAction::Install {
            item,
            replacing: Some(naval),
        },
    );
    approach(&mut w);
    work(&mut w);
    steps(&mut w, 18);
    assert_eq!(work(&mut w).extracted_units, 1);
    let before = ok(w.save_bytes());
    let belt = w.mining_belt_snapshot();
    let cargo = w.service_snapshot();
    assert_eq!(
        w.review_xyzzy(crate::XyzzyAction::Enter, w.snapshot().tick),
        Err(crate::XyzzyError::BerthRequired)
    );
    assert_eq!(ok(w.save_bytes()), before);
    let continued = cold(&w);
    assert_eq!(continued.mining_belt_snapshot(), belt);
    assert_eq!(continued.service_snapshot(), cargo);
    assert_eq!(some(continued.fitted_mining_tool()).item, item);
    assert_eq!(some(continued.mining_belt_snapshot()).pending_ore.len(), 1);
}
