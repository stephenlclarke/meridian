//! Station display materials, applied once when the original SH028 mesh loads.
use super::{Vertex, cross, difference, point};

fn linear(rgb: [u8; 3]) -> [f32; 3] {
    rgb.map(|channel| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            (value / 12.92) as f32
        } else {
            ((value + 0.055) / 1.055).powf(2.4) as f32
        }
    })
}

pub(super) fn apply(mesh: &mut [Vertex]) {
    // Original paint colours identify the five used materials in SH028.design.
    let finishes = [
        ([214, 217, 211], [151, 174, 189], false), // blue-grey hull plating
        ([182, 193, 188], [89, 112, 132], false),  // structural frame
        ([23, 35, 42], [54, 70, 83], false),       // recessed details
        ([125, 144, 152], [103, 119, 130], false), // exposed metal
        ([141, 221, 210], [255, 191, 100], true),  // original emissive strips
    ]
    .map(|(source, finish, emissive)| (linear(source), linear(finish), emissive));
    for triangle in mesh.chunks_exact_mut(3) {
        let Some((_, finish, emissive)) = finishes.iter().find(|(source, _, _)| {
            source
                .iter()
                .zip(triangle[0].color)
                .all(|(a, b)| (a - b).abs() < 1e-6)
        }) else {
            continue;
        };
        let [a, b, c] = [0, 1, 2].map(|index| point(triangle[index].position));
        let normal = cross(difference(b, a), difference(c, a));
        let light = if *emissive {
            1.0
        } else {
            // Two-sided diffuse response also handles the thin original panels.
            let facing = (normal.x * -0.48 + normal.y * 0.64 + normal.z * 0.60).abs();
            (0.52 + 0.48 * facing / normal.dot(normal).sqrt().max(f64::EPSILON)) as f32
        };
        for vertex in triangle {
            vertex.color = finish.map(|value| value * light);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn station_finish_changes_paint_only_and_keeps_original_lights_readable() {
        let original =
            crate::mesh::decode_mesh(include_bytes!("../../../../assets/legacy/sh028.mesh"))
                .unwrap_or_else(|error| panic!("{error}"));
        let mut styled = original.clone();
        apply(&mut styled);
        let source_light = linear([141, 221, 210]);
        let mut lights = 0;
        for (before, after) in original.iter().zip(&styled) {
            assert_eq!(before.position, after.position);
            assert_ne!(before.color, after.color);
            assert!(after.color.iter().all(|c| (0.0..=1.0).contains(c)));
            if before.color == source_light {
                lights += 1;
                assert_eq!(after.color, linear([255, 191, 100]));
            } else {
                assert!(after.color[2] > after.color[0]);
            }
        }
        assert_eq!(lights, 24 * 3);
    }
}
