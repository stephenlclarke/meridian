//! Exercise actual HUD preparation; no window, queue or GPU submission is fabricated.
use super::Renderer;
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
