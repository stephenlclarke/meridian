use super::*;
fn world() -> meridian_core::World {
    meridian_core::World::new_commander(96001).unwrap_or_else(|e| panic!("{e}"))
}
fn launched() -> meridian_core::World {
    let mut w = world();
    w.request_launch(0).unwrap_or_else(|e| panic!("{e}"));
    w.step(meridian_core::FlightInput::default())
        .unwrap_or_else(|e| panic!("{e}"));
    w
}
fn station(w: &meridian_core::World) -> StationRender<'_> {
    StationRender {
        vertices: &[],
        pose: w.snapshot().station.unwrap_or_else(|| panic!("pose")),
        port: w.station_port().unwrap_or_else(|| panic!("port")),
        docked: false,
        medusa_preview: false,
    }
}
#[test]
fn triangles_are_exact_physical_faces_in_the_actual_camera_and_do_not_change_world() {
    let w = launched();
    let s = w.snapshot();
    let belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    let before = w.save_bytes().unwrap_or_else(|e| panic!("{e}"));
    let mut out = Vec::with_capacity(VERTEX_COUNT);
    append(&s, station(&w), &belt, None, &mut out).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out.len(), VERTEX_COUNT);
    let camera = mesh::scene_camera_pose(&s, station(&w)).unwrap_or_else(|e| panic!("{e}"));
    for (actual, point) in out.iter().zip(belt.bodies.iter().flat_map(|body| {
        ASTEROID_FACES
            .into_iter()
            .flat_map(|f| f.map(|i| body.vertices[i]))
    })) {
        let p = camera
            .orientation
            .inverse_coordinates(sub(point, camera.position));
        assert_eq!(actual.position, [p.x as f32, p.y as f32, p.z as f32]);
    }
    assert_eq!(w.save_bytes().unwrap_or_else(|e| panic!("{e}")), before);
}
#[test]
fn terminal_snapshot_removes_only_the_spent_mesh_and_cues_report_actual_native_units() {
    let w = launched();
    let s = w.snapshot();
    let mut belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    let mut out = Vec::with_capacity(VERTEX_COUNT);
    append(&s, station(&w), &belt, None, &mut out).unwrap_or_else(|e| panic!("{e}"));
    let original: Vec<_> = out.iter().map(|v| v.position).collect();
    // Descriptive renderer fixture only: it cannot change the World depletion ledger.
    belt.bodies[0].remaining_energy = 0.0;
    belt.bodies[0].remaining_units = 0;
    belt.bodies[0].extracted_units = belt.bodies[0].initial_units;
    out.clear();
    append(&s, station(&w), &belt, None, &mut out).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out.len(), VERTEX_COUNT - ASTEROID_FACES.len() * 3);
    assert_eq!(
        out.iter().map(|v| v.position).collect::<Vec<_>>(),
        original[ASTEROID_FACES.len() * 3..]
    );
    let cues = cues(s, &belt).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(cues[0].id, belt.bodies[0].id);
    assert_eq!(cues[0].remaining, 0);
    assert!(
        cues[0]
            .line()
            .unwrap_or_else(|e| panic!("{e}"))
            .contains("DEPLETED")
    );
    assert!(cues.iter().all(|c| {
        [c.range, c.bearing_degrees, c.elevation_degrees]
            .into_iter()
            .all(f64::is_finite)
    }));
}
#[test]
fn partial_work_changes_the_front_contact_cue_but_preserves_every_face() {
    let w = launched();
    let s = w.snapshot();
    let mut belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    let before = w.save_bytes().unwrap_or_else(|error| panic!("{error}"));
    let mut intact = Vec::new();
    append(&s, station(&w), &belt, None, &mut intact).unwrap_or_else(|error| panic!("{error}"));

    belt.bodies[0].remaining_energy /= 2.0;
    let body = belt.bodies[0];
    belt.front_contact = Some(meridian_core::mining::MiningContact {
        asteroid: body.id,
        distance: 1.0,
        point: body.vertices[0],
    });
    let mut worn = Vec::new();
    Scene {
        belt: &belt,
        visual: None,
    }
    .append(&s, station(&w), None, &mut worn)
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&intact),
        bytemuck::cast_slice::<_, u8>(&worn)
    );
    let cue = hud_cue(s, &belt, None)
        .unwrap_or_else(|error| panic!("{error}"))
        .unwrap_or_else(|| panic!("front contact"));
    assert_eq!(cue.id, body.id);
    assert_eq!(cue.remaining, body.initial_units);
    assert_eq!(cue.extracted, 0);
    let line = hud_line(s, &belt, None)
        .unwrap_or_else(|error| panic!("{error}"))
        .unwrap_or_else(|| panic!("front contact line"));
    assert!(line.contains(&format!(
        "WORK {:.0}/{:.0}",
        body.remaining_energy, body.energy
    )));
    assert!(line.ends_with("FRONT CONTACT"));
    assert_eq!(
        w.save_bytes().unwrap_or_else(|error| panic!("{error}")),
        before
    );
}

#[test]
fn terminal_flash_ends_at_recorded_contact_and_requires_terminal_quantity() {
    let w = launched();
    let s = w.snapshot();
    let mut belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    let body = &mut belt.bodies[0];
    body.remaining_energy = 0.0;
    body.remaining_units = 0;
    body.extracted_units = body.initial_units;
    let point = body.vertices[0];
    let mut visual = MiningVisual {
        opening: s,
        shot: MiningShot {
            contact: meridian_core::mining::MiningContact {
                asteroid: body.id,
                distance: 1.0,
                point,
            },
            item_id: 1,
            commodity: body.commodity,
            extracted_units: body.initial_units,
            remaining_energy: 0.0,
            tick: s.tick,
        },
    };
    let mut out = Vec::new();
    let mesh_end = VERTEX_COUNT - ASTEROID_FACES.len() * 3;
    Scene {
        belt: &belt,
        visual: Some(visual),
    }
    .append(&s, station(&w), None, &mut out)
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(out.len(), mesh_end + BEAM_VERTEX_COUNT);
    let camera = mesh::scene_camera_pose(&s, station(&w)).unwrap_or_else(|error| panic!("{error}"));
    let end = camera
        .orientation
        .inverse_coordinates(sub(point, camera.position));
    assert_eq!(
        out[mesh_end + 2].position,
        [(end.x + 0.8) as f32, end.y as f32, end.z as f32]
    );

    let existing_vertices = out.len();
    visual.shot.extracted_units = 0;
    assert!(
        Scene {
            belt: &belt,
            visual: Some(visual)
        }
        .append(&s, station(&w), None, &mut out)
        .is_err()
    );
    assert_eq!(out.len(), existing_vertices);
}

#[test]
fn wrong_system_corrupt_depletion_and_nan_geometry_refuse_without_partial_mesh() {
    let w = launched();
    let mut s = w.snapshot();
    let mut belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    let mut out = Vec::new();
    belt.bodies[0].remaining_units += 1;
    assert!(append(&s, station(&w), &belt, None, &mut out).is_err());
    assert!(out.is_empty());
    belt.bodies[0].remaining_units -= 1;
    belt.bodies[0].vertices[0].x = f64::NAN;
    assert!(append(&s, station(&w), &belt, None, &mut out).is_err());
    assert!(out.is_empty());
    if let Some(n) = s.navigation.as_mut() {
        n.system = meridian_core::navigation::SystemId::RENAS;
    }
    assert!(cues(s, &belt).is_err());
}

#[test]
fn descriptive_flash_expires_and_never_changes_world_ore_or_resources() {
    let w = launched();
    let s = w.snapshot();
    let belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    let before = w.save_bytes().unwrap_or_else(|e| panic!("{e}"));
    // Renderer fixture only. This public description cannot mint a paid tool or extract ore.
    let body = belt.bodies[0];
    let visual = MiningVisual {
        opening: s,
        shot: MiningShot {
            contact: meridian_core::mining::MiningContact {
                asteroid: body.id,
                distance: 1.0,
                point: body.vertices[0],
            },
            item_id: 1,
            commodity: body.commodity,
            extracted_units: 0,
            remaining_energy: body.remaining_energy,
            tick: s.tick,
        },
    };
    let mut output = Vec::with_capacity(BEAM_VERTEX_COUNT);
    append_beam(&s, station(&w), &belt, visual, &mut output).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(output.len(), BEAM_VERTEX_COUNT);
    assert!(
        output
            .iter()
            .all(|v| v.position.into_iter().all(f32::is_finite))
    );
    let mut expired = s;
    expired.tick += 5;
    output.clear();
    append_beam(&expired, station(&w), &belt, visual, &mut output)
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(output.is_empty());
    assert_eq!(w.save_bytes().unwrap_or_else(|e| panic!("{e}")), before);
}

#[test]
fn lighting_changes_colour_and_terminal_work_preserves_all_other_body_triangles() {
    let w = launched();
    let s = w.snapshot();
    let mut belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    let mut dark = Vec::new();
    append(&s, station(&w), &belt, None, &mut dark).unwrap_or_else(|e| panic!("{e}"));
    let sun = Vec3::new(4000.0, 10000.0, 6000.0);
    let mut lit = Vec::new();
    append(&s, station(&w), &belt, Some(sun), &mut lit).unwrap_or_else(|e| panic!("{e}"));
    assert!(dark.iter().zip(&lit).any(|(a, b)| a.color != b.color));
    assert!(dark.iter().zip(&lit).all(|(a, b)| a.position == b.position));
    belt.bodies[0].remaining_energy = 0.0;
    belt.bodies[0].remaining_units = 0;
    belt.bodies[0].extracted_units = belt.bodies[0].initial_units;
    let mut depleted = Vec::new();
    append(&s, station(&w), &belt, Some(sun), &mut depleted).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(depleted.len(), VERTEX_COUNT - ASTEROID_FACES.len() * 3);
    assert!(
        lit.iter()
            .skip(ASTEROID_FACES.len() * 3)
            .zip(&depleted)
            .all(|(a, b)| a.position == b.position && a.color == b.color)
    );
}

#[test]
fn later_degenerate_face_restores_the_callers_existing_geometry() {
    let w = launched();
    let s = w.snapshot();
    let mut belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    belt.bodies[11].vertices[0] = belt.bodies[11].vertices[2];
    let sentinel = Vertex {
        position: [1.0, 2.0, 3.0],
        color: [0.4, 0.5, 0.6],
    };
    let mut output = vec![sentinel];
    assert!(append(&s, station(&w), &belt, None, &mut output).is_err());
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].position, sentinel.position);
    assert_eq!(output[0].color, sentinel.color);
}

#[test]
fn docked_and_absent_commander_do_not_create_a_visible_belt() {
    let w = world();
    let s = w.snapshot();
    let belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    let mut output = Vec::new();
    append(&s, station(&w), &belt, None, &mut output).unwrap_or_else(|e| panic!("{e}"));
    assert!(output.is_empty());
    assert!(
        append(
            &meridian_core::World::new().snapshot(),
            station(&w),
            &belt,
            None,
            &mut output
        )
        .is_err()
    );
    assert!(output.is_empty());
}

#[test]
fn cues_use_actual_native_units_and_reject_unrenderable_coordinates() {
    let w = launched();
    let s = w.snapshot();
    let mut belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    for cue in cues(s, &belt).unwrap_or_else(|e| panic!("{e}")) {
        let unit = match cue.commodity.unit() {
            meridian_core::station_services::CargoUnit::Tonne => " t |",
            meridian_core::station_services::CargoUnit::Kilogram => " kg |",
            meridian_core::station_services::CargoUnit::Gram => " g |",
        };
        assert!(cue.line().unwrap_or_else(|e| panic!("{e}")).contains(unit));
    }
    belt.bodies[0].position.x = f64::MAX;
    assert!(cues(s, &belt).is_err());
}

#[test]
fn flash_refuses_foreign_visit_future_tick_and_corrupt_payload_without_appending() {
    let w = launched();
    let s = w.snapshot();
    let belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    let body = belt.bodies[0];
    // Descriptive negative fixture; no authority, extraction or save mutation.
    let visual = MiningVisual {
        opening: s,
        shot: MiningShot {
            contact: meridian_core::mining::MiningContact {
                asteroid: body.id,
                distance: 1.0,
                point: body.vertices[0],
            },
            item_id: 1,
            commodity: body.commodity,
            extracted_units: 0,
            remaining_energy: body.remaining_energy,
            tick: s.tick,
        },
    };
    let mut output = Vec::new();
    let mut foreign = s;
    foreign
        .navigation
        .as_mut()
        .unwrap_or_else(|| panic!("nav"))
        .visit += 1;
    append_beam(&foreign, station(&w), &belt, visual, &mut output)
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(output.is_empty());
    let mut future = visual;
    future.shot.tick += 1;
    future.opening.tick += 1;
    append_beam(&s, station(&w), &belt, future, &mut output).unwrap_or_else(|e| panic!("{e}"));
    assert!(output.is_empty());
    let mut corrupt = visual;
    corrupt.shot.contact.point.x = f64::NAN;
    assert!(append_beam(&s, station(&w), &belt, corrupt, &mut output).is_err());
    assert!(output.is_empty());
}

#[test]
fn chart_address_selects_the_same_actual_rock_without_fabricating_centre_or_foreign_ids() {
    let w = launched();
    let s = w.snapshot();
    let belt = w.mining_belt_snapshot().unwrap_or_else(|| panic!("belt"));
    let body = belt.bodies[7];
    let selected = Some((body.id.system(), u64::from(body.id.ordinal())));
    let cue = hud_cue(s, &belt, selected)
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|| panic!("cue"));
    assert_eq!(cue.id, body.id);
    assert_eq!(cue.position, body.position);
    assert_eq!(cue.remaining, body.remaining_units);
    // The launch fixture has no forward rock contact; the belt centre remains navigation-only.
    assert!(belt.front_contact.is_none());
    assert!(
        hud_cue(s, &belt, Some((body.id.system(), u64::MAX)))
            .unwrap_or_else(|e| panic!("{e}"))
            .is_none()
    );
    assert!(
        hud_cue(
            s,
            &belt,
            Some((meridian_core::navigation::SystemId::RENAS, 7))
        )
        .unwrap_or_else(|e| panic!("{e}"))
        .is_none()
    );
}
