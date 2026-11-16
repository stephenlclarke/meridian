//! Geometry and finite-ledger component tests, not paid-tool gameplay acceptance.
use super::*;

fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("unexpected refusal: {error:?}"),
    }
}
fn some<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => panic!("missing authored body/contact"),
    }
}

#[test]
fn one_closed_mesh_supplies_stable_identity_geometry_and_actual_front_surface_contact() {
    let state = State::default();
    let bodies = state.bodies();
    assert_eq!(bodies.len(), 12);
    assert_eq!(
        some(belt_destination(SystemId::AVENOR)).position,
        BELT_CENTRE
    );
    assert!(belt_destination(SystemId::RENAS).is_none());
    for (i, body) in bodies.into_iter().enumerate() {
        assert_eq!(usize::from(body.id.ordinal()), i);
        assert_eq!(body.id.system(), SystemId::AVENOR);
        assert_eq!(body.remaining_units, body.initial_units);
        assert_eq!(body.extracted_units, 0);
        assert!(body.contains_point(body.position));
        for face in ASTEROID_FACES {
            let [a, b, c] = face.map(|index| body.vertices[index]);
            let edge = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
            let other = Vec3::new(c.x - a.x, c.y - a.y, c.z - a.z);
            let normal = Vec3::new(
                edge.y * other.z - edge.z * other.y,
                edge.z * other.x - edge.x * other.z,
                edge.x * other.y - edge.y * other.x,
            );
            assert!(
                normal.dot(Vec3::new(
                    a.x - body.position.x,
                    a.y - body.position.y,
                    a.z - body.position.z
                )) > 0.0
            );
        }
    }
    let contact = some(state.beam(
        Vec3::new(4000.0, 0.0, 5000.0),
        Vec3::new(0.0, 0.0, 1.0),
        1000.0,
    ));
    assert_eq!(contact.asteroid, bodies[0].id);
    assert_eq!(contact.distance, 928.0);
    assert_eq!(contact.point.z, 5928.0);
    assert!(
        state
            .beam(
                Vec3::new(4000.0, 0.0, 5000.0),
                Vec3::new(0.0, 0.0, 1.0),
                927.9
            )
            .is_none()
    );
    assert!(
        state
            .beam(
                Vec3::new(0.0, 0.0, 1200.0),
                Vec3::new(0.0, 0.0, 1.0),
                6000.0
            )
            .is_none()
    );
}

#[test]
fn every_native_unit_is_depleted_once_and_exhausted_body_keeps_identical_collision_geometry() {
    let mut state = State::default();
    let original = state.bodies()[0];
    for count in 1..=original.initial_units {
        let (candidate, commodity) = ok(state.prepare_extract(original.id));
        state = candidate;
        assert_eq!(commodity, CommodityId::Minerals);
        let body = state.bodies()[0];
        assert_eq!(body.extracted_units, count);
        assert_eq!(
            u16::from(body.remaining_units) + u16::from(body.extracted_units),
            u16::from(original.initial_units)
        );
        assert_eq!(body.vertices, original.vertices);
        assert_eq!(body.position, original.position);
        let mut w = Writer::new();
        state.encode(&mut w);
        let restored = ok(State::decode(&mut Reader::new(&w.bytes)));
        assert_eq!(restored, state);
        assert_eq!(restored.work_count(), u64::from(count));
    }
    let before = state;
    assert_eq!(
        state.prepare_extract(original.id),
        Err(MiningError::Depleted)
    );
    assert_eq!(state, before);
    assert!(
        state
            .beam(
                Vec3::new(4000.0, 0.0, 5000.0),
                Vec3::new(0.0, 0.0, 1.0),
                1000.0
            )
            .is_some()
    );
}

#[test]
fn malformed_and_legacy_absent_states_cannot_regenerate_or_describe_a_work_target() {
    let mut state = State::default();
    state.remaining[0] += 1;
    assert!(state.validate().is_err());
    let mut state = State::default();
    state.extracted[0] = 1;
    assert!(state.validate().is_err());
    assert!(AsteroidId::from_ordinal(12).is_err());
    let legacy = State::legacy_absent();
    ok(legacy.validate());
    assert!(!legacy.enabled());
    assert_eq!(legacy.work_count(), 0);
    assert!(
        legacy
            .beam(
                Vec3::new(4000.0, 0.0, 5000.0),
                Vec3::new(0.0, 0.0, 1.0),
                1000.0
            )
            .is_none()
    );
    assert_eq!(
        legacy.prepare_extract(State::default().bodies()[0].id),
        Err(MiningError::NoTarget)
    );
    let mut corrupted = legacy;
    corrupted.remaining[0] = 1;
    assert!(corrupted.validate().is_err());
}
