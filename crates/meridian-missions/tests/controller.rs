//! Author tests. Native/compiler execution belongs to the existing owner lane.
mod support;
use meridian_core::{FlightReceptionAction, World};
use meridian_missions::{
    Error, Event, Instance, Pack, Status, Value, prepare_flight_acceptance,
    restore_flight_publication,
};
use std::collections::BTreeMap;
use support::Required;

fn original_t01() -> &'static meridian_mission_content::Pack<'static> {
    meridian_mission_content::BUNDLED_PACKS
        .iter()
        .find(|p| p.definition == "l337.training/flight-reception" && p.version == "1.0.0")
        .required()
}
fn t01() -> Pack {
    Pack::bundled_flight_reception().required()
}
fn test_pack(source: &[u8]) -> Pack {
    Pack::load(
        br#"schemaVersion=2
missionAPI=2
id="tests.controller"
version="1.0.0"
minimumEngine="2.0.0"
priority=1
source="Authored hostile/controller test fixture; no native authority"
license="MIT"
environments=["space"]
capabilities=["script-lua-v1","story-v2"]
module="scripts/main.lua"
[mission]
id="explicit"
title="Controller test"
briefing="Explicit test data only"
destination="None"
reward="None"
milestones=["Test state boundary"]
[[handlers]]
event="mission.accepted"
function="run"
"#,
        &BTreeMap::from([("scripts/main.lua".into(), source.to_vec())]),
    )
    .required()
}
fn event(sequence: u64) -> Event {
    Event {
        sequence,
        kind: "mission.accepted".into(),
        value: Value::empty_map(),
    }
}

#[test]
fn genuine_t01_publication_failure_drop_success_and_cold_roundtrip_use_actual_world() {
    let pack = t01();
    assert_eq!(pack.identity().definition, "l337.training/flight-reception");
    assert_eq!(
        pack.identity().digest,
        "bc01834e04f87816c8021b76fabe3d9b8f3af394ff8fcd420c4b7c3acf1350d4"
    );
    let mut world = World::new_commander(501).required();
    let mut instance = Instance::new(&pack);
    let before_world = world.save_bytes().required();
    let before_script = instance.to_bytes().required();
    let credits = world.credits_tenths();
    let review = world
        .review_flight_reception(FlightReceptionAction::Accept, world.snapshot().tick)
        .required();
    {
        let preparation =
            prepare_flight_acceptance(&mut world, &mut instance, &pack, review, 0).required();
        assert_eq!(preparation.candidate_script().journal().len(), 1);
        assert!(
            preparation.candidate_script().journal()[0].contains("current commander, owned fit")
        );
    }
    assert_eq!(world.save_bytes().required(), before_world);
    assert_eq!(instance.to_bytes().required(), before_script);
    let failure = prepare_flight_acceptance(&mut world, &mut instance, &pack, review, 0)
        .required()
        .publish(|_| Err(Error::Publish("injected owned storage refusal".into())));
    assert!(matches!(failure, Err(Error::Publish(_))));
    assert_eq!(world.save_bytes().required(), before_world);
    assert_eq!(instance.to_bytes().required(), before_script);
    let mut published = Vec::new();
    prepare_flight_acceptance(&mut world, &mut instance, &pack, review, 0)
        .required()
        .publish(|bytes| {
            published = bytes.to_vec();
            Ok(())
        })
        .required();
    assert_eq!(world.credits_tenths(), credits);
    assert!(
        world
            .flight_reception_snapshot()
            .required()
            .active
            .is_some()
    );
    assert_eq!(instance.acknowledged_through(), 1);
    assert_eq!(instance.revision(), 1);
    assert!(instance.pending().is_empty());
    assert_eq!(instance.data(), &Value::empty_map());
    let (cold_world, cold_script) = restore_flight_publication(&published, Some(&pack)).required();
    assert_eq!(
        cold_world.save_bytes().required(),
        world.save_bytes().required()
    );
    assert_eq!(cold_script, instance);
    assert_eq!(cold_world.credits_tenths(), credits);
    let (_, missing) = restore_flight_publication(&published, None).required();
    assert!(matches!(missing.status(), Status::Suspended(_)));
    assert_eq!(missing.resumed(&pack).required(), instance);
    let mut corrupt = published.clone();
    corrupt[20] ^= 1;
    assert!(restore_flight_publication(&corrupt, Some(&pack)).is_err());
    assert!(prepare_flight_acceptance(&mut world, &mut instance, &pack, review, 0).is_err());
    assert_eq!(world.credits_tenths(), credits);
}

#[test]
fn actual_foreign_review_pause_and_changed_content_refuse_without_either_mutation() {
    let pack = t01();
    let foreign = World::new_commander(702).required();
    let review = foreign
        .review_flight_reception(FlightReceptionAction::Accept, 0)
        .required();
    let mut world = World::new_commander(701).required();
    let mut script = Instance::new(&pack);
    let before = world.save_bytes().required();
    let explicit = script.to_bytes().required();
    assert!(prepare_flight_acceptance(&mut world, &mut script, &pack, review, 0).is_err());
    assert_eq!(world.save_bytes().required(), before);
    assert_eq!(script.to_bytes().required(), explicit);
    let review = world
        .review_flight_reception(FlightReceptionAction::Accept, 0)
        .required();
    world.request_pause(true, 0).required();
    let paused = world.save_bytes().required();
    assert!(prepare_flight_acceptance(&mut world, &mut script, &pack, review, 0).is_err());
    assert_eq!(world.save_bytes().required(), paused);
    world.request_pause(false, 0).required();
    let mut source = original_t01().entry_bytes().required().to_vec();
    source.extend(b"\n-- different content, same descriptive identity\n");
    let different = Pack::load(
        original_t01().manifest_bytes().required(),
        &BTreeMap::from([("scripts/main.lua".into(), source)]),
    )
    .required();
    let before = world.save_bytes().required();
    assert!(prepare_flight_acceptance(&mut world, &mut script, &different, review, 0).is_err());
    assert_eq!(world.save_bytes().required(), before);
    assert_eq!(script.to_bytes().required(), explicit);
}

#[test]
fn explicit_state_only_survives_each_fresh_vm_and_replay_cannot_repeat_journal() {
    let pack = test_pack(b"hidden=(hidden or 0)+1; return {run=function(e,w,d) d.n=(d.n or 0)+1; d.hidden=hidden; return {data=d,commands={{op='journal.append',text='once per actual event'}}} end}");
    let mut instance = Instance::new(&pack).with_event(event(1)).required();
    instance
        .prepare(&pack, 1, &Value::empty_map())
        .required()
        .commit_storage(&mut instance)
        .required();
    assert_eq!(instance.data().get("n"), Some(&Value::Integer(1)));
    assert_eq!(instance.data().get("hidden"), Some(&Value::Integer(1)));
    assert!(instance.prepare(&pack, 1, &Value::empty_map()).is_err());
    instance = Instance::from_bytes(&instance.to_bytes().required())
        .required()
        .with_event(event(2))
        .required();
    instance
        .prepare(&pack, 2, &Value::empty_map())
        .required()
        .commit_storage(&mut instance)
        .required();
    assert_eq!(instance.data().get("n"), Some(&Value::Integer(2)));
    assert_eq!(instance.data().get("hidden"), Some(&Value::Integer(1)));
    assert_eq!(instance.journal().len(), 2);
    assert!(instance.with_event(event(2)).is_err());
}

#[test]
fn unsupported_credit_command_after_journal_proposal_rolls_back_all_script_effects() {
    let pack = test_pack(b"return {run=function(e,w,d) d.changed=true; return {data=d,commands={{op='journal.append',text='must not publish'},{op='credits',amount=999999}}} end}");
    let instance = Instance::new(&pack).with_event(event(1)).required();
    let before = instance.to_bytes().required();
    assert!(matches!(
        instance.prepare(&pack, 1, &Value::empty_map()),
        Err(Error::UnsupportedOperation(_))
    ));
    assert_eq!(instance.to_bytes().required(), before);
    assert_eq!(instance.acknowledged_through(), 0);
    assert!(instance.journal().is_empty());
    let suspension = instance
        .prepare_dispatch(Some(&pack), 1, &Value::empty_map())
        .required();
    assert!(matches!(
        suspension.candidate().status(),
        Status::Suspended(_)
    ));
    assert_eq!(suspension.candidate().data(), instance.data());
    assert_eq!(suspension.candidate().pending(), instance.pending());
    assert_eq!(suspension.candidate().acknowledged_through(), 0);
    assert!(suspension.candidate().journal().is_empty());
}

#[test]
fn closed_declared_modules_cache_within_vm_reset_between_callbacks_and_bind_raw_digest() {
    let mut manifest = String::from_utf8(original_t01().manifest_bytes().required().to_vec())
        .required()
        .replace("\"story-v2\"", "\"story-v2\", \"pack-modules-v2\"");
    manifest.push_str("\n[[modules]]\nid='Shared'\npath='scripts/shared.lua'\n");
    let resources = BTreeMap::from([
        ("scripts/main.lua".into(), b"local a=require('Shared');local b=require('Shared');return {accepted=function(e,w,d) return {data={same=a==b,n=a.n},commands={}} end}".to_vec()),
        ("scripts/shared.lua".into(), b"created=(created or 0)+1;return {n=created}".to_vec()),
    ]);
    let pack = Pack::load(manifest.as_bytes(), &resources).required();
    for _ in 0..2 {
        let result = meridian_missions::evaluate(
            pack.source(),
            "accepted",
            &Value::empty_map(),
            &Value::empty_map(),
            &Value::empty_map(),
            meridian_missions::Limits::default(),
        )
        .required();
        assert_eq!(result.data.get("same"), Some(&Value::Bool(true)));
        assert_eq!(result.data.get("n"), Some(&Value::Integer(1)));
    }
    let mut changed = resources.clone();
    changed
        .get_mut("scripts/shared.lua")
        .required()
        .extend(b"\n-- different raw resource\n");
    assert_ne!(
        Pack::load(manifest.as_bytes(), &changed)
            .required()
            .identity()
            .digest,
        pack.identity().digest
    );
    let undeclared = manifest.replace("id='Shared'", "id='Other'");
    assert!(Pack::load(undeclared.as_bytes(), &resources).is_err());
    let invalid = manifest.replace("scripts/shared.lua", "../shared.lua");
    assert!(Pack::load(invalid.as_bytes(), &resources).is_err());
}

#[test]
fn frozen_event_prefix_suspension_exact_resume_and_storage_checksum_are_explicit() {
    let pack = test_pack(b"return {run=function(e,w,d) return {data=d,commands={}} end}");
    let mut instance = Instance::new(&pack);
    for sequence in 1..=70 {
        instance = instance.with_event(event(sequence)).required();
    }
    assert_eq!(instance.batch().len(), 64);
    assert_eq!(instance.batch().last().required().sequence, 64);
    assert!(matches!(
        instance.prepare(&pack, 2, &Value::empty_map()),
        Err(Error::Stale)
    ));
    let suspended = instance.reconcile(None).required();
    assert!(matches!(
        suspended.prepare(&pack, 1, &Value::empty_map()),
        Err(Error::Suspended)
    ));
    assert_eq!(suspended.pending(), instance.pending());
    assert_eq!(suspended.resumed(&pack).required(), instance);
    let wrong =
        test_pack(b"return {run=function(e,w,d) return {data={wrong=true},commands={}} end}");
    assert_eq!(suspended.resumed(&wrong), Err(Error::Identity));
    let mut bytes = instance.to_bytes().required();
    bytes[30] ^= 1;
    assert!(Instance::from_bytes(&bytes).is_err());
    let before = instance.clone();
    drop(instance.prepare(&pack, 1, &Value::empty_map()).required());
    assert_eq!(instance, before);
}

#[test]
fn loader_refuses_traversal_unlisted_resources_unimplemented_geometry_and_wrong_handlers() {
    let manifest = original_t01().manifest_bytes().required();
    let source = original_t01().entry_bytes().required().to_vec();
    let mut resources = BTreeMap::from([("scripts/main.lua".into(), source.clone())]);
    resources.insert("extra.lua".into(), source.clone());
    assert!(Pack::load(manifest, &resources).is_err());
    resources.remove("extra.lua");
    for replacement in ["../outside.lua", "/tmp/mission.lua", "scripts\\main.lua"] {
        let text = String::from_utf8(manifest.to_vec())
            .required()
            .replace("scripts/main.lua", replacement);
        assert!(Pack::load(text.as_bytes(), &resources).is_err());
    }
    let mut text = manifest.to_vec();
    text.extend(b"\n[[models]]\nid='unimplemented'\npath='ship.toml'\n");
    assert!(Pack::load(&text, &resources).is_err());
    resources.insert("scripts/main.lua".into(), b"return {}".to_vec());
    assert!(Pack::load(manifest, &resources).is_err());
}
