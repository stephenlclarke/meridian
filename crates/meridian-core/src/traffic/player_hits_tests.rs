//! Explicit component scenes; these fixtures are not public/native journey acceptance.
use super::{
    Arc, BeamSource, MAX_TRAFFIC_SHOTS, OriginalStationData, PlayerBeamInput, PlayerTarget,
    RigidPose, State, StationMarkets, SystemId, TickContext, Vec3, add, pose, scale, sub, unit,
};
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("unexpected refusal: {e:?}"))
}
fn scene() -> (
    State,
    StationMarkets,
    OriginalStationData,
    RigidPose,
    PlayerTarget,
) {
    let data = ok(OriginalStationData::bundled());
    let mut state = State::fresh(12001);
    let mut market = ok(StationMarkets::new_commander(12001));
    let player = pose(Vec3::new(0.0, 0.0, 7000.0), core::f64::consts::PI);
    for tick in 1..=900 {
        (state, market) = ok(state.prepare_step(
            TickContext {
                system: SystemId::AVENOR,
                data: &data,
                player_before: pose(Vec3::new(-600.0, 0.0, 1200.0), 0.0),
                player_after: pose(Vec3::new(-600.0, 0.0, 1200.0), 0.0),
                world_tick: tick,
                protected: false,
                celestial_clear: &|_, _, _| true,
                beam_clear: &|_, _, _| true,
                player_target: None,
            },
            &market,
        ));
    }
    let trader = state.actors[0]
        .as_mut()
        .unwrap_or_else(|| panic!("missing paid trader"));
    trader.position = Vec3::new(5000.0, 0.0, 6000.0);
    let pirate = state.actors[1]
        .as_mut()
        .unwrap_or_else(|| panic!("missing source pirate"));
    pirate.position = Vec3::new(0.0, 0.0, 6000.0);
    let aim = ok(data.player_planar_aimpoint(player, pirate.position))
        .unwrap_or_else(|| panic!("missing native planar surface"));
    let delta = sub(aim, pirate.position);
    pirate.yaw = delta.x.atan2(delta.z);
    pirate.speed = 0.0;
    (
        state,
        market,
        data,
        player,
        PlayerTarget {
            hull: 12002,
            visit: 2,
            physical_tick: 900,
            cargo: true,
        },
    )
}
fn fire(
    state: &mut State,
    data: &OriginalStationData,
    player: RigidPose,
    target: PlayerTarget,
    clear: bool,
) {
    ok(state.fire_npc_beams(
        SystemId::AVENOR,
        data,
        player,
        900,
        &|_, _, _| clear,
        Some(target),
    ));
}
#[test]
fn npc_player_receipt_has_native_first_contact_facing_and_exact_attacker_resource_debit() {
    let (mut state, market, data, player, target) = scene();
    let before = state.actors[1].unwrap_or_else(|| panic!("pirate"));
    fire(&mut state, &data, player, target, true);
    let hit = state.player_hits[0].clone();
    assert_eq!(hit.attacker, before.id);
    assert_eq!(hit.player_hull, 12002);
    assert_eq!(hit.source_pose, before.pose());
    assert_eq!(hit.player_pose, player);
    assert!(hit.front);
    let after = state.actors[1].unwrap_or_else(|| panic!("pirate"));
    assert_eq!(after.resources.energy, before.resources.energy - 100);
    assert_eq!(after.resources.heat, before.resources.heat + 50);
    assert_eq!(after.resources.cooldown, 18);
    assert_eq!(after.resources.shots, before.resources.shots + 1);
    ok(state.validate(12001, 900, 0, &data, &market));
    let mut w = crate::save::Writer::new();
    state.encode(&mut w);
    let mut r = crate::save::Reader::new(&w.bytes);
    assert_eq!(ok(State::decode(&mut r)), state);
    assert!(r.finished());
    let mut bad = state.clone();
    Arc::make_mut(&mut bad.player_hits)[0].front = false;
    assert!(bad.validate(12001, 900, 0, &data, &market).is_err());
    let mut bad = state.clone();
    let resources = &mut bad.actors[1]
        .as_mut()
        .unwrap_or_else(|| panic!("pirate"))
        .resources;
    resources.energy += 1;
    resources.energy_regen += 1; // aggregate conservation alone would accept this
    assert!(bad.validate(12001, 900, 0, &data, &market).is_err());
}
#[test]
fn empty_player_has_no_cargo_authority_and_backwards_or_blocked_beams_spend_nothing() {
    let (state, _, data, player, target) = scene();
    let mut empty = state.clone();
    fire(
        &mut empty,
        &data,
        player,
        PlayerTarget {
            cargo: false,
            ..target
        },
        true,
    );
    assert!(empty.player_hits.is_empty());
    let mut blocked = state.clone();
    fire(&mut blocked, &data, player, target, false);
    assert_eq!(blocked, state);
    let mut backwards = state.clone();
    backwards.actors[1]
        .as_mut()
        .unwrap_or_else(|| panic!("pirate"))
        .yaw = core::f64::consts::PI;
    let opening = backwards.clone();
    fire(&mut backwards, &data, player, target, true);
    assert_eq!(backwards, opening);
}
#[test]
fn actual_peer_mesh_occludes_and_missing_peer_or_changed_contact_cannot_load() {
    let (mut state, _, data, player, target) = scene();
    state.actors[0]
        .as_mut()
        .unwrap_or_else(|| panic!("trader"))
        .position = Vec3::new(0.0, 0.0, 6500.0);
    // Query the bounded native component boundary directly; this is not a public journey.
    let actor = state.actors[1].unwrap_or_else(|| panic!("pirate"));
    let peer = state.actors[0].unwrap_or_else(|| panic!("trader"));
    let surface = ok(data.player_planar_aimpoint(peer.pose(), actor.position))
        .unwrap_or_else(|| panic!("missing native peer surface"));
    let desired = add(actor.position, scale(actor.pose().orientation.z, 500.0));
    state.actors[0]
        .as_mut()
        .unwrap_or_else(|| panic!("trader"))
        .position = add(peer.position, sub(desired, surface));
    let actor = state.actors[1].unwrap_or_else(|| panic!("pirate"));
    let before = state.clone();
    ok(state.fire_at_player(
        1,
        actor,
        super::player_hits::PlayerEmission {
            data: &data,
            player,
            target,
            world_tick: 900,
            beam_clear: &|_, _, _| true,
        },
    ));
    assert_eq!(state, before);
    state.actors[0]
        .as_mut()
        .unwrap_or_else(|| panic!("trader"))
        .position = Vec3::new(5000.0, 0.0, 6000.0);
    fire(&mut state, &data, player, target, true);
    let hit = state.player_hits[0].clone();
    let mut missing = state.clone();
    Arc::make_mut(&mut missing.player_hits)[0].peers[0] = None;
    assert!(missing.validate_player_hits(900, &data, None).is_err());
    let mut bad = state.clone();
    Arc::make_mut(&mut bad.player_hits)[0].point.x = hit.point.x + 1.0;
    assert!(bad.validate_player_hits(900, &data, None).is_err());
}
#[test]
fn full_shared_beam_receipt_bound_refuses_npc_emission_without_spending_or_dropping() {
    let (mut state, _, data, player, target) = scene();
    fire(&mut state, &data, player, target, true);
    let hit = state.player_hits[0].clone();
    state.player_hits = Arc::new(vec![hit; MAX_TRAFFIC_SHOTS]);
    state.actors[1]
        .as_mut()
        .unwrap_or_else(|| panic!("pirate"))
        .resources
        .cooldown = 0;
    let before = state.clone();
    fire(&mut state, &data, player, target, true);
    assert_eq!(state, before);
    assert_eq!(state.receipt_count(), 128);
}
#[test]
fn actual_native_player_hit_admits_pirate_retaliation_without_claiming_player_cargo() {
    let (state, _, data, player, target) = scene();
    let pirate = state.actors[1].unwrap_or_else(|| panic!("pirate"));
    let aim = ok(data.player_planar_aimpoint(pirate.pose(), player.position))
        .unwrap_or_else(|| panic!("missing native pirate surface"));
    let direction = unit(sub(aim, player.position)).unwrap_or_else(|| panic!("invalid native aim"));
    let contact = state
        .beam(&data, SystemId::AVENOR, player.position, direction, 6000.0)
        .unwrap_or_else(|| panic!("missing actual pirate surface"));
    assert_eq!(contact.id.serial(), 2);
    let mut state = ok(state.prepare_player_hit(
        &data,
        PlayerBeamInput {
            weapon: crate::OrdinaryWeapon::FrontPulseLaser,
            system: SystemId::AVENOR,
            source: BeamSource::Player {
                owner: 12001,
                hull: 12002,
                item: 12005,
                emission: 1,
            },
            origin: player.position,
            direction,
            contact,
            world_tick: 900,
        },
    ));
    fire(
        &mut state,
        &data,
        player,
        PlayerTarget {
            cargo: false,
            ..target
        },
        true,
    );
    assert_eq!(state.player_hits.len(), 1);
    assert_eq!(state.player_hits[0].attacker, contact.id);
}

#[test]
fn a_peregrine_hit_before_medusa_issue_replays_against_the_earlier_player_mesh() {
    let (mut state, market, stock, player, target) = scene();
    fire(&mut state, &stock, player, target, true);
    assert_eq!(state.player_hits.len(), 1);
    let medusa = ok(OriginalStationData::medusa_bundled());
    ok(state.validate_with_hull_change(12001, 900, 0, &medusa, &market, Some(target.visit)));
    assert!(
        state
            .validate_player_hits(900, &medusa, Some(target.visit - 1))
            .is_err()
    );
    let mut wrong = state.clone();
    Arc::make_mut(&mut wrong.player_hits)[0].point.x += 1.0;
    assert!(
        wrong
            .validate_player_hits(900, &medusa, Some(target.visit))
            .is_err()
    );
}
