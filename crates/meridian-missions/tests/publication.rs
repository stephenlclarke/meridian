//! Host publication preservation. All gameplay authority comes from genuine World reviews.
mod support;
use meridian_core::save::MAX_SAVE_BYTES;
use meridian_core::{
    FlightInput, FlightReceptionAction, FlightReceptionOutcome, FlightView, World,
};
use meridian_missions::{
    Error, Event, Instance, MAX_FLIGHT_PUBLICATION_BYTES, Pack, Status, Value,
    prepare_flight_acceptance, prepare_flight_transition, restore_flight_publication,
    serialize_flight_publication,
};
use sha2::{Digest, Sha256};
use support::Required;

fn accepted(seed: u64, pack: &Pack) -> (World, Instance) {
    let mut world = World::new_commander(seed).required();
    let mut instance = Instance::new(pack);
    first_callback(&mut world, &mut instance, pack);
    (world, instance)
}
fn first_callback(world: &mut World, instance: &mut Instance, pack: &Pack) {
    let tick = world.snapshot().tick;
    let review = world
        .review_flight_reception(FlightReceptionAction::Accept, tick)
        .required();
    prepare_flight_acceptance(world, instance, pack, review, tick)
        .required()
        .publish(|_| Ok(()))
        .required();
}
fn native_confirmation(world: &mut World, action: FlightReceptionAction) {
    let tick = world.snapshot().tick;
    let review = world.review_flight_reception(action, tick).required();
    world
        .prepare_flight_reception(review, tick)
        .required()
        .commit();
}
fn wrap(native: &[u8], script: &[u8]) -> Vec<u8> {
    let mut bytes = b"MMIS0001".to_vec();
    bytes.extend(u32::try_from(native.len()).required().to_le_bytes());
    bytes.extend(u32::try_from(script.len()).required().to_le_bytes());
    bytes.extend_from_slice(native);
    bytes.extend_from_slice(script);
    reseal(&mut bytes, false);
    bytes
}
fn reseal(bytes: &mut Vec<u8>, replace: bool) {
    if replace {
        bytes.truncate(bytes.len() - 32);
    }
    let checksum = Sha256::digest(bytes.as_slice());
    bytes.extend_from_slice(&checksum);
}

#[test]
fn ordinary_publication_keeps_exact_native_bytes_and_suspended_pending_script_state() {
    let pack = Pack::bundled_flight_reception().required();
    let (mut world, instance) = accepted(801, &pack);
    let instance = instance
        .with_event(Event {
            sequence: 2,
            kind: "unsupported.lifecycle".into(),
            value: Value::map([("retained", Value::Integer(41))]),
        })
        .required()
        .suspended("Retained fault; publication must not resume or dispatch.")
        .required();
    let script_bytes = instance.to_bytes().required();
    world.step(FlightInput::default()).required();
    let tick = world.snapshot().tick;
    world.request_flight_view(FlightView::Left, tick).required();
    world.request_pause(true, tick).required();
    let native = world.save_bytes().required();
    let bytes = serialize_flight_publication(&native, &instance).required();
    assert_eq!(&bytes[16..16 + native.len()], native.as_slice());
    assert_eq!(
        &bytes[16 + native.len()..bytes.len() - 32],
        script_bytes.as_slice()
    );
    let (restored, retained) = restore_flight_publication(&bytes, Some(&pack)).required();
    assert_eq!(restored.save_bytes().required(), native);
    assert_eq!(restored.flight_view(), FlightView::Left);
    assert!(restored.is_paused());
    assert_eq!(retained.to_bytes().required(), script_bytes);
    assert!(matches!(retained.status(), Status::Suspended(_)));
    assert_eq!(retained.pending().len(), 1);
    assert_eq!(retained.acknowledged_through(), 1);
    assert_eq!(retained.journal().len(), 1);
    assert_eq!(world.save_bytes().required(), native);
    assert_eq!(instance.to_bytes().required(), script_bytes);
}

#[test]
fn later_native_confirmations_preserve_origin_binding_and_never_replay_the_callback() {
    let pack = Pack::bundled_flight_reception().required();
    let (mut world, mut instance) = accepted(802, &pack);
    instance = instance
        .with_event(Event {
            sequence: 2,
            kind: "unsupported.lifecycle".into(),
            value: Value::empty_map(),
        })
        .required()
        .suspended("Later native actions must preserve this fault and pending event.")
        .required();
    let script_bytes = instance.to_bytes().required();
    let before = world.save_bytes().required();
    let review = world
        .review_flight_reception(FlightReceptionAction::Cancel, 0)
        .required();
    {
        let preparation =
            prepare_flight_transition(&mut world, &mut instance, review, 0).required();
        let (prospective, retained) =
            restore_flight_publication(preparation.bytes(), Some(&pack)).required();
        assert_eq!(retained.to_bytes().required(), script_bytes);
        assert_eq!(
            prospective.flight_reception_snapshot().required().attempts[0].outcome,
            Some(FlightReceptionOutcome::Abandoned)
        );
    }
    assert_eq!(world.save_bytes().required(), before);
    assert_eq!(instance.to_bytes().required(), script_bytes);
    let failure = prepare_flight_transition(&mut world, &mut instance, review, 0)
        .required()
        .publish(|_| Err(Error::Publish("later native write failed".into())));
    assert!(matches!(failure, Err(Error::Publish(_))));
    assert_eq!(world.save_bytes().required(), before);
    assert_eq!(instance.to_bytes().required(), script_bytes);
    let cash = world.credits_tenths();
    for action in [
        FlightReceptionAction::Cancel,
        FlightReceptionAction::Accept,
        FlightReceptionAction::Cancel,
    ] {
        let review = world.review_flight_reception(action, 0).required();
        let mut publication = Vec::new();
        prepare_flight_transition(&mut world, &mut instance, review, 0)
            .required()
            .publish(|bytes| {
                publication = bytes.to_vec();
                Ok(())
            })
            .required();
        let (cold, retained) = restore_flight_publication(&publication, Some(&pack)).required();
        assert_eq!(cold.save_bytes().required(), world.save_bytes().required());
        assert_eq!(retained.to_bytes().required(), script_bytes);
        assert_eq!(instance.to_bytes().required(), script_bytes);
        assert_eq!(instance.journal().len(), 1);
        assert_eq!(instance.revision(), 1);
        assert_eq!(instance.pending().len(), 1);
        assert_eq!(instance.acknowledged_through(), 1);
        assert!(matches!(instance.status(), Status::Suspended(_)));
        assert_eq!(world.credits_tenths(), cash);
    }
    let snapshot = world.flight_reception_snapshot().required();
    assert_eq!(snapshot.attempts.len(), 2);
    assert!(snapshot.active.is_none());
    assert!(snapshot.trace.is_none());
    assert!(prepare_flight_transition(&mut world, &mut instance, review, 0).is_err());
    assert_eq!(instance.to_bytes().required(), script_bytes);
}

#[test]
fn foreign_stale_paused_and_unbound_native_preparations_refuse_without_publication() {
    let pack = Pack::bundled_flight_reception().required();
    let (mut world, mut instance) = accepted(803, &pack);
    let (foreign, _) = accepted(804, &pack);
    let foreign_review = foreign
        .review_flight_reception(FlightReceptionAction::Cancel, 0)
        .required();
    let before = world.save_bytes().required();
    let script_bytes = instance.to_bytes().required();
    assert!(prepare_flight_transition(&mut world, &mut instance, foreign_review, 0).is_err());
    assert_eq!(world.save_bytes().required(), before);
    assert!(
        world
            .review_flight_reception(FlightReceptionAction::Receive, 0)
            .is_err()
    );
    let review = world
        .review_flight_reception(FlightReceptionAction::Cancel, 0)
        .required();
    assert!(prepare_flight_transition(&mut world, &mut instance, review, 1).is_err());
    world.request_pause(true, 0).required();
    let paused = world.save_bytes().required();
    assert!(prepare_flight_transition(&mut world, &mut instance, review, 0).is_err());
    assert_eq!(world.save_bytes().required(), paused);
    world.request_pause(false, 0).required();
    let mut unbound = Instance::new(&pack);
    assert!(prepare_flight_transition(&mut world, &mut unbound, review, 0).is_err());
    assert_eq!(world.save_bytes().required(), before);
    assert_eq!(instance.to_bytes().required(), script_bytes);
    assert_eq!(unbound, Instance::new(&pack));
}

#[test]
fn serializer_and_restore_share_identity_and_retained_attempt_refusals() {
    let pack = Pack::bundled_flight_reception().required();
    let (world, instance) = accepted(805, &pack);
    let native = world.save_bytes().required();
    let fresh = World::new_commander(805).required().save_bytes().required();
    let (foreign, _) = accepted(806, &pack);
    for refused in [fresh, foreign.save_bytes().required()] {
        assert!(matches!(
            serialize_flight_publication(&refused, &instance),
            Err(Error::Identity)
        ));
        assert!(matches!(
            restore_flight_publication(
                &wrap(&refused, &instance.to_bytes().required()),
                Some(&pack)
            ),
            Err(Error::Identity)
        ));
    }
    assert!(matches!(
        serialize_flight_publication(&native, &Instance::new(&pack)),
        Err(Error::Identity)
    ));
    // Deliberately corrupt semantic fields in otherwise valid/checksummed MLST0001 fixtures.
    let original = instance.to_bytes().required();
    let mut cursor = 8;
    let mut fields = Vec::new();
    for _ in 0..3 {
        let length =
            u32::from_le_bytes(original[cursor..cursor + 4].try_into().required()) as usize;
        fields.push(cursor + 4);
        cursor += 4 + length;
    }
    assert_eq!(original[cursor], 1); // Native binding follows the three identity fields.
    for offset in [fields[0], fields[1], fields[2], cursor + 1, cursor + 9] {
        let mut altered = original.clone();
        altered[offset] = if offset == fields[2] {
            b'0'
        } else {
            altered[offset] ^ 1
        };
        reseal(&mut altered, true);
        let invalid = Instance::from_bytes(&altered).required();
        assert!(matches!(
            serialize_flight_publication(&native, &invalid),
            Err(Error::Identity)
        ));
        assert!(matches!(
            restore_flight_publication(&wrap(&native, &altered), Some(&pack)),
            Err(Error::Identity)
        ));
    }
    // An instance from the second actual acceptance cannot join an older same-owner save.
    let mut newer = World::from_save_bytes(&native).required();
    native_confirmation(&mut newer, FlightReceptionAction::Cancel);
    let mut second = Instance::new(&pack);
    first_callback(&mut newer, &mut second, &pack);
    assert!(matches!(
        serialize_flight_publication(&native, &second),
        Err(Error::Identity)
    ));
    // Conversely, the original binding remains valid in retained history during a later attempt.
    assert!(serialize_flight_publication(&newer.save_bytes().required(), &instance).is_ok());
    assert_eq!(world.save_bytes().required(), native);
    assert_eq!(instance.to_bytes().required(), original);
}

#[test]
fn publication_enforces_outer_native_and_script_caps_before_payload_decoding() {
    let pack = Pack::bundled_flight_reception().required();
    let (world, instance) = accepted(807, &pack);
    assert_eq!(MAX_FLIGHT_PUBLICATION_BYTES, 33_620_048);
    assert!(matches!(
        serialize_flight_publication(&vec![0; MAX_SAVE_BYTES + 1], &instance),
        Err(Error::Limit("native publication bytes"))
    ));
    assert!(matches!(
        serialize_flight_publication(b"invalid native bytes", &instance),
        Err(Error::World(_))
    ));
    let mut outer = vec![0; MAX_FLIGHT_PUBLICATION_BYTES + 1];
    outer[..8].copy_from_slice(b"MMIS0001");
    assert!(matches!(
        restore_flight_publication(&outer, Some(&pack)),
        Err(Error::Limit("mission publication bytes"))
    ));
    outer.truncate(MAX_FLIGHT_PUBLICATION_BYTES);
    assert!(matches!(
        restore_flight_publication(&outer, Some(&pack)),
        Err(Error::Codec("mission publication checksum"))
    ));
    drop(outer);
    let script = instance.to_bytes().required();
    assert!(matches!(
        restore_flight_publication(&wrap(&vec![0; MAX_SAVE_BYTES + 1], &script), Some(&pack)),
        Err(Error::Limit("mission publication payload bytes"))
    ));
    let max_script = MAX_FLIGHT_PUBLICATION_BYTES - MAX_SAVE_BYTES - 48;
    let oversized_script = wrap(&world.save_bytes().required(), &vec![0; max_script + 1]);
    assert!(oversized_script.len() < MAX_FLIGHT_PUBLICATION_BYTES);
    assert!(matches!(
        restore_flight_publication(&oversized_script, Some(&pack)),
        Err(Error::Limit("mission publication payload bytes"))
    ));
    let mut malformed_lengths =
        serialize_flight_publication(&world.save_bytes().required(), &instance).required();
    malformed_lengths[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    reseal(&mut malformed_lengths, true);
    assert!(matches!(
        restore_flight_publication(&malformed_lengths, Some(&pack)),
        Err(Error::Codec("mission publication lengths"))
    ));
}
