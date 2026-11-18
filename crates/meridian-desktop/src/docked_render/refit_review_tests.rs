//! Public paid/physical-berth controls through the CPU render boundary, not native GPU evidence.
use super::*;
use crate::{
    docked::Docked,
    docked_screens as ui,
    profile::{
        Profile,
        tests::{TestDirectory, open},
    },
    session::Action,
};
use meridian_core::{
    FlightInput, FlightMode, FlightView, LaserFacing, ModuleAction, ModuleKind, ModuleProvenance,
    ModuleSlot, World, XyzzyAction,
    navigation::{DockOrigin, SystemId},
};
fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("actual public refusal: {e:?}"))
}
fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("actual state/control missing"))
}
fn step(world: &mut World, input: FlightInput) {
    ok(world.step(input));
    assert_eq!(world.snapshot().motion_refusal, None);
}
fn physical_berth(world: &mut World) {
    ok(world.request_launch(world.snapshot().tick));
    step(world, FlightInput::default());
    for _ in 0..300 {
        if world.snapshot().speed == 0.0 {
            break;
        }
        step(
            world,
            FlightInput {
                brake: true,
                ..FlightInput::default()
            },
        );
    }
    assert_eq!(world.snapshot().speed, 0.0);
    // Source launch is +Z; public yaw half-turn retains upY. No pose/arrival fixture.
    let ticks = (core::f32::consts::PI / (0.65 / 60.0)).ceil() as u16;
    for _ in 0..ticks {
        step(
            world,
            FlightInput {
                yaw: core::f32::consts::PI / (0.65 / 60.0) / f32::from(ticks),
                ..FlightInput::default()
            },
        );
    }
    for _ in 0..9000 {
        let snapshot = world.snapshot();
        if snapshot.mode == FlightMode::Docked {
            let dock = some(world.current_dock_context());
            assert_eq!(dock.system(), SystemId::AVENOR);
            assert_eq!(dock.origin(), DockOrigin::PhysicalEntry);
            return;
        }
        step(
            world,
            FlightInput {
                throttle: ((4.0 - snapshot.speed) / (9.1 / 60.0)).clamp(-1.0, 1.0),
                ..FlightInput::default()
            },
        );
    }
    panic!("genuine public return did not reach the Avenor berth");
}
fn paint(menu: &mut Docked, profile: &Profile, viewport: ui::Viewport) -> Frame {
    let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
    // CPU harness acknowledgement only; this does not claim native presentation.
    menu.presented(frame.clone());
    frame
}
fn assert_review(frame: &Frame, model: &Model) {
    for expected in [
        "Equipment review",
        "Beam Laser",
        "Owned item; no charge for fitting or storing",
        "Confirm publishes the complete equipment change",
        "Confirm",
        "Cancel",
    ] {
        assert!(
            frame.labels.iter().any(|label| label.text == expected),
            "missing {expected}: {:?}",
            frame.labels
        );
    }
    assert!(
        frame
            .hits
            .iter()
            .any(|hit| hit.enabled && hit.control == ui::Control::ReviewConfirm)
    );
    assert!(
        frame
            .hits
            .iter()
            .any(|hit| hit.enabled && hit.control == ui::Control::ReviewCancel)
    );
    let mut geometry = Geometry::with_capacity();
    ok(geometry.compose(frame, model, true, None));
    assert!(!geometry.shapes.is_empty() && !geometry.text.is_empty());
    assert!(geometry.shapes.len() <= MAX_GEOMETRY && geometry.text.len() <= MAX_GLYPHS * 6);
    assert!(geometry.models.is_empty());
    assert!(
        geometry
            .shapes
            .iter()
            .all(|v| { v.position.into_iter().chain(v.color).all(f32::is_finite) })
    );
    let (vertices, bytes, fingerprint) = geometry.capture_evidence();
    assert_eq!(
        vertices,
        [
            geometry.shapes.len(),
            geometry.text.len(),
            geometry.models.len()
        ]
    );
    let buffers: [&[u8]; 3] = [
        bytemuck::cast_slice(&geometry.shapes),
        bytemuck::cast_slice(&geometry.text),
        bytemuck::cast_slice(&geometry.models),
    ];
    assert_eq!(bytes, buffers.map(|b| b.len()));
    assert_eq!(fingerprint, crate::review_capture::digest(&buffers));
    assert!(bytes[0] > 0 && bytes[1] > 0 && bytes[2] == 0);
    let atlas = atlas();
    let mut visible = 0;
    for glyph in geometry.text.chunks_exact(6) {
        assert!(glyph.iter().all(|v| {
            v.position
                .into_iter()
                .chain(v.uv)
                .chain(v.colour)
                .all(f32::is_finite)
        }));
        let low = core::array::from_fn::<_, 2, _>(|i| {
            glyph.iter().map(|v| v.uv[i]).fold(f32::INFINITY, f32::min)
        });
        let high = core::array::from_fn::<_, 2, _>(|i| {
            glyph
                .iter()
                .map(|v| v.uv[i])
                .fold(f32::NEG_INFINITY, f32::max)
        });
        for y in 0..56 {
            for x in 0..80 {
                let uv = [(x as f32 + 0.5) / 80.0, (y as f32 + 0.5) / 56.0];
                if uv[0] >= low[0]
                    && uv[0] < high[0]
                    && uv[1] >= low[1]
                    && uv[1] < high[1]
                    && atlas[y * 80 + x] != 0
                    && glyph[0].colour.iter().any(|v| *v > 0.1)
                {
                    visible += 1;
                }
            }
        }
    }
    assert!(
        visible > 0,
        "review has no visible atlas samples despite metadata: {:?}",
        frame.labels
    );
}
#[test]
fn actual_paid_beam_store_then_immediate_fit_keeps_visible_review_controls_and_cpu_geometry_at_a_physical_berth()
 {
    let directory = TestDirectory::new();
    let mut profile = open(&directory.0);
    assert!(ok(profile.new_commander(29401)));
    // The same explicitly labelled public testing setup as native; no imported player profile.
    ok(profile.xyzzy(XyzzyAction::Enter));
    ok(profile.xyzzy(XyzzyAction::ToggleUnlimitedFuel));
    let scoop = some(
        some(profile.world.modular_equipment_snapshot())
            .items
            .into_iter()
            .find(|i| i.slot == Some(ModuleSlot::UtilityFuelScoops)),
    )
    .id;
    ok(profile.command(Action::Module(ModuleAction::Store { item: scoop })));
    for (kind, facing) in [
        (ModuleKind::BeamLaser, LaserFacing::Front),
        (ModuleKind::PulseLaser, LaserFacing::Left),
        (ModuleKind::PulseLaser, LaserFacing::Right),
        (ModuleKind::PulseLaser, LaserFacing::Rear),
    ] {
        ok(profile.command(Action::Module(ModuleAction::PurchaseLaser { kind, facing })));
    }
    ok(profile.command(Action::Module(ModuleAction::Purchase {
        kind: ModuleKind::EcmSystem,
    })));
    assert_eq!(profile.world.credits_tenths(), Some(9888500));
    let original = some(profile.world.modular_equipment_snapshot());
    let beam = some(
        original
            .items
            .iter()
            .find(|i| i.kind == ModuleKind::BeamLaser),
    )
    .id;
    let paid: Vec<_> = original
        .items
        .iter()
        .filter(|i| matches!(i.provenance, ModuleProvenance::Paid { .. }))
        .map(|i| (i.id, i.kind, i.provenance))
        .collect();
    assert_eq!(paid.len(), 5);
    physical_berth(&mut profile.world);
    ok(profile.command(Action::SetFlightView(FlightView::Front)));
    ok(profile.save());
    let resources = profile.world.flight_resources_snapshot();
    let mut menu = ok(Docked::new(&profile.world));
    ok(menu.open(&profile.world, ui::Screen::Equipment));
    let views = [
        some(ui::Viewport::new(640.0, 360.0)),
        some(ui::Viewport::new(1280.0, 720.0)),
        some(ui::Viewport::new(1800.0, 1125.0)),
        some(ui::Viewport::new(3600.0, 2250.0)),
    ];
    paint(&mut menu, &profile, views[0]);
    for _ in 0..2 {
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Next)));
        paint(&mut menu, &profile, views[0]);
    }
    let frame = paint(&mut menu, &profile, views[0]);
    assert!(frame.labels.iter().any(|l| l.text == "> Beam Laser"));
    let before = ok(profile.world.save_bytes());
    let key = some(crate::docked::key(
        winit::keyboard::KeyCode::KeyU,
        false,
        ui::Screen::Equipment,
    ));
    assert_eq!(key, ui::Key::Store);
    ok(menu.input(&mut profile, ui::Input::Key(key)));
    let model = ok(Model::sh025());
    let mut stored_reviews = Vec::new();
    for viewport in views {
        let frame = paint(&mut menu, &profile, viewport);
        assert_review(&frame, &model);
        let identity = some(menu.review_capture_identity());
        assert_eq!(identity.operation, "Store");
        assert_eq!(identity.item, beam);
        crate::review_capture::tests::assert_actual_frame_record(&frame, identity);
        stored_reviews.push(frame);
    }
    assert_eq!(ok(profile.world.save_bytes()), before);
    // The native sequence used Enter then immediate Y, without an Equipment repaint.
    let enter = some(crate::docked::key(
        winit::keyboard::KeyCode::Enter,
        false,
        ui::Screen::Equipment,
    ));
    assert_eq!(enter, ui::Key::Confirm);
    ok(menu.input(&mut profile, ui::Input::Key(enter)));
    let stored = ok(profile.world.save_bytes());
    assert_eq!(
        ok(std::fs::read(directory.0.join("commander.meridian"))),
        stored
    );
    assert_eq!(
        some(
            some(profile.world.modular_equipment_snapshot())
                .items
                .into_iter()
                .find(|i| i.id == beam)
        )
        .slot,
        None
    );
    let key = some(crate::docked::key(
        winit::keyboard::KeyCode::KeyY,
        false,
        ui::Screen::Equipment,
    ));
    assert_eq!(key, ui::Key::Fit);
    ok(menu.input(&mut profile, ui::Input::Key(key)));
    assert_eq!(ok(profile.world.save_bytes()), stored);
    // The next normal build opens Fit, without an intervening Equipment frame or acknowledgement.
    let candidate = ok(menu.build_frame(&profile.world, views[0], 0.0));
    assert_review(&candidate, &model);
    // A built-but-unpresented Fit cannot authorize Enter or an old Store Confirm hit.
    ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
    let old_confirm = some(stored_reviews.last().and_then(|f| {
        f.hits
            .iter()
            .find(|h| h.control == ui::Control::ReviewConfirm)
    }))
    .bounds
    .centre();
    ok(menu.input(&mut profile, ui::Input::Click(old_confirm)));
    assert_eq!(ok(profile.world.save_bytes()), stored);
    assert_eq!(
        ok(std::fs::read(directory.0.join("commander.meridian"))),
        stored
    );
    menu.presented(candidate);
    for (viewport, store) in views.into_iter().zip(&stored_reviews) {
        let fit = paint(&mut menu, &profile, viewport);
        assert_review(&fit, &model);
        let identity = some(menu.review_capture_identity());
        assert_eq!(identity.operation, "Fit");
        assert_eq!(identity.item, beam);
        assert_eq!(identity.facing, Some(LaserFacing::Front));
        crate::review_capture::tests::assert_actual_frame_record(&fit, identity);
        assert_eq!(fit.labels, store.labels);
        assert_eq!(fit.lines, store.lines);
        assert_eq!(fit.hits, store.hits);
    }
    assert_eq!(ok(profile.world.save_bytes()), stored);
    ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Back)));
    let frame = paint(&mut menu, &profile, views[0]);
    assert!(frame.labels.iter().any(|l| l.text == "> Beam Laser"));
    assert_eq!(ok(profile.world.save_bytes()), stored);
    // A new visible test review may confirm; this is authored CPU/host coverage, not native cure.
    ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Fit)));
    let frame = paint(&mut menu, &profile, views[0]);
    assert_review(&frame, &model);
    let confirm = some(
        frame
            .hits
            .iter()
            .find(|h| h.control == ui::Control::ReviewConfirm),
    )
    .bounds
    .centre();
    ok(menu.input(&mut profile, ui::Input::Click(confirm)));
    assert_eq!(profile.world.credits_tenths(), Some(9888500));
    assert_eq!(profile.world.flight_resources_snapshot(), resources);
    let inventory = some(profile.world.modular_equipment_snapshot());
    assert_eq!(inventory.items.len(), original.items.len());
    assert_eq!(
        some(inventory.items.iter().find(|i| i.id == beam)).slot,
        Some(ModuleSlot::FrontLaser)
    );
    assert_eq!(
        inventory
            .items
            .iter()
            .filter(|i| matches!(i.provenance, ModuleProvenance::Paid { .. }))
            .map(|i| (i.id, i.kind, i.provenance))
            .collect::<Vec<_>>(),
        paid
    );
    let bytes = ok(profile.world.save_bytes());
    assert_eq!(
        ok(std::fs::read(directory.0.join("commander.meridian"))),
        bytes
    );
    let owner = some(profile.world.commander_id()).seed();
    let expected_continue = super::continue_oracle::Expected::from_original(&bytes, owner);
    // Crosscheck public loader against the independent format/counter/checksum oracle.
    let public_continue = ok(ok(World::from_save_bytes(&bytes)).save_bytes());
    assert_eq!(public_continue, expected_continue.bytes);
    drop(profile);
    let mut cold = open(&directory.0);
    ok(cold.continue_saved());
    assert_eq!(cold.world.modular_equipment_snapshot(), Some(inventory));
    assert_eq!(cold.world.flight_view(), FlightView::Front);
    assert_eq!(cold.world.credits_tenths(), Some(9888500));
    assert_eq!(cold.world.flight_resources_snapshot(), resources);
    // Continue revokes old review permissions. The original raw-file equality predicate
    // and its actual failure remain in the sealed 7661 evidence, not as a false invariant.
    let continued_bytes = ok(cold.world.save_bytes());
    expected_continue.assert_actual(&bytes, &continued_bytes);
    assert_eq!(continued_bytes, expected_continue.bytes);
    assert_eq!(
        ok(std::fs::read(directory.0.join("commander.meridian"))),
        bytes
    );
}
