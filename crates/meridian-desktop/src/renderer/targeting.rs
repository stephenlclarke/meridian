//! The scanner chooses contacts in World; the overlay projects the selected contact.
use crate::{
    hud,
    mesh::{self, Vertex},
};
use meridian_core::docking::{RigidPose, Vec3};
use meridian_core::{FlightView, TargetingSnapshot};

const CYAN: [f32; 3] = [0.0, 1.0, 1.0];
const MAX_ROWS: usize = 16;
const MAX_COLUMNS: usize = 30;
const MAX_PREVIEW_TRIANGLES: usize = 24_000;
pub(super) const MAX_VERTICES: usize =
    MAX_ROWS * MAX_COLUMNS * 35 * 6 + MAX_PREVIEW_TRIANGLES * 3 + 60;

fn relative(camera: RigidPose, pose: RigidPose, source: [f32; 3]) -> [f32; 3] {
    let point = pose.orientation.apply(Vec3::new(
        source[0].into(),
        source[1].into(),
        source[2].into(),
    ));
    let difference = Vec3::new(
        pose.position.x + point.x - camera.position.x,
        pose.position.y + point.y - camera.position.y,
        pose.position.z + point.z - camera.position.z,
    );
    let local = camera.orientation.inverse_coordinates(difference);
    [local.x as f32, local.y as f32, local.z as f32]
}

/// The same physical asteroid corners and face ordering as the ordinary belt renderer.
pub(super) fn asteroid_model(body: &meridian_core::mining::AsteroidSnapshot) -> Vec<Vertex> {
    mineral_model(body.vertices, body.position)
}

pub(super) fn ore_model(ore: &meridian_core::mining::OreSnapshot) -> Vec<Vertex> {
    mineral_model(ore.vertices(), ore.position)
}

fn mineral_model(
    vertices: [meridian_core::docking::Vec3; 6],
    centre: meridian_core::docking::Vec3,
) -> Vec<Vertex> {
    meridian_core::mining::ASTEROID_FACES
        .into_iter()
        .flatten()
        .map(|index| {
            let p = vertices[index];
            Vertex {
                position: [
                    (p.x - centre.x) as f32,
                    (p.y - centre.y) as f32,
                    (p.z - centre.z) as f32,
                ],
                color: [0.31, 0.23, 0.14],
            }
        })
        .collect()
}

fn rect(out: &mut Vec<Vertex>, bounds: [f32; 4], color: [f32; 3], size: [f32; 2]) {
    let [l, t, r, b] = bounds;
    let vertex = |x, y| Vertex {
        position: [2.0 * x / size[0] - 1.0, 1.0 - 2.0 * y / size[1], 0.0],
        color,
    };
    out.extend([
        vertex(l, t),
        vertex(l, b),
        vertex(r, t),
        vertex(r, t),
        vertex(l, b),
        vertex(r, b),
    ]);
}

fn text(out: &mut Vec<Vertex>, value: &str, start: [f32; 2], size: [f32; 2]) {
    for (column, c) in value.chars().take(MAX_COLUMNS).enumerate() {
        for (row, bits) in hud::glyph(c).into_iter().enumerate() {
            for bit in 0..5 {
                if bits & (1 << (4 - bit)) != 0 {
                    let x = start[0] + (column * 6 + bit) as f32;
                    let y = start[1] + row as f32;
                    rect(out, [x, y, x + 1.0, y + 1.0], CYAN, size);
                }
            }
        }
    }
}

fn marker(
    out: &mut Vec<Vertex>,
    size: [f32; 2],
    camera: RigidPose,
    view: FlightView,
    target: RigidPose,
    projection: [f32; 4],
) {
    let mut point = [Vertex {
        position: relative(camera, target, [0.0; 3]),
        color: CYAN,
    }];
    mesh::apply_flight_view(view, &mut point);
    let [x, y, z] = point[0].position;
    if ![x, y, z].into_iter().all(f32::is_finite) {
        return;
    }
    let half = [size[0] * 0.5, size[1] * 0.5];
    let [aspect, focal, near, far] = projection;
    if ![aspect, focal, near, far]
        .into_iter()
        .all(|v| v.is_finite() && v > 0.0)
    {
        return;
    }
    let mut screen = if z > near {
        [x * half[0] * focal / (aspect * z), -y * half[1] * focal / z]
    } else if x.abs() + y.abs() > f32::EPSILON {
        [x, -y]
    } else {
        [0.0, half[1]]
    };
    let edge = (screen[0].abs() / (half[0] - 10.0)).max(screen[1].abs() / (half[1] - 10.0));
    let offscreen = z <= near || z > far || edge > 1.0;
    if offscreen && edge > 0.0 {
        screen = [screen[0] / edge, screen[1] / edge];
    }
    let [x, y] = [half[0] + screen[0], half[1] + screen[1]];
    for bounds in [
        [x - 8.0, y - 8.0, x + 8.0, y - 6.0],
        [x - 8.0, y + 6.0, x + 8.0, y + 8.0],
        [x - 8.0, y - 6.0, x - 6.0, y + 6.0],
        [x + 6.0, y - 6.0, x + 8.0, y + 6.0],
    ] {
        rect(out, bounds, CYAN, size);
    }
    if offscreen {
        rect(out, [x - 2.0, y - 2.0, x + 2.0, y + 2.0], CYAN, size);
    }
}

fn preview(
    out: &mut Vec<Vertex>,
    mesh: &[Vertex],
    camera: RigidPose,
    target: RigidPose,
    center: [f32; 2],
    size: [f32; 2],
) {
    let transformed: Vec<_> = mesh
        .iter()
        .take(MAX_PREVIEW_TRIANGLES * 3)
        .map(|vertex| {
            let rotated = target.orientation.apply(Vec3::new(
                vertex.position[0].into(),
                vertex.position[1].into(),
                vertex.position[2].into(),
            ));
            let p = camera.orientation.inverse_coordinates(rotated);
            ([p.x as f32, p.y as f32, p.z as f32], vertex.color)
        })
        .collect();
    let radius = transformed
        .iter()
        .map(|(p, _)| p[0].abs().max(p[1].abs()))
        .fold(0.0f32, f32::max);
    if !radius.is_finite() || radius <= 0.0 {
        return;
    }
    let mut triangles: Vec<_> = transformed.as_chunks::<3>().0.iter().collect();
    let depth = |triangle: &[([f32; 3], [f32; 3])]| triangle.iter().map(|(p, _)| p[2]).sum::<f32>();
    triangles.sort_by(|a, b| depth(b.as_slice()).total_cmp(&depth(a.as_slice())));
    for triangle in triangles {
        for &(p, color) in triangle {
            let x = center[0] + p[0] / radius * 22.0;
            let y = center[1] - p[1] / radius * 22.0;
            out.push(Vertex {
                position: [2.0 * x / size[0] - 1.0, 1.0 - 2.0 * y / size[1], 0.0],
                color,
            });
        }
    }
}

pub(super) struct Overlay<'a> {
    pub target: &'a TargetingSnapshot,
    pub camera: RigidPose,
    pub view: FlightView,
    pub projection: [f32; 4],
    pub details: Option<bool>,
    pub model: &'a [Vertex],
}

pub(super) fn append(out: &mut Vec<Vertex>, size: [u32; 2], overlay: Overlay<'_>) {
    if size[0] < 64 || size[1] < 64 {
        return;
    }
    let expanded = overlay.details.unwrap_or(size[0] >= 1100);
    let size = size.map(|v| v as f32);
    marker(
        out,
        size,
        overlay.camera,
        overlay.view,
        overlay.target.pose,
        overlay.projection,
    );
    if size[0] < 220.0 {
        return;
    }
    let right = size[0] - 8.0;
    let left = right - 204.0;
    let top = 8.0;
    let mut lines = vec![
        overlay.target.identity.label(),
        overlay.target.guidance.to_owned(),
        overlay.target.tier.map_or_else(
            || "NO TARGETING CORE".into(),
            |tier| format!("{} CORE", tier.label()),
        ),
    ];
    if expanded {
        if let Some(model) = overlay.target.model {
            lines.push(model.into());
        }
        if let Some(status) = overlay
            .target
            .status
            .filter(|status| *status != overlay.target.guidance)
        {
            lines.push(status.into());
        }
        lines.extend(
            overlay
                .target
                .fields
                .iter()
                .map(|(key, value)| format!("{key}: {value}")),
        );
    }
    let preview_height = if expanded
        && overlay.target.model.is_some()
        && !overlay.model.is_empty()
        && size[1] > 200.0
    {
        50.0
    } else {
        0.0
    };
    let count = lines
        .len()
        .min(MAX_ROWS)
        .min(((size[1] - 30.0 - preview_height) / 9.0) as usize);
    rect(
        out,
        [
            left,
            top,
            right,
            top + 8.0 + count as f32 * 9.0 + preview_height,
        ],
        [0.006, 0.012, 0.025],
        size,
    );
    if preview_height > 0.0 {
        preview(
            out,
            overlay.model,
            overlay.camera,
            overlay.target.pose,
            [left + 102.0, top + 25.0],
            size,
        );
    }
    for (index, line) in lines.iter().take(count).enumerate() {
        text(
            out,
            line,
            [left + 6.0, top + 4.0 + preview_height + index as f32 * 9.0],
            size,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_core::docking::Basis3;
    use meridian_core::{
        TargetIdentity, TargetMode, equipment::TargetingTier, navigation::SystemId,
    };
    #[test]
    fn cue_uses_supplied_projection_and_tiny_drawable_produces_no_vertices() {
        let camera = RigidPose {
            position: Vec3::new(0.0, 0.0, 0.0),
            orientation: Basis3::IDENTITY,
        };
        let pose = RigidPose {
            position: Vec3::new(1.0, 0.0, 10.0),
            ..camera
        };
        let target = TargetingSnapshot {
            tracked_fragment: None,
            identity: TargetIdentity::Station {
                system: SystemId::AVENOR,
                visit: 1,
            },
            tier: Some(TargetingTier::Docking),
            filter: meridian_core::TargetFilter::Ships,
            mode: TargetMode::Station,
            guidance: "Approach",
            pose,
            model: None,
            status: None,
            fields: vec![],
        };
        let projection = [2.0, 2.0, 1.0, 100.0];
        let mut out = Vec::new();
        let overlay = || Overlay {
            target: &target,
            camera,
            view: FlightView::Front,
            projection,
            details: Some(false),
            model: &[],
        };
        append(&mut out, [1, 200], overlay());
        append(&mut out, [200, 1], overlay());
        assert!(out.is_empty());
        marker(
            &mut out,
            [400.0, 200.0],
            camera,
            FlightView::Front,
            pose,
            projection,
        );
        assert_eq!(out.len(), 24);
        let min = out
            .iter()
            .map(|v| v.position[0])
            .fold(f32::INFINITY, f32::min);
        let max = out
            .iter()
            .map(|v| v.position[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(((min + max) * 0.5 - 0.1).abs() < 0.0001);
        out.clear();
        marker(
            &mut out,
            [400.0, 200.0],
            camera,
            FlightView::Rear,
            pose,
            projection,
        );
        assert_eq!(out.len(), 30);
        assert!(
            out.iter()
                .all(|v| v.position.iter().all(|p| p.is_finite() && p.abs() <= 1.0))
        );
        out.clear();
        append(&mut out, [640, 360], overlay());
        assert!(out.len() > 24 && out.len() <= MAX_VERTICES);
    }
}
