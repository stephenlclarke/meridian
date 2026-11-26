//! Pure replay and whole World refusal cases. Authored UNRUN in the source-only lane.
use super::*;

#[test]
fn wide_shield_grade_changes_replay_with_damage_recharge_and_berth_without_reset() {
    let effects = [
        (0, 0, 1, Effect::ShieldFit(ShieldGrade::AdvancedII)),
        (
            1,
            1,
            1,
            Effect::Hit {
                front: true,
                damage: 16_000,
            },
        ),
        (
            1,
            2,
            0,
            Effect::Hit {
                front: false,
                damage: 1_500,
            },
        ),
        (11, 0, 1, Effect::ShieldFit(ShieldGrade::Elite)),
        (
            12,
            2,
            1,
            Effect::Hit {
                front: true,
                damage: 1_500,
            },
        ),
    ];
    let state = ok(replay(&effects, 12));
    assert_eq!(state.shields.grade(), ShieldGrade::Elite);
    // At 11 the old banks are 191030/205530 of 207000. Scale by exactly 4/3,
    // round to nearest hundredth, then add one recharge tick and damage the front.
    assert_eq!(state.shields.amounts(), [253_210, 274_043]);
    assert_eq!(state.pulse, PulseResources::default());
    let mut docked = effects.to_vec();
    docked.push((13, 0, 0, Effect::Berth));
    let state = ok(replay(&docked, 13));
    assert_eq!(state.shields.amounts(), [276_000; 2]);
    assert_eq!(state.shields.grade(), ShieldGrade::Elite);
}
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("unexpected refusal: {e:?}"))
}
#[test]
fn interleaved_missile_emissions_leave_each_weapon_slot_and_later_missile_pending() {
    // Component sequence only. No World receipt or successful launch is synthesized.
    let mut missiles = [(1, 5), (3, 7), (4, 7), (6, 11)].into_iter().peekable();
    assert_eq!(merge_missile_emissions(&mut missiles, 1, 5), Ok(2));
    assert_eq!(missiles.peek(), Some(&(3, 7)));
    assert_eq!(merge_missile_emissions(&mut missiles, 3, 7), Ok(5));
    assert_eq!(missiles.peek(), Some(&(6, 11)));
    assert_eq!(merge_missile_emissions(&mut missiles, 6, 11), Ok(7));
    assert_eq!(missiles.next(), None);
    assert_eq!(merge_missile_emissions(&mut missiles, 8, 12), Ok(8));
}
#[test]
fn duplicate_missing_and_future_missile_emissions_refuse_before_consuming_the_bad_receipt() {
    for (receipts, rejected) in [
        (vec![(1, 9), (1, 9)], (1, 9)),
        (vec![(2, 9)], (2, 9)),
        (vec![(1, 11)], (1, 11)),
    ] {
        let mut missiles = receipts.into_iter().peekable();
        assert_eq!(merge_missile_emissions(&mut missiles, 1, 10), Err(INVALID));
        assert_eq!(missiles.peek(), Some(&rejected));
    }
}
#[test]
fn incoming_missile_then_pulse_share_facing_shields_and_actual_energy_once() {
    let events = [
        (
            1,
            1,
            1,
            Effect::Hit {
                front: true,
                damage: 16000,
            },
        ),
        (
            1,
            1,
            2,
            Effect::Hit {
                front: true,
                damage: 16000,
            },
        ),
        (
            1,
            2,
            0,
            Effect::Hit {
                front: true,
                damage: 1500,
            },
        ),
        (
            1,
            3,
            0,
            Effect::Laser(crate::OrdinaryWeapon::FrontPulseLaser),
        ),
    ];
    let actual = ok(replay(&events, 1));
    let mut expected = PulseResources::default();
    expected.damage_energy(3500);
    expected = ok(expected.prepare_discharge());
    assert_eq!(actual.pulse, expected);
    assert_eq!(actual.shields.amounts(), [0, 30000]);
    assert_eq!(actual.shields.grade(), ShieldGrade::Basic);
    let after = ok(replay(&events, 19));
    let mut expected = actual;
    advance(&mut expected, 18);
    assert_eq!(after, expected);
}
#[test]
fn rear_damage_and_true_berth_reset_are_exact_with_retained_weapon_cooldown() {
    let events = [
        (
            1,
            1,
            1,
            Effect::Hit {
                front: false,
                damage: 16000,
            },
        ),
        (
            1,
            2,
            0,
            Effect::Hit {
                front: false,
                damage: 1500,
            },
        ),
        (
            1,
            3,
            0,
            Effect::Laser(crate::OrdinaryWeapon::FrontPulseLaser),
        ),
        (2, 0, 0, Effect::Berth),
    ];
    let actual = ok(replay(&events, 2));
    assert_eq!(actual.shields.amounts(), [30000; 2]);
    let mut expected = ok(PulseResources::default().prepare_discharge());
    expected.replay_advance(1);
    expected.replay_berth();
    assert_eq!(actual.pulse, expected);
    assert!(actual.pulse.prepare_discharge().is_err());
}
#[test]
fn hull_overkill_is_saturating_and_npc_cannot_emit_a_new_hit_on_dead_player() {
    let mut events = Vec::new();
    for sequence in 0..5 {
        events.push((
            1,
            1,
            sequence,
            Effect::Hit {
                front: true,
                damage: 16000,
            },
        ));
    }
    let dead = ok(replay(&events, 1));
    assert!(!dead.pulse.alive());
    events.push((
        1,
        2,
        0,
        Effect::Hit {
            front: true,
            damage: 1500,
        },
    ));
    assert!(replay(&events, 1).is_err());
}
#[test]
fn huge_elapsed_clock_is_saturating_without_per_tick_replay_and_bad_cadence_refuses() {
    let events = [(
        1,
        3,
        0,
        Effect::Laser(crate::OrdinaryWeapon::FrontPulseLaser),
    )];
    assert_eq!(
        ok(replay(&events, u64::MAX)).pulse,
        PulseResources::default()
    );
    let bad = [
        (
            1,
            3,
            0,
            Effect::Laser(crate::OrdinaryWeapon::FrontPulseLaser),
        ),
        (
            18,
            3,
            0,
            Effect::Laser(crate::OrdinaryWeapon::FrontPulseLaser),
        ),
    ];
    assert!(replay(&bad, 18).is_err());
    assert!(replay(&events, 0).is_err());
}
#[test]
fn missing_spend_or_unbound_lower_energy_cannot_borrow_an_unrelated_hit() {
    let mut world = ok(World::new_commander(19001));
    crate::flight::practice::tests::launch(&mut world);
    crate::flight::missiles::tests::stop(&mut world);
    crate::flight::practice::tests::aim_at(
        &mut world,
        crate::docking::Vec3::new(0.0, 0.0, 20000.0),
    );
    ok(world.request_fire_front_pulse(world.snapshot().tick));
    let good = ok(world.save_bytes());
    let mut bad = ok(World::from_save_bytes(&good));
    bad.damage = State::fresh();
    let corrupt = ok(bad.unchecked_test_save());
    assert!(world.restore_bytes(&corrupt).is_err());
    assert_eq!(ok(world.save_bytes()), good);
    let mut bad = ok(World::from_save_bytes(&good));
    bad.pulse.damage_energy(1);
    assert!(World::from_save_bytes(&ok(bad.unchecked_test_save())).is_err());
}
#[test]
fn exact_byte_budget_refuses_coupled_fire_before_resources_receipts_or_legal_change() {
    let mut world = ok(World::new_commander(19002));
    crate::flight::practice::tests::launch(&mut world);
    crate::flight::missiles::tests::stop(&mut world);
    // Source-level boundary fixture: do not claim this synthetic saturated journal is a
    // playable or semantically valid save. It exercises pre-publication capacity only.
    world.damage.events = Arc::new(vec![
        Event {
            clock: 1,
            kind: Kind::Pulse
        };
        MAX_EVENTS
    ]);
    let resources = world.flight_resources_snapshot();
    let combat = world.combat;
    let traffic = world.traffic.clone();
    let markets = world.service_snapshot();
    let legal = world.legal_value();
    assert_eq!(
        world.request_fire_front_pulse(world.snapshot().tick),
        Err(FireError::SequenceExhausted)
    );
    assert_eq!(world.flight_resources_snapshot(), resources);
    assert_eq!(world.combat, combat);
    assert_eq!(world.traffic, traffic);
    assert_eq!(world.service_snapshot(), markets);
    assert_eq!(world.legal_value(), legal);
}
#[test]
fn whole_save_byte_limit_refuses_a_real_discharge_before_the_decode_event_bound() {
    let mut world = ok(World::new_commander(19003));
    crate::flight::practice::tests::launch(&mut world);
    crate::flight::missiles::tests::stop(&mut world);
    crate::flight::practice::tests::aim_at(
        &mut world,
        crate::docking::Vec3::new(0.0, 0.0, 20000.0),
    );
    let base = ok(world.encoded_current_size());
    let count = (MAX_SAVE_BYTES - base) / 9;
    assert!(count < MAX_EVENTS);
    // Negative boundary fixture only. The real whole-save encoder measures the envelope;
    // the synthetic journal is deliberately not presented as a valid journey receipt.
    world.damage.events = Arc::new(vec![
        Event {
            clock: 1,
            kind: Kind::Pulse
        };
        count
    ]);
    assert!(ok(world.encoded_current_size()) <= MAX_SAVE_BYTES);
    let pulse = world.pulse;
    let combat = world.combat;
    let traffic = world.traffic.clone();
    let damage = world.damage.clone();
    assert_eq!(
        world.request_fire_front_pulse(world.snapshot().tick),
        Err(FireError::SequenceExhausted)
    );
    assert_eq!(world.pulse, pulse);
    assert_eq!(world.combat, combat);
    assert_eq!(world.traffic, traffic);
    assert_eq!(world.damage, damage);
}
#[test]
fn byte_capacity_failed_native_npc_step_keeps_player_attacker_market_tick_and_receipts() {
    let mut world = ok(World::new_commander(12001));
    ok(world.request_station_service(crate::station_services::Command::BuyFood, 0));
    crate::flight::practice::tests::launch(&mut world);
    crate::flight::missiles::tests::stop(&mut world);
    crate::flight::missiles::tests::fly_axis(&mut world, 0, -600.0, 10.0);
    let mut contacted = false;
    for _ in 0..24000 {
        ok(world.step(crate::FlightInput::default()));
        if world
            .traffic_snapshot()
            .is_some_and(|s| s.last_player_hit.is_some())
        {
            contacted = true;
            break;
        }
    }
    assert!(contacted);
    let _actual_valid_save = ok(world.save_bytes());
    // Separate negative boundary fixture after real public contact, not a valid journey save.
    let count = (MAX_SAVE_BYTES - ok(world.encoded_current_size())) / 9;
    world.damage.events = Arc::new(vec![
        Event {
            clock: 1,
            kind: Kind::Pulse
        };
        count
    ]);
    let mut refused = false;
    for _ in 0..180 {
        let before = ok(world.unchecked_test_save());
        let snapshot = world.snapshot();
        let credits = world.credits_tenths();
        match world.step(crate::FlightInput::default()) {
            Ok(()) => {}
            Err(StepError::TickOverflow) => {
                assert_eq!(ok(world.unchecked_test_save()), before);
                assert_eq!(world.snapshot(), snapshot);
                assert_eq!(world.credits_tenths(), credits);
                refused = true;
                break;
            }
            Err(e) => panic!("unexpected step failure: {e:?}"),
        }
    }
    assert!(
        refused,
        "next native NPC beam did not reach the pre-publication byte gate"
    );
}

#[test]
fn exact_pool_replay_refuses_a_journal_spend_shifted_before_its_actual_impact() {
    let mut world = ok(World::new_commander(1471));
    crate::flight::practice::tests::launch(&mut world);
    crate::flight::missiles::tests::stop(&mut world);
    crate::flight::practice::tests::aim_at(
        &mut world,
        crate::docking::Vec3::new(70.0, 0.0, 136.32),
    );
    ok(world.request_fire_front_pulse(world.snapshot().tick));
    let hit = world
        .combat
        .last_impact()
        .unwrap_or_else(|| panic!("missing real station hit"));
    assert_eq!(hit.target, OrdinaryTarget::MainStation);
    assert_eq!(hit.physical_tick, world.combat.physical_tick());
    let good = ok(world.save_bytes());
    let mut forged = ok(World::from_save_bytes(&good));
    let events = Arc::make_mut(&mut forged.damage.events);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        Event {
            clock: hit.physical_tick,
            kind: Kind::Pulse
        }
    );
    events[0].clock -= 1;
    forged.pulse.replay_advance(1);
    let resources = forged
        .flight_resources_snapshot()
        .unwrap_or_else(|| panic!("missing resource pools"));
    assert_eq!(
        (
            resources.energy_hundredths,
            resources.heat_fifths,
            resources.cooldown_ticks
        ),
        (37907, 49, 17)
    );
    let bytes = ok(forged.unchecked_test_save());
    assert_eq!(world.restore_bytes(&bytes), Err(INVALID));
    assert_eq!(ok(world.save_bytes()), good);
}

#[test]
fn one_real_station_discharge_accepts_an_identical_mirror_and_refuses_npc_ownership() {
    let mut world = ok(World::new_commander(1471));
    crate::flight::practice::tests::launch(&mut world);
    crate::flight::missiles::tests::stop(&mut world);
    crate::flight::practice::tests::aim_at(
        &mut world,
        crate::docking::Vec3::new(70.0, 0.0, 136.32),
    );
    ok(world.request_fire_front_pulse(world.snapshot().tick));
    let hit = world
        .combat
        .last_impact()
        .unwrap_or_else(|| panic!("missing real station hit"));
    assert_eq!(hit.target, OrdinaryTarget::MainStation);
    assert_eq!(
        validate_exclusive_laser_targets([hit, hit], [hit.emission + 1]),
        Ok(())
    );
    assert_eq!(
        validate_exclusive_laser_targets([hit, hit], [hit.emission]),
        Err(INVALID)
    );

    let mut different = hit;
    different.incident += 1;
    assert_eq!(
        validate_exclusive_laser_targets([hit, different], []),
        Err(INVALID)
    );
}
