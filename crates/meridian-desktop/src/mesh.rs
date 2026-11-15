use meridian_core::Snapshot;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

pub fn load_ship() -> Result<Vec<Vertex>, String> {
    decode_mesh(include_bytes!("../../../assets/legacy/sh025.mesh"))
}

fn decode_mesh(bytes: &[u8]) -> Result<Vec<Vertex>, String> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(72) {
        return Err("Imported ship must contain complete 24-byte triangle vertices".into());
    }
    bytes
        .as_chunks::<24>()
        .0
        .iter()
        .map(|vertex| {
            let mut values = [0.0; 6];
            for (value, chunk) in values.iter_mut().zip(vertex.as_chunks::<4>().0.iter()) {
                *value = f32::from_le_bytes(*chunk);
            }
            if values.iter().any(|value| !value.is_finite()) {
                return Err("Imported ship has non-finite vertex data".into());
            }
            Ok(Vertex {
                position: [values[0], values[1], values[2]],
                color: [values[3], values[4], values[5]],
            })
        })
        .collect()
}

fn rotate_inverse(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let u = [-q[0], -q[1], -q[2]];
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let t = cross(u, v).map(|x| 2.0 * x);
    let c = cross(u, t);
    std::array::from_fn(|i| v[i] + q[3] * t[i] + c[i])
}

pub fn frame_vertices(snapshot: &Snapshot, ship: &[Vertex], output: &mut Vec<Vertex>) {
    output.clear();
    let eye_offset = [0.0, -45.0, 220.0];
    output.extend(ship.iter().map(|vertex| Vertex {
        position: std::array::from_fn(|i| vertex.position[i] + eye_offset[i]),
        color: vertex.color,
    }));
    // Navigation markers are prototype fixtures, not station collision/admission geometry.
    for z in [0.0, 1200.0, 2400.0] {
        for segment in 0..32 {
            let angle = segment as f32 * std::f32::consts::TAU / 32.0;
            let next = (segment + 1) as f32 * std::f32::consts::TAU / 32.0;
            let p = |a: f32, radius: f32| [radius * a.cos(), radius * a.sin(), z];
            let quad = [
                p(angle, 150.0),
                p(next, 150.0),
                p(next, 155.0),
                p(angle, 155.0),
            ];
            for index in [0, 1, 2, 0, 2, 3] {
                let relative = std::array::from_fn(|i| quad[index][i] - snapshot.position[i]);
                let local = rotate_inverse(snapshot.orientation, relative);
                output.push(Vertex {
                    position: std::array::from_fn(|i| local[i] + eye_offset[i]),
                    color: [0.08, 0.65, 0.7],
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_or_nonfinite_mesh_is_refused() {
        assert!(decode_mesh(&[]).is_err());
        assert!(decode_mesh(&[0; 73]).is_err());
        let mut bytes = [0; 72];
        bytes[..4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(decode_mesh(&bytes).is_err());
    }

    #[test]
    fn imported_reference_is_complete_and_finite() {
        let ship = load_ship().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(ship.len(), 3484 * 3);
        assert!(
            ship.iter()
                .flat_map(|v| v.color)
                .all(|c| (0.0..=1.0).contains(&c))
        );
    }

    #[test]
    fn inverse_camera_rotation_has_the_expected_handedness() {
        let half = std::f32::consts::FRAC_PI_4;
        let actual = rotate_inverse([0.0, half.sin(), 0.0, half.cos()], [1.0, 0.0, 0.0]);
        assert!(actual[0].abs() < 0.00001);
        assert!((actual[2] - 1.0).abs() < 0.00001);
    }

    #[test]
    fn scene_buffers_are_reused_and_snapshots_are_not_mutated() {
        let snapshot = meridian_core::World::new().snapshot();
        let original = snapshot;
        let ship = load_ship().unwrap_or_else(|error| panic!("{error}"));
        let count = ship.len() + 3 * 32 * 6;
        let mut output = Vec::with_capacity(count);
        let capacity = output.capacity();
        for _ in 0..10 {
            frame_vertices(&snapshot, &ship, &mut output);
            assert_eq!(output.len(), count);
            assert_eq!(output.capacity(), capacity);
            assert_eq!(snapshot, original);
        }
    }
}
