//! Native geometry/resource component cases, not claimed as public gameplay witnesses.
use super::super::{StationMarkets, TickContext, unit};
use super::*;
fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn born() -> (State, OriginalStationData) {
    let data = ok(OriginalStationData::bundled());
    let mut s = State::fresh(12001);
    let mut market = ok(StationMarkets::new_commander(12001));
    let player = pose(Vec3::new(-600.0, 0.0, 1200.0), 0.0);
    for tick in 1..=900 {
        (s, market) = ok(s.prepare_step(
            TickContext {
                system: SystemId::AVENOR,
                data: &data,
                player_before: player,
                player_after: player,
                world_tick: tick,
                protected: false,
                celestial_clear: &|_, _, _| true,
                beam_clear: &|_, _, _| true,
                player_target: None,
            },
            &market,
        ));
    }
    (s, data)
}
#[test]
fn same_native_mesh_selects_nearest_actor_independently_of_scene_vector_order() {
    let (state, data) = born();
    let mut scene = state.missile_scene(SystemId::AVENOR, 900, SceneStage::Step(0));
    assert_eq!(scene.bodies.len(), 2);
    let target = scene.bodies[0];
    let origin = add(target.pose.position, Vec3::new(0.0, 0.0, -500.0));
    let point = ok(data.player_missile_aimpoint(target.pose, origin));
    let direction = unit(sub(point, origin)).unwrap_or_else(|| panic!("invalid native aim"));
    let first = scene
        .contact(&data, origin, direction, 6000.0)
        .unwrap_or_else(|| panic!("native mesh was missed"));
    scene.bodies.reverse();
    let reversed = scene
        .contact(&data, origin, direction, 6000.0)
        .unwrap_or_else(|| panic!("reordered mesh was missed"));
    assert_eq!(first, reversed);
    assert_eq!(first.id, target.id);
    assert_eq!(
        Some(first.distance),
        ok(data.player_beam_distance(target.pose, origin, direction, 6000.0))
    );
}
#[test]
fn mixed_missile_and_pulse_damage_keeps_facing_hull_overkill_and_exact_recharge() {
    let mut r = Resources::fresh();
    ok(r.hit_missile(true));
    assert_eq!((r.front, r.rear, r.energy), (14000, 30000, 38000));
    ok(r.hit(false));
    ok(r.hit_missile(true));
    assert_eq!((r.front, r.rear, r.energy), (0, 28500, 36000));
    ok(r.advance_exact(10));
    assert_eq!((r.front, r.rear, r.energy), (30, 28530, 36070));
    ok(r.hit_missile(true));
    ok(r.hit_missile(true));
    ok(r.hit_missile(true));
    assert_eq!(r.energy, 0);
    assert_eq!(r.incoming_missiles, 5);
    assert_eq!(r.incoming, 1);
    assert_eq!(r.overkill, 11900);
    ok(r.validate(10, 0, 1));
    let mut forged = r;
    forged.incoming_missiles -= 1;
    assert!(forged.validate(10, 0, 1).is_err());
}
#[test]
fn native_scene_codec_retains_exact_stage_identity_pose_and_all_actors() {
    let (state, data) = born();
    let scene = state.missile_scene(SystemId::AVENOR, 900, SceneStage::Command(0));
    let mut w = Writer::new();
    scene.encode(&mut w);
    let mut reader = Reader::new(&w.bytes);
    let decoded = ok(MissileScene::decode(&mut reader));
    assert_eq!(decoded, scene);
    ok(state.validate_missile_scene(&decoded, 900, &data));
    let mut missing = decoded.clone();
    missing.bodies.pop();
    assert!(state.validate_missile_scene(&missing, 900, &data).is_err());
    let mut duplicate = decoded.clone();
    duplicate.bodies.push(duplicate.bodies[0]);
    assert!(
        state
            .validate_missile_scene(&duplicate, 900, &data)
            .is_err()
    );
    let mut tilted = decoded;
    tilted.bodies[0].pose.orientation = Basis3::IDENTITY;
    tilted.bodies[0].pose.position.y = 1.0;
    assert!(state.validate_missile_scene(&tilted, 900, &data).is_err());
}
#[test]
fn proof_budget_counts_distinct_missile_hits_without_raising_or_truncating_128() {
    assert_eq!(MAX_TRAFFIC_SHOTS, 128);
    assert_eq!(crate::save::MAX_SAVE_BYTES, 65536);
    let (state, _) = born();
    assert_eq!(
        state.combat_proof_count(),
        state.shots.len() + state.missile_hits.len()
    );
    // A corrupt over-budget collection is refused by the decoder before any candidate publication.
    let mut w = Writer::new();
    w.bool(true);
    w.u64(12001);
    w.u64(0);
    w.u64(0);
    for _ in 0..MAX_TRAFFIC_ACTORS {
        w.bool(false);
    }
    w.u16(129);
    assert!(State::decode(&mut Reader::new(&w.bytes)).is_err());
}
