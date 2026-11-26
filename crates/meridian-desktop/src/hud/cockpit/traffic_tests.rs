//! Public World positives; altered projections below are labelled scanner geometry fixtures.
use super::{
    BLUE, CYAN, Cockpit, GREEN, Layout, MAX_CONTACTS, MAX_SCANNER_TARGETS, MAX_VERTICES,
    NPC_CONTACT_START, RED, YELLOW, append, fill_traffic_contacts, relative, scanner,
};
use crate::mesh::Vertex;
use meridian_core::docking::Vec3;
use meridian_core::traffic::{MAX_TRAFFIC_ACTORS, NpcPhase, Population, TrafficSnapshot};
use meridian_core::{FlightInput, FlightMode, FlightView, Snapshot, TargetIdentity, World};
use std::sync::OnceLock;

fn ok<T, E: core::fmt::Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|error| panic!("{error:?}"))
}
fn traffic_world() -> World {
    let mut world = ok(World::new_commander(12001));
    ok(world.request_launch(world.snapshot().tick));
    for _ in 0..1600 {
        ok(world.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        }));
        if world
            .traffic_snapshot()
            .is_some_and(|s| s.actors.len() == 2)
        {
            assert_eq!(world.snapshot().mode, FlightMode::Flight);
            return world;
        }
    }
    panic!("public flight did not admit the actual trader and pirate");
}
fn fixture_source() -> &'static (Snapshot, TrafficSnapshot) {
    static SOURCE: OnceLock<(Snapshot, TrafficSnapshot)> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let world = traffic_world();
        (
            world.snapshot(),
            world
                .traffic_snapshot()
                .unwrap_or_else(|| panic!("traffic")),
        )
    })
}
fn close(actual: [f32; 3], expected: [f32; 3]) {
    for (a, b) in actual.into_iter().zip(expected) {
        assert!((a - b).abs() < 0.001, "{a} != {b}");
    }
}
fn same_vertices(a: &[Vertex], b: &[Vertex]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.position, b.position);
        assert_eq!(a.color, b.color);
    }
}

#[test]
fn actual_world_contacts_bind_both_poses_and_security_without_changing_save_or_gauges() {
    let world = traffic_world();
    let flight = world.snapshot();
    let before = ok(world.save_bytes());
    let traffic = world
        .traffic_snapshot()
        .unwrap_or_else(|| panic!("traffic"));
    let cockpit = ok(Cockpit::capture(&world, flight));
    assert_eq!(traffic.actors.len(), 2);
    for actor in &traffic.actors {
        let contact = cockpit
            .scanner_targets
            .iter()
            .find(|contact| contact.identity == TargetIdentity::Ship(actor.id))
            .unwrap_or_else(|| panic!("missing actual selectable NPC"));
        close(contact.position, relative(flight, actor.pose.position));
        assert_eq!(actor.hull_id, "SH025");
        match actor.population {
            Population::Trader => {
                assert_eq!(actor.wanted, 0);
                assert_eq!(contact.color, YELLOW);
            }
            Population::Pirate => {
                assert!(actor.wanted > 0);
                assert_eq!(contact.color, RED);
            }
            _ => panic!("unadmitted population"),
        }
    }
    assert!(
        cockpit.contacts[NPC_CONTACT_START..]
            .iter()
            .all(Option::is_none)
    );
    let admitted = world.scanner_targets();
    assert_eq!(cockpit.scanner_targets.len(), admitted.len());
    for (contact, (id, point)) in cockpit.scanner_targets.iter().zip(admitted) {
        assert_eq!(contact.identity, id);
        close(contact.position, relative(flight, point));
        assert!(!contact.selected);
    }
    assert_eq!(cockpit.shield, [Some(1.0); 2]);
    assert_eq!(cockpit.energy, Some(1.0));
    assert_eq!(cockpit.heat, Some(0.0));
    assert_eq!(cockpit.flight, flight);
    assert_eq!(cockpit.missiles, 3);
    assert!(!cockpit.armed && !cockpit.locked && !cockpit.destroyed);
    assert_eq!(cockpit.status(), "FLIGHT");
    assert_eq!(ok(world.save_bytes()), before);
}

#[test]
fn translated_player_and_pitch_yaw_roll_use_ship_relative_axes_and_elevation() {
    let (flight, traffic) = fixture_source();
    let mut flight = *flight;
    let mut traffic = traffic.clone();
    traffic.actors.truncate(1);
    flight.position = [100.0, 200.0, 500.0];
    traffic.actors[0].pose.position = Vec3::new(110.0, 220.0, 530.0);
    let h = std::f32::consts::FRAC_PI_4;
    // Descriptive camera fixtures only; no World pose setter or admission assertion.
    for (orientation, expected) in [
        ([0.0, 0.0, 0.0, 1.0], [10.0, 20.0, 30.0]),
        ([0.0, h.sin(), 0.0, h.cos()], [-30.0, 20.0, 10.0]),
        ([h.sin(), 0.0, 0.0, h.cos()], [10.0, 30.0, -20.0]),
        ([0.0, 0.0, h.sin(), h.cos()], [20.0, -10.0, 30.0]),
    ] {
        flight.orientation = orientation;
        let mut contacts = [None; MAX_CONTACTS];
        ok(fill_traffic_contacts(flight, &traffic, &mut contacts));
        close(
            contacts[NPC_CONTACT_START]
                .unwrap_or_else(|| panic!("contact"))
                .0,
            expected,
        );
    }
}

#[test]
fn all_four_selected_views_keep_original_front_ship_scanner_basis_and_real_contacts() {
    let mut world = traffic_world();
    let opening = ok(Cockpit::capture(&world, world.snapshot()));
    let mut reference = Vec::new();
    scanner(
        &mut reference,
        &opening,
        Layout::new(1280, 720),
        [1280.0, 720.0],
    );
    for view in [
        FlightView::Front,
        FlightView::Rear,
        FlightView::Left,
        FlightView::Right,
    ] {
        let tick = world.snapshot().tick;
        ok(world.request_flight_view(view, tick));
        let saved = ok(world.save_bytes());
        let cockpit = ok(Cockpit::capture(&world, world.snapshot()));
        assert_eq!(cockpit.view, view);
        assert_eq!(cockpit.contacts, opening.contacts);
        assert_eq!(cockpit.scanner_targets, opening.scanner_targets);
        let mut output = Vec::new();
        scanner(
            &mut output,
            &cockpit,
            Layout::new(1280, 720),
            [1280.0, 720.0],
        );
        same_vertices(&output, &reference);
        assert_eq!(ok(world.save_bytes()), saved);
    }
}

#[test]
fn ten_phases_keep_destroyed_solids_omit_only_complete_and_color_actual_facts() {
    let (flight, traffic) = fixture_source();
    let mut traffic = traffic.clone();
    traffic.actors.truncate(1);
    for phase in [
        NpcPhase::Departing,
        NpcPhase::Outbound,
        NpcPhase::ReturnAlign,
        NpcPhase::Inbound,
        NpcPhase::Approach,
        NpcPhase::FinalAlign,
        NpcPhase::Hunting,
        NpcPhase::Retreating,
        NpcPhase::Destroyed,
        NpcPhase::Complete,
    ] {
        traffic.actors[0].phase = phase;
        for wanted in [0, 16] {
            traffic.actors[0].wanted = wanted;
            let mut contacts = [None; MAX_CONTACTS];
            ok(fill_traffic_contacts(*flight, &traffic, &mut contacts));
            let contact = contacts[NPC_CONTACT_START];
            if phase == NpcPhase::Complete {
                assert_eq!(contact, None);
            } else {
                let color = if phase == NpcPhase::Destroyed {
                    BLUE
                } else if wanted > 0 {
                    RED
                } else {
                    YELLOW
                };
                assert_eq!(
                    contact,
                    Some((relative(*flight, traffic.actors[0].pose.position), color))
                );
            }
        }
    }
}

#[test]
fn four_npc_slots_preserve_all_260_original_contacts_and_clear_only_their_own_suffix() {
    let (flight, traffic) = fixture_source();
    let mut traffic = traffic.clone();
    // Four-body capacity fixture, not a World spawn or four-identity admission claim.
    traffic.actors.resize(MAX_TRAFFIC_ACTORS, traffic.actors[0]);
    let mut contacts = std::array::from_fn(|i| Some(([i as f32, 10.0, -20.0], CYAN)));
    let original = contacts;
    ok(fill_traffic_contacts(*flight, &traffic, &mut contacts));
    assert_eq!(NPC_CONTACT_START, 4 + crate::mesh::MAX_MISSILE_PROJECTILES);
    assert_eq!(
        &contacts[..NPC_CONTACT_START],
        &original[..NPC_CONTACT_START]
    );
    assert_eq!(
        contacts[NPC_CONTACT_START..].iter().flatten().count(),
        MAX_TRAFFIC_ACTORS
    );
    traffic.actors[1].phase = NpcPhase::Complete;
    ok(fill_traffic_contacts(*flight, &traffic, &mut contacts));
    assert_eq!(contacts[NPC_CONTACT_START + 1], None);
    assert!(contacts[NPC_CONTACT_START + 2].is_some());
    traffic.actors.clear();
    ok(fill_traffic_contacts(*flight, &traffic, &mut contacts));
    assert!(contacts[NPC_CONTACT_START..].iter().all(Option::is_none));
    assert_eq!(
        &contacts[..NPC_CONTACT_START],
        &original[..NPC_CONTACT_START]
    );
}

#[test]
fn actual_station_practice_and_launched_projectile_keep_original_slots_and_colors() {
    let mut world = ok(World::new_commander(99120));
    let review = ok(world.review_practice(0));
    world
        .prepare_practice_confirmation(review, 0)
        .unwrap_or_else(|e| panic!("{e}"))
        .commit();
    ok(world.request_launch(0));
    ok(world.step(FlightInput::default()));
    let beacons = world
        .practice_snapshot()
        .and_then(|s| s.beacons)
        .unwrap_or_else(|| panic!("beacons"));
    let tick = world.snapshot().tick;
    ok(world.request_missile_lock(Some(beacons[1].id), tick));
    ok(world.request_missile_arm(true, tick));
    ok(world.request_fire_missile(tick));
    let flight = world.snapshot();
    let saved = ok(world.save_bytes());
    let cockpit = ok(Cockpit::capture(&world, flight));
    let station = flight.station.unwrap_or_else(|| panic!("station"));
    assert_eq!(cockpit.contacts[0], None);
    let selectable_station = cockpit
        .scanner_targets
        .iter()
        .filter(|contact| matches!(contact.identity, TargetIdentity::Station { .. }))
        .collect::<Vec<_>>();
    assert_eq!(selectable_station.len(), 1);
    assert_eq!(
        selectable_station[0].position,
        relative(flight, station.pose.position)
    );
    assert_eq!(selectable_station[0].color, GREEN);
    for (index, beacon) in beacons.iter().enumerate() {
        assert_eq!(
            cockpit.contacts[index + 1],
            Some((relative(flight, beacon.pose.position), CYAN))
        );
    }
    let projectiles: Vec<_> = world
        .missile_projectiles()
        .filter(|p| p.ended.is_none())
        .collect();
    assert_eq!(projectiles.len(), 1);
    assert_eq!(
        cockpit.contacts[4],
        Some((relative(flight, projectiles[0].position), RED))
    );
    assert!(cockpit.contacts[5..].iter().all(Option::is_none));
    assert_eq!(ok(world.save_bytes()), saved);
}

#[test]
fn unsupported_nonfinite_foreign_or_excess_traffic_refuses_without_partial_contact_updates() {
    let (flight, traffic) = fixture_source();
    let original = [Some(([1.0, 2.0, 3.0], GREEN)); MAX_CONTACTS];
    for population in [
        Population::Police,
        Population::Miner,
        Population::BountyHunter,
        Population::Alien,
        Population::AiConstruct,
    ] {
        let mut invalid = traffic.clone();
        invalid.actors[0].population = population;
        let mut contacts = original;
        assert!(fill_traffic_contacts(*flight, &invalid, &mut contacts).is_err());
        assert_eq!(contacts, original);
    }
    for kind in 0..5 {
        let mut invalid = traffic.clone();
        let mut flight = *flight;
        match kind {
            0 => invalid.actors[1].pose.position.x = f64::NAN,
            1 => invalid.actors[0].hull_id = "SH031",
            2 => invalid
                .actors
                .resize(MAX_TRAFFIC_ACTORS + 1, traffic.actors[0]),
            3 => flight.navigation = None,
            _ => {
                flight
                    .navigation
                    .as_mut()
                    .unwrap_or_else(|| panic!("navigation"))
                    .system = meridian_core::navigation::SystemId::RENAS
            }
        }
        let mut contacts = original;
        assert!(fill_traffic_contacts(flight, &invalid, &mut contacts).is_err());
        assert_eq!(contacts, original);
    }
}

#[test]
fn maximum_contact_geometry_and_npc_elevation_fit_existing_reused_hud_buffers() {
    let world = traffic_world();
    let mut cockpit = ok(Cockpit::capture(&world, world.snapshot()));
    // Full descriptive scanner fixture includes every original slot plus all four NPC slots.
    for (i, contact) in cockpit.contacts.iter_mut().enumerate() {
        *contact = Some(([i as f32 * 100.0 - 13000.0, 6000.0, -8000.0], YELLOW));
    }
    assert_eq!(MAX_CONTACTS, NPC_CONTACT_START + MAX_TRAFFIC_ACTORS);
    let sample = *cockpit
        .scanner_targets
        .first()
        .unwrap_or_else(|| panic!("scanner target"));
    cockpit.scanner_targets = (0..MAX_SCANNER_TARGETS)
        .map(|i| super::ScannerTarget {
            position: [i as f32 * 100.0 - 13000.0, 6000.0, -8000.0],
            selected: i == 0,
            ..sample
        })
        .collect();
    let mut output = Vec::with_capacity(MAX_VERTICES);
    let capacity = output.capacity();
    for [width, height] in [[320, 240], [640, 360], [1280, 720], [2560, 1600]] {
        output.clear();
        append(&cockpit, width, height, &mut output);
        assert!(output.len() <= MAX_VERTICES);
        assert_eq!(output.capacity(), capacity);
        assert!(
            output
                .iter()
                .flat_map(|v| v.position)
                .all(|n| n.is_finite() && n.abs() <= 1.000001)
        );
    }
}

#[test]
fn no_core_scanner_shows_real_work_and_one_selected_contact_not_context_markers() {
    let mut world = traffic_world();
    assert!(world.targeting_tier().is_none());
    let admitted = world.scanner_targets();
    assert!(admitted.iter().any(|(identity, _)| matches!(
        identity,
        TargetIdentity::Work(meridian_core::equipment::WorkTargetId::Asteroid(_))
    )));
    let tick = world.snapshot().tick;
    ok(world.cycle_scanner_target(tick, [1280, 720]));
    let selected = world
        .targeting_snapshot()
        .unwrap_or_else(|| panic!("manual mark"))
        .identity;
    assert!(world.manual_scanner_target());
    let cockpit = ok(Cockpit::capture(&world, world.snapshot()));
    let marked = cockpit
        .scanner_targets
        .iter()
        .filter(|contact| contact.selected)
        .collect::<Vec<_>>();
    assert_eq!(marked.len(), 1);
    assert_eq!(marked[0].identity, selected);
    assert_eq!(
        cockpit
            .scanner_targets
            .iter()
            .map(|contact| contact.identity)
            .collect::<Vec<_>>(),
        admitted
            .iter()
            .map(|(identity, _)| *identity)
            .collect::<Vec<_>>()
    );

    // A synthetic projection comparison of marker geometry only, never a manufactured
    // World target. A selectable bar, contextual cross and selected bracket differ.
    let mut sample = *marked[0];
    let mut display = cockpit;
    display.contacts.fill(None);
    sample.position = [100.0, 200.0, 300.0];
    sample.selected = false;
    display.scanner_targets = vec![sample];
    let mut bar = Vec::new();
    scanner(&mut bar, &display, Layout::new(1280, 720), [1280.0, 720.0]);
    display.scanner_targets[0].selected = true;
    let mut bracket = Vec::new();
    scanner(
        &mut bracket,
        &display,
        Layout::new(1280, 720),
        [1280.0, 720.0],
    );
    assert_eq!(bracket.len(), bar.len() + 24);
    display.scanner_targets.clear();
    display.contacts[1] = Some((sample.position, sample.color));
    let mut cross = Vec::new();
    scanner(
        &mut cross,
        &display,
        Layout::new(1280, 720),
        [1280.0, 720.0],
    );
    assert_eq!(cross.len(), bar.len() + 6);
}
