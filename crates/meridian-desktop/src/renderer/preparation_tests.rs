//! Exercise actual HUD preparation; no window, queue or GPU submission is fabricated.
use super::{PendingProjection, Renderer, SurfaceRecovery, flight_projection};
use crate::{hud, mesh::Vertex};

fn stale_buffer() -> Vec<Vertex> {
    vec![
        Vertex {
            position: [99.0; 3],
            color: [99.0; 3]
        };
        6
    ]
}

fn same_vertices(actual: &[Vertex], expected: &[Vertex]) {
    assert_eq!(
        bytemuck::cast_slice::<Vertex, u8>(actual),
        bytemuck::cast_slice::<Vertex, u8>(expected)
    );
}

fn cockpit() -> hud::cockpit::Cockpit {
    let world = meridian_core::World::new_commander(90531).unwrap_or_else(|e| panic!("{e:?}"));
    hud::cockpit::Cockpit::capture(&world, world.snapshot()).unwrap_or_else(|e| panic!("{e:?}"))
}

#[test]
fn hud_adapter_submits_actual_cockpit_bytes_after_one_scale_query() {
    let cockpit = cockpit();
    let lines = vec!["DOCKED".into()];
    let events = std::cell::RefCell::new(Vec::new());
    let mut actual = stale_buffer();
    let mut submitted = Vec::new();
    Renderer::prepare_and_submit_hud(
        &lines,
        Some(&cockpit),
        winit::dpi::PhysicalSize::new(1280, 720),
        [1280, 720],
        || {
            events.borrow_mut().push("scale");
            2.0
        },
        &mut actual,
        |bytes| {
            events.borrow_mut().push("submit");
            submitted.extend_from_slice(bytes);
        },
    );
    assert_eq!(*events.borrow(), ["scale", "submit"]);
    let mut expected = Vec::new();
    hud::vertices_with_cockpit(&lines, 640, 360, Some(&cockpit), &mut expected);
    same_vertices(&actual, &expected);
    assert_eq!(submitted, bytemuck::cast_slice::<Vertex, u8>(&expected));
    assert!(actual.iter().all(|v| v.color != [99.0; 3]));
}

#[test]
fn hud_adapter_submits_configured_text_viewport_without_querying_scale() {
    let lines = vec!["NO SAVED COMMANDER".into()];
    let mut actual = stale_buffer();
    let mut calls = 0;
    let mut submitted = Vec::new();
    Renderer::prepare_and_submit_hud(
        &lines,
        None,
        winit::dpi::PhysicalSize::new(1920, 1080),
        [640, 360],
        || panic!("text upload queried the window scale"),
        &mut actual,
        |bytes| {
            calls += 1;
            submitted.extend_from_slice(bytes);
        },
    );
    assert_eq!(calls, 1);
    let mut expected = Vec::new();
    hud::vertices(&lines, 640, 360, &mut expected);
    same_vertices(&actual, &expected);
    assert_eq!(submitted, bytemuck::cast_slice::<Vertex, u8>(&expected));
}

#[test]
fn hud_adapter_submits_empty_bytes_once_after_clearing_a_zero_viewport() {
    let lines = vec!["DOCKED".into()];
    let mut actual = stale_buffer();
    let mut calls = 0;
    Renderer::prepare_and_submit_hud(
        &lines,
        None,
        winit::dpi::PhysicalSize::new(1280, 720),
        [0, 360],
        || panic!("text upload queried the window scale"),
        &mut actual,
        |bytes| {
            calls += 1;
            assert!(bytes.is_empty());
        },
    );
    assert_eq!(calls, 1);
    assert!(actual.is_empty());
}

#[test]
fn text_hud_uses_physical_viewport_without_querying_the_window() {
    let lines = vec!["NO SAVED COMMANDER".into(), "N NEW COMMANDER".into()];
    let mut actual = stale_buffer();
    Renderer::prepare_hud_frame(
        &lines,
        None,
        || panic!("text path queried logical viewport"),
        [640, 360],
        &mut actual,
    );
    let mut expected = Vec::new();
    hud::vertices(&lines, 640, 360, &mut expected);
    same_vertices(&actual, &expected);
    assert!(actual.len() > 6);
    assert!(actual.iter().all(|v| {
        v.position
            .iter()
            .all(|n| n.is_finite() && (-1.0..=1.0).contains(n))
    }));
    assert!(actual.iter().all(|v| v.color != [99.0; 3]));
}

#[test]
fn cockpit_hud_uses_logical_retina_dimensions_and_the_actual_snapshot() {
    let cockpit = cockpit();
    let lines = vec!["DOCKED".into()];
    let mut calls = 0;
    let mut actual = stale_buffer();
    Renderer::prepare_hud_frame(
        &lines,
        Some(&cockpit),
        || {
            calls += 1;
            [640.0, 360.0]
        },
        [1280, 720],
        &mut actual,
    );
    assert_eq!(calls, 1);
    let mut expected = Vec::new();
    hud::vertices_with_cockpit(&lines, 640, 360, Some(&cockpit), &mut expected);
    same_vertices(&actual, &expected);
    assert!(actual.iter().any(|v| v.color == [0.0, 1.0, 0.0]));
    let mut physical = Vec::new();
    hud::vertices_with_cockpit(&lines, 1280, 720, Some(&cockpit), &mut physical);
    assert_ne!(
        bytemuck::cast_slice::<Vertex, u8>(&actual),
        bytemuck::cast_slice::<Vertex, u8>(&physical)
    );
}

#[test]
fn fractional_logical_dimensions_round_before_cockpit_layout() {
    let cockpit = cockpit();
    let lines = vec!["FUEL".into()];
    let mut actual = Vec::new();
    Renderer::prepare_hud_frame(
        &lines,
        Some(&cockpit),
        || [639.6, 359.6],
        [1279, 719],
        &mut actual,
    );
    let mut expected = Vec::new();
    hud::vertices_with_cockpit(&lines, 640, 360, Some(&cockpit), &mut expected);
    same_vertices(&actual, &expected);
}

#[test]
fn zero_viewports_clear_reused_buffers_in_both_hud_paths() {
    let cockpit = cockpit();
    let lines = vec!["DOCKED".into()];
    let mut actual = stale_buffer();
    Renderer::prepare_hud_frame(
        &lines,
        None,
        || panic!("text path queried window"),
        [0, 720],
        &mut actual,
    );
    assert!(actual.is_empty());
    actual = stale_buffer();
    Renderer::prepare_hud_frame(
        &lines,
        Some(&cockpit),
        || [640.0, 0.0],
        [1280, 720],
        &mut actual,
    );
    assert!(actual.is_empty());
}

#[test]
fn repeated_surface_skips_never_prepare_or_accumulate_frame_uploads() {
    use wgpu::CurrentSurfaceTexture::{Lost, Occluded, Outdated, Timeout};
    let mut uploads = 0;
    let mut recoveries = Vec::new();
    // Concrete wgpu skip statuses, but no interactive surface, GPU or queue is created.
    for _ in 0..256 {
        for status in [Timeout, Occluded, Outdated, Lost] {
            let acquired = Renderer::resolve_surface_acquisition(status, |recovery| {
                recoveries.push(recovery);
                Ok(())
            });
            let result = Renderer::prepare_acquired_frame(acquired, || {
                uploads += 1;
                Ok(())
            });
            assert!(matches!(result, Ok(None)));
            assert_eq!(uploads, 0);
        }
    }
    assert_eq!(recoveries.len(), 512);
    let (pairs, trailing) = recoveries.as_chunks::<2>();
    assert!(trailing.is_empty());
    assert!(
        pairs
            .iter()
            .all(|pair| *pair == [SurfaceRecovery::Configure, SurfaceRecovery::Recreate])
    );

    // A later successful normal or suboptimal frame runs the exact prepare callback once.
    for token in [(7, false), (8, true)] {
        let prepared = Renderer::prepare_acquired_frame(Ok(Some(token)), || {
            uploads += 1;
            Ok(())
        });
        assert_eq!(prepared, Ok(Some(token)));
    }
    assert_eq!(uploads, 2);
}

#[test]
fn surface_validation_or_failed_recovery_never_runs_uploads() {
    use wgpu::CurrentSurfaceTexture::{Lost, Outdated, Validation};
    let validation =
        Renderer::resolve_surface_acquisition(Validation, |_| panic!("validation cannot recover"));
    assert_eq!(
        Renderer::prepare_acquired_frame(validation, || panic!("validation cannot stage data"))
            .err()
            .as_deref(),
        Some("Surface validation failed")
    );
    for (status, expected) in [
        (Outdated, SurfaceRecovery::Configure),
        (Lost, SurfaceRecovery::Recreate),
    ] {
        let acquired = Renderer::resolve_surface_acquisition(status, |recovery| {
            assert_eq!(recovery, expected);
            Err("recover failed".into())
        });
        assert_eq!(
            Renderer::prepare_acquired_frame(acquired, || panic!(
                "failed recovery cannot stage data"
            ))
            .err()
            .as_deref(),
            Some("recover failed")
        );
    }
    let failure = Renderer::prepare_acquired_frame(Ok(Some(1)), || Err("prepare refused".into()));
    assert_eq!(failure, Err("prepare refused".into()));
}

#[test]
fn resize_projection_coalesces_through_skipped_frames_and_uploads_latest_once() {
    use wgpu::CurrentSurfaceTexture::{Occluded, Timeout};
    let mut pending = PendingProjection::default();
    let mut writes = Vec::new();
    for width in 640..896 {
        pending.resize(winit::dpi::PhysicalSize::new(width, 720));
        for status in [Occluded, Timeout] {
            let acquired = Renderer::resolve_surface_acquisition(status, |_| panic!("no recovery"));
            let prepared = Renderer::prepare_acquired_frame(acquired, || {
                pending.upload(|projection| writes.push(projection));
                Ok(())
            });
            assert!(matches!(prepared, Ok(None)));
        }
        assert!(writes.is_empty());
    }
    pending.resize(winit::dpi::PhysicalSize::new(0, 0));
    let prepared = Renderer::prepare_acquired_frame(Ok(Some(1)), || {
        pending.upload(|projection| writes.push(projection));
        Ok(())
    });
    assert_eq!(prepared, Ok(Some(1)));
    assert_eq!(writes, [flight_projection(895, 720)]);
    let prepared = Renderer::prepare_acquired_frame(Ok(Some(2)), || {
        pending.upload(|projection| writes.push(projection));
        Ok(())
    });
    assert_eq!(prepared, Ok(Some(2)));
    assert_eq!(writes, [flight_projection(895, 720)]);
}
