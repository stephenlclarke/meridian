//! Public scanner cycling and read-only/manual target gates. No renderer hints or state injection.
use meridian_core::{
    FlightInput, FlightMode, FlightView, ModuleAction, ModuleKind, ModuleSlot, StepError,
    TargetIdentity, World, XyzzyAction,
    docking::Vec3,
    equipment::{TargetingTier, WorkTargetId},
    navigation::NavigationGrade,
    traffic::NpcPhase,
};

const VIEW: [u32; 2] = [1280, 720];

fn ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("{error:?}"))
}

fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("required public World value"))
}

fn module(world: &mut World, action: ModuleAction) -> u64 {
    let tick = world.snapshot().tick;
    let review = ok(world.review_module(action, tick));
    ok(world.prepare_module(review, tick)).commit()
}

fn owned(world: &World, kind: ModuleKind) -> u64 {
    some(
        some(world.modular_equipment_snapshot())
            .items
            .into_iter()
            .find(|item| item.kind == kind),
    )
    .id
}

fn store(world: &mut World, kind: ModuleKind) {
    let item = owned(world, kind);
    module(world, ModuleAction::Store { item });
}

fn outfit(seed: u64) -> World {
    let mut world = ok(World::new_commander(seed));
    // This is the explicit supported test outfit, not a claim of earned funding.
    let review = ok(world.review_xyzzy(XyzzyAction::Enter, 0));
    ok(world.prepare_xyzzy(review, 0)).commit();
    world
}

fn basic_without_specialists(seed: u64) -> World {
    let mut world = outfit(seed);
    store(&mut world, ModuleKind::MiningTargeting);
    store(&mut world, ModuleKind::SalvageTargeting);
    store(&mut world, ModuleKind::Targeting(TargetingTier::Military));
    module(
        &mut world,
        ModuleAction::Purchase {
            kind: ModuleKind::Targeting(TargetingTier::Basic),
        },
    );
    assert_eq!(world.targeting_tier(), Some(TargetingTier::Basic));
    assert!(!world.work_targeting_fit().mining && !world.work_targeting_fit().salvage);
    world
}

fn launch(world: &mut World) {
    ok(world.request_launch(world.snapshot().tick));
    ok(world.step(FlightInput::default()));
    assert_eq!(world.snapshot().mode, FlightMode::Flight);
}

fn stop(world: &mut World) {
    for _ in 0..300 {
        if world.snapshot().speed == 0.0 {
            return;
        }
        ok(world.step(FlightInput {
            brake: true,
            ..FlightInput::default()
        }));
        assert_eq!(world.snapshot().motion_refusal, None);
    }
    panic!("ordinary braking failed to stop");
}

fn cycle(world: &mut World) -> TargetIdentity {
    let tick = world.snapshot().tick;
    assert!(ok(world.cycle_scanner_target(tick, VIEW)));
    some(world.targeting_snapshot()).identity
}

fn station_identity(world: &World) -> TargetIdentity {
    let nav = some(world.snapshot().navigation);
    TargetIdentity::Station {
        system: nav.system,
        visit: nav.visit,
    }
}

fn select(world: &mut World, wanted: TargetIdentity) {
    for _ in 0..64 {
        if cycle(world) == wanted {
            return;
        }
    }
    panic!("physical scanner contact not found: {wanted:?}");
}

fn offset(world: &World, p: Vec3) -> Vec3 {
    let [x, y, z] = world.snapshot().position.map(f64::from);
    Vec3::new(p.x - x, p.y - y, p.z - z)
}

fn distance_squared(world: &World, p: Vec3) -> f64 {
    let delta = offset(world, p);
    delta.dot(delta)
}

fn forward(world: &World) -> Vec3 {
    let [x, y, z, w] = world.snapshot().orientation.map(f64::from);
    Vec3::new(
        2.0 * (x * z + w * y),
        2.0 * (y * z - w * x),
        1.0 - 2.0 * (x * x + y * y),
    )
}

#[test]
fn no_core_cycles_actual_scanner_contacts_by_distance_wraps_and_includes_the_rear() {
    // The existing public local traffic fixture uses this seed and ordinary outward flight.
    let mut world = ok(World::new_commander(12001));
    assert_eq!(world.targeting_tier(), None);
    launch(&mut world);
    for _ in 0..1600 {
        if some(world.traffic_snapshot()).actors.len() == 2 {
            break;
        }
        ok(world.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        }));
    }
    let traffic = some(world.traffic_snapshot());
    assert_eq!(traffic.actors.len(), 2);
    assert!(
        traffic
            .actors
            .iter()
            .all(|actor| !matches!(actor.phase, NpcPhase::Complete | NpcPhase::Destroyed))
    );
    let snapshot = world.snapshot();
    let station = some(snapshot.station).pose.position;
    let mut expected = vec![(station_identity(&world), station)];
    expected.extend(
        traffic
            .actors
            .into_iter()
            .map(|actor| (TargetIdentity::Ship(actor.id), actor.pose.position)),
    );
    let belt = some(world.mining_belt_snapshot());
    assert!(belt.pending_ore.is_empty());
    expected.extend(
        belt.bodies
            .into_iter()
            .filter(|body| body.remaining_energy > 0.0 && body.remaining_units > 0)
            .map(|body| {
                (
                    TargetIdentity::Work(WorkTargetId::Asteroid(body.id)),
                    body.position,
                )
            }),
    );
    let rear: Vec<_> = expected
        .iter()
        .filter(|(_, point)| offset(&world, *point).dot(forward(&world)) < 0.0)
        .map(|(identity, _)| *identity)
        .collect();
    assert!(
        !rear.is_empty(),
        "the actual fixture must include a rear contact"
    );
    let scanner = world.scanner_targets();
    assert_eq!(scanner.len(), expected.len());
    assert!(expected.iter().all(|contact| scanner.contains(contact)));
    ok(world.request_flight_view(FlightView::Rear, snapshot.tick));
    let before_resources = world.flight_resources_snapshot();
    let before_missiles = world.missile_snapshot();
    let before_front = world.pulse_laser_snapshot();
    let before_cash = world.credits_tenths();
    let mut visited = Vec::new();
    let mut previous_distance = 0.0;
    for _ in 0..expected.len() {
        let id = cycle(&mut world);
        assert!(world.manual_scanner_target());
        assert_eq!(some(world.targeting_snapshot()).tier, None);
        assert!(
            !visited.contains(&id),
            "scanner repeated {id:?} before wrap"
        );
        let (_, position) = some(expected.iter().find(|(candidate, _)| *candidate == id));
        let distance = distance_squared(&world, *position);
        assert!(
            distance >= previous_distance,
            "scanner order decreased at {id:?}"
        );
        previous_distance = distance;
        visited.push(id);
    }
    assert!(rear.iter().all(|id| visited.contains(id)));
    assert_eq!(
        visited,
        scanner.iter().map(|(id, _)| *id).collect::<Vec<_>>()
    );
    assert_eq!(cycle(&mut world), visited[0]);
    assert_eq!(world.flight_view(), FlightView::Rear);
    // Selection neither changes the manual weapon inventory/resources nor authorises
    // an automatic action. It does not need a renderer-provided list.
    assert_eq!(world.flight_resources_snapshot(), before_resources);
    assert_eq!(world.missile_snapshot(), before_missiles);
    assert_eq!(world.pulse_laser_snapshot(), before_front);
    assert_eq!(world.credits_tenths(), before_cash);
    assert!(!world.auto_docking_active());
    let manual = FlightInput {
        yaw: -0.25,
        throttle: 0.4,
        ..FlightInput::default()
    };
    assert_eq!(
        world.targeting_flight_input(FlightInput::default()),
        FlightInput::default()
    );
    assert_eq!(world.targeting_flight_input(manual), manual);
    assert_eq!(some(world.targeting_snapshot()).identity, visited[0]);
}

#[test]
fn scanner_refuses_stale_or_zero_view_without_displacing_a_manual_marker() {
    let mut world = ok(World::new_commander(12002));
    launch(&mut world);
    let id = cycle(&mut world);
    let tick = world.snapshot().tick;
    assert_eq!(
        world.cycle_scanner_target(tick - 1, VIEW),
        Err(StepError::StaleCommand)
    );
    assert_eq!(some(world.targeting_snapshot()).identity, id);
    assert!(world.cycle_scanner_target(tick, [0, 720]).is_err());
    assert_eq!(some(world.targeting_snapshot()).identity, id);
    let cold = ok(World::from_save_bytes(&ok(world.save_bytes())));
    assert!(cold.targeting_snapshot().is_none());
}

#[test]
fn basic_core_aligns_rear_station_but_needs_docking_tier_for_automatic_approach() {
    let mut world = basic_without_specialists(12003);
    launch(&mut world);
    stop(&mut world);
    let station = some(world.snapshot().station).pose.position;
    assert!(offset(&world, station).dot(forward(&world)) < 0.0);
    let id = station_identity(&world);
    select(&mut world, id);
    assert!(!world.manual_scanner_target());
    assert!(!ok(world.toggle_scanner_assistance()));
    assert!(world.manual_scanner_target());
    let manual = FlightInput {
        yaw: 0.2,
        throttle: 0.1,
        ..FlightInput::default()
    };
    assert_eq!(world.targeting_flight_input(manual), manual);
    assert_eq!(some(world.targeting_snapshot()).identity, id);
    assert!(ok(world.toggle_scanner_assistance()));
    assert!(!world.manual_scanner_target());
    let aligned = world.targeting_flight_input(FlightInput::default());
    assert!(aligned.yaw != 0.0 || aligned.pitch != 0.0);
    assert_eq!(aligned.throttle, 0.0);
    assert!(!world.auto_docking_active());
    let before = world.snapshot().position;
    ok(world.step(FlightInput::default()));
    assert!(!world.auto_docking_active());
    assert_eq!(world.snapshot().position, before);
    assert_eq!(world.snapshot().speed, 0.0);
}

#[test]
fn scanner_station_action_requires_both_docking_tier_and_navigation() {
    let mut fitted = outfit(12004);
    assert!(some(fitted.targeting_tier()) >= TargetingTier::Docking);
    assert_ne!(fitted.navigation_computer_grade(), NavigationGrade::Empty);
    launch(&mut fitted);
    stop(&mut fitted);
    let id = station_identity(&fitted);
    select(&mut fitted, id);
    ok(fitted.step(FlightInput::default()));
    assert!(fitted.auto_docking_active());
    fitted.cancel_targeting();
    assert!(!fitted.auto_docking_active());

    let mut without_nav = outfit(12006);
    let navigation = some(
        some(without_nav.modular_equipment_snapshot())
            .items
            .into_iter()
            .find(|item| item.slot == Some(ModuleSlot::Navigation)),
    )
    .id;
    module(&mut without_nav, ModuleAction::Store { item: navigation });
    assert_eq!(
        without_nav.navigation_computer_grade(),
        NavigationGrade::Empty
    );
    launch(&mut without_nav);
    stop(&mut without_nav);
    let id = station_identity(&without_nav);
    select(&mut without_nav, id);
    ok(without_nav.step(FlightInput::default()));
    assert!(!without_nav.auto_docking_active());
}

#[test]
fn rock_is_a_real_track_without_specialist_and_auto_work_requires_matching_fit() {
    let mut track_only = basic_without_specialists(12007);
    launch(&mut track_only);
    stop(&mut track_only);
    let belt = some(track_only.mining_belt_snapshot());
    let rock = belt.bodies[0];
    let id = TargetIdentity::Work(WorkTargetId::Asteroid(rock.id));
    select(&mut track_only, id);
    let before_cargo = track_only.commodity_quantity(rock.commodity);
    let before_front = track_only.front_laser_snapshot();
    let directed = track_only.targeting_flight_input(FlightInput::default());
    assert_eq!(directed.throttle, 0.0);
    ok(track_only.step(FlightInput::default()));
    let after = some(track_only.mining_belt_snapshot());
    assert_eq!(after.bodies, belt.bodies);
    assert!(after.pending_ore.is_empty());
    assert_eq!(track_only.commodity_quantity(rock.commodity), before_cargo);
    assert_eq!(track_only.front_laser_snapshot(), before_front);
    assert_eq!(track_only.snapshot().speed, 0.0);

    let mut fitted = outfit(12008);
    assert!(fitted.work_targeting_fit().mining);
    assert!(fitted.fitted_fuel_scoops().is_some());
    launch(&mut fitted);
    stop(&mut fitted);
    let original_front = some(fitted.front_laser_snapshot()).fitted_item_id;
    let rock = some(fitted.mining_belt_snapshot()).bodies[0];
    let id = TargetIdentity::Work(WorkTargetId::Asteroid(rock.id));
    select(&mut fitted, id);
    ok(fitted.step(FlightInput::default()));
    assert_eq!(some(fitted.targeting_snapshot()).identity, id);
    assert!(fitted.automatic_work_status().is_some());
    // One far-away guidance tick cannot deplete or transfer any physical material.
    assert_eq!(some(fitted.mining_belt_snapshot()).bodies[0], rock);
    assert!(fitted.fitted_mining_tool().is_some());
    let manual = FlightInput {
        brake: true,
        ..FlightInput::default()
    };
    assert_eq!(fitted.targeting_flight_input(manual), manual);
    assert!(fitted.manual_scanner_target());
    assert_eq!(some(fitted.targeting_snapshot()).identity, id);
    ok(fitted.step(manual));
    assert!(fitted.manual_scanner_target());
    assert_eq!(some(fitted.targeting_snapshot()).identity, id);
    assert_eq!(
        some(fitted.front_laser_snapshot()).fitted_item_id,
        original_front
    );

    // F resumes assistance on the same retained mark after a stick override.
    assert!(ok(fitted.toggle_scanner_assistance()));
    assert!(!fitted.manual_scanner_target());
    assert_eq!(some(fitted.targeting_snapshot()).identity, id);
    ok(fitted.step(FlightInput::default()));
    assert!(fitted.fitted_mining_tool().is_some());

    // Tab after another stick override selects the next physical scanner contact
    // and restores assistance instead of remaining in the temporary pilot override.
    assert_eq!(fitted.targeting_flight_input(manual), manual);
    assert!(fitted.manual_scanner_target());
    let next = cycle(&mut fitted);
    assert_ne!(next, id);
    assert!(!fitted.manual_scanner_target());
    assert_eq!(some(fitted.targeting_snapshot()).identity, next);
}
