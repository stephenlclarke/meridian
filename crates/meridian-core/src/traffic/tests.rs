//! Component and real World tests. Runtime execution remains with the assigned existing lane.
use super::*;
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("unexpected refusal: {error:?}"),
    }
}
#[test]
fn all_seven_populations_keep_real_source_gates_and_never_alias_unadmitted_hulls() {
    assert_eq!(POPULATIONS.len(), 7);
    assert_eq!(POPULATIONS[0].required_source, "SH025");
    assert_eq!(POPULATIONS[1].required_source, "SH025");
    assert_eq!(POPULATIONS[2].gate, PopulationGate::RequiredHullUnavailable);
    assert_eq!(POPULATIONS[2].required_source, "StationPoliceSource: SH031");
    assert!(
        POPULATIONS[3..]
            .iter()
            .all(|p| p.gate != PopulationGate::ImplementedSourceUnqualified)
    );
}
fn some<T>(v: Option<T>) -> T {
    match v {
        Some(v) => v,
        None => panic!("missing admitted actor"),
    }
}
fn far_player() -> RigidPose {
    pose(Vec3::new(-600.0, 0.0, 1200.0), 0.0)
}
fn tick(
    state: &State,
    markets: &StationMarkets,
    data: &OriginalStationData,
    world_tick: u64,
) -> (State, StationMarkets) {
    state
        .prepare_step(
            TickContext {
                system: SystemId::AVENOR,
                data,
                player_before: far_player(),
                player_after: far_player(),
                world_tick,
                protected: false,
                celestial_clear: &|_, _, _| true,
                beam_clear: &|_, _, _| true,
                player_target: None,
            },
            markets,
        )
        .unwrap_or_else(|e| panic!("{e:?}"))
}
#[test]
fn deterministic_actual_hull_birth_pays_finite_stock_and_leaves_player_accounting_untouched() {
    let data = ok(OriginalStationData::bundled());
    let mut state = State::fresh(12005);
    let mut market = ok(StationMarkets::new_commander(12005));
    let opening = market.commodity_quote(MarketSite::Avenor, CommodityId::Food);
    for world_tick in 1..=480 {
        (state, market) = tick(&state, &market, &data, world_tick);
    }
    let actor = some(state.actors[0]);
    assert_eq!(actor.id.serial(), 1);
    assert_eq!(actor.snapshot().hull_id, "SH025");
    assert_eq!(actor.snapshot().population, Population::Trader);
    assert_eq!((actor.manifest, actor.units), (CommodityId::Food, 9));
    assert_eq!(actor.wallet, CAPITAL - 9 * 226);
    assert_eq!(
        market
            .commodity_quote(MarketSite::Avenor, CommodityId::Food)
            .stock,
        opening.stock - 9
    );
    assert_eq!(market.transaction_revision(), 0);
    assert!(market.validate_save(12005, 92, 92).is_err());
    assert!(
        market
            .validate_save_with_traffic(12005, 92, 92, state.flows())
            .is_ok()
    );
    ok(state.validate(12005, 480, 0, &data, &market));
    let duplicate = State::fresh(12005);
    let mut again = duplicate;
    let mut another = ok(StationMarkets::new_commander(12005));
    for time in 1..=480 {
        (again, another) = tick(&again, &another, &data, time);
    }
    assert_eq!(again, state);
    assert_eq!(
        another.commodity_quote(MarketSite::Avenor, CommodityId::Food),
        market.commodity_quote(MarketSite::Avenor, CommodityId::Food)
    );
}
#[test]
fn hull_and_arrival_clearance_or_missing_stock_never_mint_a_ghost_birth_or_payment() {
    let data = ok(OriginalStationData::bundled());
    let mut state = State::fresh(12005);
    state.clocks[0] = 479;
    let market = ok(StationMarkets::new_commander(12005));
    let launch = ok(data.launch_start());
    let player = some(crate::station::flight_pose(
        launch.position,
        launch.orientation,
    ));
    let (blocked, unchanged) = ok(state.prepare_step(
        TickContext {
            system: SystemId::AVENOR,
            data: &data,
            player_before: player,
            player_after: player,
            world_tick: 480,
            protected: false,
            celestial_clear: &|_, _, _| true,
            beam_clear: &|_, _, _| true,
            player_target: None,
        },
        &market,
    ));
    assert!(blocked.actors[0].is_none());
    assert_eq!(
        unchanged.quote(MarketSite::Avenor),
        market.quote(MarketSite::Avenor)
    );
    // Deliberately depleted arithmetic fixture, not player cargo or a valid fabricated market.
    let depleted =
        ok(market.prepare_traffic_stock(MarketSite::Avenor, CommodityId::Food, 128, true));
    let (blocked, still) = tick(&state, &depleted, &data, 480);
    assert!(blocked.actors[0].is_none());
    assert_eq!(still.quote(MarketSite::Avenor).stock, 0);
    let (blocked, still) = ok(state.prepare_step(
        TickContext {
            system: SystemId::AVENOR,
            data: &data,
            player_before: far_player(),
            player_after: far_player(),
            world_tick: 480,
            protected: false,
            celestial_clear: &|_, _, _| false,
            beam_clear: &|_, _, _| true,
            player_target: None,
        },
        &market,
    ));
    assert!(blocked.actors[0].is_none());
    assert_eq!(
        still.quote(MarketSite::Avenor),
        market.quote(MarketSite::Avenor)
    );
}
#[test]
fn exact_native_ray_and_conservative_sweep_use_the_existing_sh025_physical_mesh() {
    let data = ok(OriginalStationData::bundled());
    let mut state = State::fresh(12005);
    let mut market = ok(StationMarkets::new_commander(12005));
    for time in 1..=500 {
        (state, market) = tick(&state, &market, &data, time);
    }
    let actor = some(state.actors[0]);
    let origin = add(actor.position, Vec3::new(0.0, 0.0, -1000.0));
    let point = some(ok(data.player_planar_aimpoint(actor.pose(), origin)));
    let direction = some(unit(sub(point, origin)));
    let contact = some(state.beam(&data, SystemId::AVENOR, origin, direction, 2000.0));
    let exact = some(ok(data.player_beam_distance(
        actor.pose(),
        origin,
        direction,
        2000.0,
    )));
    assert_eq!(contact.distance, exact);
    assert_eq!(contact.id, actor.id);
    let medusa = ok(OriginalStationData::medusa_bundled());
    assert_ne!(
        ok(medusa.player_beam_distance(actor.pose(), origin, direction, 2000.0)),
        Some(exact)
    );
    assert_eq!(
        state.beam(&medusa, SystemId::AVENOR, origin, direction, 2000.0),
        Some(contact)
    );
    assert!(
        state
            .beam(&data, SystemId::RENAS, origin, direction, 2000.0)
            .is_none()
    );
    assert!(!state.player_motion_clear(
        SystemId::AVENOR,
        origin,
        add(actor.position, Vec3::new(0.0, 0.0, 1000.0)),
        data.player_npc_clearance()
    ));
    assert!(state.player_motion_clear(
        SystemId::RENAS,
        origin,
        actor.position,
        data.player_npc_clearance()
    ));
}
#[test]
fn ship_resources_preserve_generated_stock_real_heat_energy_cooldown_and_damage_conservation() {
    let mut resources = Resources::fresh();
    ok(resources.fire());
    assert_eq!(
        (resources.energy, resources.heat, resources.cooldown),
        (37900, 50, 18)
    );
    let before = resources;
    assert!(resources.fire().is_err());
    assert_eq!(resources, before);
    for _ in 0..18 {
        ok(resources.advance());
    }
    assert_eq!(resources.cooldown, 0);
    ok(resources.hit(true));
    assert_eq!(resources.front, 28500);
    assert_eq!(resources.rear, 30000);
    ok(resources.validate(19, 1, 1));
    let mut forged = resources;
    forged.front += 1;
    assert!(forged.validate(19, 1, 1).is_err());
    resources.heat = 1151;
    assert!(resources.fire().is_err());
}
#[test]
fn foreign_role_identity_cash_and_damage_or_oversized_codec_refuse_before_use() {
    let data = ok(OriginalStationData::bundled());
    let mut state = State::fresh(12005);
    let mut market = ok(StationMarkets::new_commander(12005));
    for time in 1..=480 {
        (state, market) = tick(&state, &market, &data, time);
    }
    let original = some(state.actors[0]);
    for corrupted in [
        Actor {
            id: NpcId {
                serial: 3,
                ..original.id
            },
            ..original
        },
        Actor {
            wallet: original.wallet + 1,
            ..original
        },
        Actor {
            resources: Resources {
                front: 29999,
                ..original.resources
            },
            ..original
        },
        Actor {
            position: Vec3::new(f64::NAN, 0.0, 0.0),
            ..original
        },
    ] {
        let mut bad = state.clone();
        bad.actors[0] = Some(corrupted);
        assert!(bad.validate(12005, 480, 0, &data, &market).is_err());
    }
    let empty = State::fresh(12005);
    let mut writer = crate::save::Writer::new();
    empty.encode(&mut writer);
    let index = writer.bytes.len() - 2;
    writer.bytes[index..].copy_from_slice(&129_u16.to_le_bytes());
    assert!(State::decode(&mut crate::save::Reader::new(&writer.bytes)).is_err());
}
#[test]
fn prefix_flow_history_keeps_actual_purchase_tick_and_empty_absent_state_never_regenerates() {
    let data = ok(OriginalStationData::bundled());
    let mut state = State::fresh(12005);
    let mut market = ok(StationMarkets::new_commander(12005));
    for time in 1..=480 {
        (state, market) = tick(&state, &market, &data, time);
    }
    assert_eq!(state.flows_at(479), Flows::default());
    assert_eq!(state.flows_at(480).bought[0][0], 9);
    assert_eq!(state.flows().sold[0][0], 0);
    let absent = State::default();
    let market = ok(StationMarkets::new_commander(12005));
    let (again, market) = tick(&absent, &market, &data, 10000);
    assert_eq!(again, absent);
    assert_eq!(market.quote(MarketSite::Avenor).stock, 128);
    assert!(again.snapshot(SystemId::AVENOR).is_none());
}
#[test]
fn described_remote_sale_cannot_replace_a_real_native_entry_receipt_even_when_money_balances() {
    let data = ok(OriginalStationData::bundled());
    let mut state = State::fresh(12005);
    let mut market = ok(StationMarkets::new_commander(12005));
    for time in 1..=480 {
        (state, market) = tick(&state, &market, &data, time);
    }
    let mut actor = some(state.actors[0]);
    actor.sold = true;
    actor.sold_world_tick = Some(480);
    actor.phase = NpcPhase::Complete;
    actor.speed = 0.0;
    actor.wallet += 9 * 208;
    state.actors[0] = Some(actor);
    let matching_stock =
        ok(market.prepare_traffic_stock(MarketSite::Avenor, CommodityId::Food, 9, false));
    assert!(
        matching_stock
            .validate_save_with_traffic(12005, 92, 92, state.flows())
            .is_ok()
    );
    assert!(
        state
            .validate(12005, 480, 0, &data, &matching_stock)
            .is_err()
    );
    actor.berth = Some(BerthReceipt {
        before: actor.position,
        yaw: actor.yaw,
        speed: 4.0,
        angular: 0.0,
    });
    state.actors[0] = Some(actor);
    assert!(
        state
            .validate(12005, 480, 0, &data, &matching_stock)
            .is_err()
    );
}
#[test]
fn actual_npc_beam_visibility_refusal_spends_no_laser_or_damage_receipt() {
    let data = ok(OriginalStationData::bundled());
    let mut state = State::fresh(12001);
    let mut market = ok(StationMarkets::new_commander(12001));
    let mut observed = false;
    for time in 1..=12000 {
        let (next, updated) = tick(&state, &market, &data, time);
        if next.shots.len() > state.shots.len() {
            let (blocked, _) = ok(state.prepare_step(
                TickContext {
                    system: SystemId::AVENOR,
                    data: &data,
                    player_before: far_player(),
                    player_after: far_player(),
                    world_tick: time,
                    protected: false,
                    celestial_clear: &|_, _, _| true,
                    beam_clear: &|_, _, _| false,
                    player_target: None,
                },
                &market,
            ));
            assert_eq!(blocked.shots, state.shots);
            assert_eq!(
                some(blocked.actors[1]).resources.shots,
                some(state.actors[1]).resources.shots
            );
            assert_eq!(
                some(blocked.actors[0]).resources.incoming,
                some(state.actors[0]).resources.incoming
            );
            observed = true;
            break;
        }
        state = next;
        market = updated;
    }
    assert!(
        observed,
        "no actual NPC contact reached the obstruction boundary"
    );
}

#[test]
fn native_death_freezes_resources_but_retained_scenes_keep_the_corpse_without_resurrection() {
    let data = ok(OriginalStationData::bundled());
    let mut state = State::fresh(12001);
    let mut market = ok(StationMarkets::new_commander(12001));
    let mut death = None;
    for time in 1..=12000 {
        let before = state.missile_scene(SystemId::AVENOR, time - 1, SceneStage::Command(u64::MAX));
        (state, market) = tick(&state, &market, &data, time);
        if state.actors[0].is_some_and(|actor| actor.phase == NpcPhase::Destroyed) {
            death = Some((time, before));
            break;
        }
    }
    let (time, before_death) = some(death);
    let corpse = some(state.actors[0]);
    assert_eq!(corpse.resources.energy, 0);
    assert_eq!(corpse.resource_clock, state.clocks[0]);
    assert!(state.shots.iter().any(|shot| shot.victim == corpse.id));
    ok(state.validate(12001, time, 0, &data, &market));
    (state, market) = tick(&state, &market, &data, time + 1);
    assert_eq!(some(state.actors[0]), corpse);
    assert!(state.clocks[0] > corpse.resource_clock);
    ok(state.validate(12001, time + 1, 0, &data, &market));
    let scene = state.missile_scene(SystemId::AVENOR, time + 1, SceneStage::Command(u64::MAX));
    assert!(
        scene
            .bodies
            .iter()
            .any(|body| body.id == corpse.id && body.phase == NpcPhase::Destroyed)
    );
    // The generic resource replay must STILL refuse post-death advancement.
    assert!(
        state
            .resources_at(corpse, scene.local_tick, scene.world_tick, scene.stage)
            .is_err()
    );
    ok(state.validate_missile_scene(&scene, time + 1, &data));
    // The earlier live scene must use its historical clock, not the later death state.
    assert!(before_death.local_tick < corpse.resource_clock);
    ok(state.validate_missile_scene(&before_death, time + 1, &data));
    let mut resurrected = scene.clone();
    resurrected
        .bodies
        .iter_mut()
        .find(|body| body.id == corpse.id)
        .unwrap_or_else(|| panic!("retained native corpse missing from resurrection fixture"))
        .phase = NpcPhase::Hunting;
    assert!(
        state
            .validate_missile_scene(&resurrected, time + 1, &data)
            .is_err()
    );
    let mut omitted = scene;
    omitted.bodies.retain(|body| body.id != corpse.id);
    assert!(
        state
            .validate_missile_scene(&omitted, time + 1, &data)
            .is_err()
    );
    assert_eq!(some(state.actors[0]), corpse);
}
