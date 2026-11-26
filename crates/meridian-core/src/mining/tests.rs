//! Pure geometry/source/ledger components, never paid gameplay or live World injection.
use super::*;
fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn some<T>(v: Option<T>) -> T {
    v.unwrap_or_else(|| panic!("missing native source"))
}
fn component_stroke(state: &State, ordinal: u8, sequence: u64, visit: u64) -> MiningStroke {
    let body = some(state.body(usize::from(ordinal)));
    let pose = RigidPose {
        position: add(body.position, Vec3::new(0.0, 0.0, -400.0)),
        orientation: crate::docking::Basis3::IDENTITY,
    };
    let contact = some(state.beam(pose.position, pose.orientation.z, MINING_RANGE));
    assert_eq!(contact.asteroid, body.id);
    let tick = sequence * 18 + 1;
    MiningStroke {
        owner: crate::CombatIdentity {
            owner: 99,
            hull: 100,
            system: SystemId::AVENOR,
            visit,
        },
        item: 100110,
        account_revision: 1,
        world_tick: tick,
        physical_tick: tick,
        emission: sequence,
        pose,
        speed: 0.0,
        contact,
        peers: crate::traffic::State::default().missile_scene(
            SystemId::AVENOR,
            tick,
            crate::traffic::SceneStage::Command(sequence),
        ),
        missiles: Arc::new(Vec::new()),
    }
}
#[test]
fn frozen_original_chunk_quantities_materials_and_damage_thresholds_are_independent_literals() {
    let state = State::default();
    use CommodityId::{Alloys, Gold, Minerals};
    let materials = [
        Alloys, Minerals, Minerals, Minerals, Minerals, Minerals, Minerals, Gold, Minerals,
        Minerals, Alloys, Minerals,
    ];
    let strokes = [2, 4, 5, 5, 6, 6, 7, 7, 7, 8, 8, 2];
    for (i, body) in state.bodies().into_iter().enumerate() {
        assert_eq!(
            body.initial_units,
            [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 1][i]
        );
        assert_eq!(body.commodity, materials[i]);
        assert_eq!(body.id.original_field(), 4 + (i / 4) as u16);
        assert_eq!(body.id.original_rock(), (i % 4) as u16);
        assert_eq!((body.energy / 50.0).ceil() as u8, strokes[i]);
        assert!(body.contains_point(body.position));
        assert_eq!(body.remaining_energy, body.energy);
    }
}
#[test]
fn partial_work_then_terminal_native_hit_releases_unique_pending_ore_and_no_cargo_counter() {
    let mut state = State::default();
    let original = some(state.body(0));
    let hit = component_stroke(&state, 0, 1, 2);
    let (partial, released) = ok(state.prepare_work(hit));
    assert_eq!(released, 0);
    assert_eq!(some(partial.body(0)).remaining_energy, 50.0);
    assert!(partial.pending_ore(19).is_empty());
    state = partial;
    let hit = component_stroke(&state, 0, 2, 2);
    let (depleted, released) = ok(state.prepare_work(hit));
    assert_eq!(released, 1);
    assert_eq!(some(depleted.body(0)).remaining_energy, 0.0);
    let ore = depleted.pending_ore(37);
    assert_eq!(ore.len(), 1);
    assert_eq!(ore[0].id.asteroid(), original.id);
    assert_eq!(ore[0].id.fragment(), 0);
    assert_eq!(
        (ore[0].commodity, ore[0].quantity),
        (CommodityId::Alloys, 1)
    );
    assert_eq!(
        ore[0].position,
        add(original.position, Vec3::new(0.0, 0.0, 10.0))
    );
    assert_eq!(
        depleted.pending_ore(97)[0].position,
        add(ore[0].position, Vec3::new(0.0, 2.0, 3.0))
    );
    let mut bytes = Writer::new();
    depleted.encode(&mut bytes);
    let mut r = Reader::new(&bytes.bytes);
    assert_eq!(ok(State::decode(&mut r)), depleted);
    assert!(r.finished());
    assert_eq!(depleted.work_count(), 2);
    ok(depleted.validate());
}

#[test]
fn moving_authored_shard_has_outward_faces_and_once_only_lower_forward_intake() {
    let mut state = State::default();
    for sequence in 1..=2 {
        state = ok(state.prepare_work(component_stroke(&state, 0, sequence, 2))).0;
    }
    let ore = state.pending_ore(97)[0];
    let vertices = ore.vertices();
    let offset = |p: Vec3| {
        Vec3::new(
            p.x - ore.position.x,
            p.y - ore.position.y,
            p.z - ore.position.z,
        )
    };
    for face in ASTEROID_FACES {
        let [a, b, c] = face.map(|i| offset(vertices[i]));
        let ab = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
        let ac = Vec3::new(c.x - a.x, c.y - a.y, c.z - a.z);
        let cross = Vec3::new(
            ab.y * ac.z - ab.z * ac.y,
            ab.z * ac.x - ab.x * ac.z,
            ab.x * ac.y - ab.y * ac.x,
        );
        assert!(
            cross.dot(a) > 0.0,
            "authored shard has inward or degenerate face"
        );
    }
    let pose = RigidPose {
        position: add(ore.position, Vec3::new(0.0, 45.0, -55.0)),
        orientation: crate::docking::Basis3::IDENTITY,
    };
    assert!(ore.scoop_admissible(pose, 0.0, true));
    assert!(!ore.scoop_admissible(pose, 2.0, true));
    assert!(ore.scoop_admissible(pose, 2.0, false));
    let above = RigidPose {
        position: add(ore.position, Vec3::new(0.0, -45.0, -55.0)),
        ..pose
    };
    assert!(!ore.scoop_admissible(above, 0.0, false));
    let receipt = Collection {
        id: ore.id,
        owner: state.strokes()[1].owner,
        item: 123,
        revision: 3,
        world_tick: 97,
        physical_tick: 97,
        pose,
        speed: 0.0,
        full_aperture: true,
    };
    let collected = ok(state.prepare_collection(receipt));
    assert!(collected.pending_ore(98).is_empty());
    assert!(collected.prepare_collection(receipt).is_err());
    assert_eq!(
        collected.cargo_intakes().collect::<Vec<_>>(),
        vec![crate::station_services::CargoIntake {
            revision: 3,
            commodity: ore.commodity
        }]
    );
    let mut base = Writer::new();
    collected.encode(&mut base);
    let mut trailer = Writer::new();
    collected.encode_collections(&mut trailer);
    let mut restored = ok(State::decode_base(&mut Reader::new(&base.bytes)));
    ok(restored.decode_collections(&mut Reader::new(&trailer.bytes)));
    assert_eq!(restored, collected);
}
#[test]
fn exact_range_closed_native_mesh_and_legacy_absence_never_make_a_mining_capability() {
    let state = State::default();
    let body = some(state.body(0));
    let point = body.vertices[5];
    let origin = add(point, Vec3::new(0.0, 0.0, -MINING_RANGE));
    let contact = some(state.beam(origin, Vec3::new(0.0, 0.0, 1.0), MINING_RANGE));
    assert_eq!(contact.distance, MINING_RANGE);
    assert_eq!(contact.asteroid, body.id);
    assert_eq!(contact.point, point);
    let direction = Vec3::new(0.0, 0.0, 1.0);
    assert!(
        state
            .beam(origin, direction, MINING_RANGE.next_down())
            .is_none()
    );
    // This shift is smaller than one ULP at 6000. The rounded distance by itself
    // cannot distinguish it from the exact endpoint.
    let beyond = Vec3::new(origin.x, origin.y, origin.z.next_down());
    assert!(state.beam(beyond, direction, MINING_RANGE).is_none());
    assert!(
        state
            .beam(origin, Vec3::new(0.0, 0.0, 1.0), MINING_RANGE - 0.001)
            .is_none()
    );
    let absent = State::legacy_absent();
    assert!(!absent.enabled());
    assert!(absent.pending_ore(0).is_empty());
    assert!(
        absent
            .beam(origin, Vec3::new(0.0, 0.0, 1.0), MINING_RANGE)
            .is_none()
    );
    assert!(AsteroidId::from_ordinal(12).is_err());
}
#[test]
fn storage_refusal_precedes_terminal_work_and_actual_departure_does_not_replenish_partial_or_spent_sources()
 {
    let mut state = State::default();
    let mut sequence = 0;
    for ordinal in 0..10 {
        while some(state.body(ordinal)).remaining_energy > 0.0 {
            sequence += 1;
            let hit = component_stroke(&state, ordinal as u8, sequence, 2);
            state = ok(state.prepare_work(hit)).0;
        }
    }
    assert_eq!(state.pending_count(), 226);
    for _ in 0..7 {
        sequence += 1;
        let hit = component_stroke(&state, 10, sequence, 2);
        state = ok(state.prepare_work(hit)).0;
    }
    let before = state.clone();
    let refused = component_stroke(&state, 10, sequence + 1, 2);
    assert_eq!(state.prepare_work(refused), Err(MiningError::ProofCapacity));
    assert_eq!(state, before);
    let owner = crate::CombatIdentity {
        owner: 99,
        hull: 100,
        system: SystemId::AVENOR,
        visit: 2,
    };
    let departed = state.prepare_departure(owner, SystemId::RENAS, (sequence + 1) * 18);
    assert_eq!(departed.pending_count(), 0);
    assert_eq!(some(departed.body(0)).remaining_energy, 0.0);
    assert_eq!(
        some(departed.body(10)).remaining_energy,
        some(state.body(10)).remaining_energy
    );
    let hit = component_stroke(&departed, 10, sequence + 2, 4);
    let (worked, released) = ok(departed.prepare_work(hit));
    assert_eq!(released, 50);
    assert_eq!(worked.pending_count(), 50);
    ok(worked.validate());
}
#[test]
fn retained_prefix_first_contact_cannot_change_a_body_contact_clock_or_duplicate_terminal_work() {
    let mut state = State::default();
    let mut hit = component_stroke(&state, 0, 1, 2);
    hit.contact.point.x += 1.0;
    assert_eq!(state.prepare_work(hit), Err(MiningError::InvalidState));
    let hit = component_stroke(&state, 0, 1, 2);
    state = ok(state.prepare_work(hit)).0;
    let hit = component_stroke(&state, 0, 2, 2);
    state = ok(state.prepare_work(hit)).0;
    let mut encoded = Writer::new();
    state.encode(&mut encoded);
    let mut truncated = Reader::new(&encoded.bytes[..encoded.bytes.len() - 1]);
    assert!(State::decode(&mut truncated).is_err());
    let mut bad = state.clone();
    Arc::make_mut(&mut bad.strokes)[1].emission = 1;
    assert!(bad.validate().is_err());
    let mut bad = state.clone();
    Arc::make_mut(&mut bad.strokes).push(state.strokes[1].clone());
    assert!(bad.validate().is_err());
}

#[test]
fn native_actor_envelope_and_step_prefix_keep_partial_rocks_solid_until_terminal_command() {
    // Pure scene boundary: no live World, pilot, collision or paid acceptance is injected.
    let mut state = State::default();
    let body = some(state.body(0));
    let a = add(body.position, Vec3::new(0.0, 0.0, -400.0));
    let b = add(body.position, Vec3::new(0.0, 0.0, 400.0));
    assert!(!state.segment_clear(a, b, 0.0));
    assert!(!state.segment_clear(a, b, 40.0));
    assert!(state.segment_clear(
        add(a, Vec3::new(200.0, 0.0, 0.0)),
        add(b, Vec3::new(200.0, 0.0, 0.0)),
        40.0
    ));
    for sequence in 1..=2 {
        let hit = component_stroke(&state, 0, sequence, 2);
        state = ok(state.prepare_work(hit)).0;
    }
    assert!(!state.at_step(37).segment_clear(a, b, 0.0));
    assert!(state.at_step(38).segment_clear(a, b, 40.0));
    assert!(state.segment_clear(a, b, 40.0));
    assert!(State::legacy_absent().segment_clear(a, b, 40.0));
}
