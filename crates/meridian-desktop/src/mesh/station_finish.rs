//! Station display materials, applied once when the original SH028 mesh loads.
use super::{Vertex, cross, difference, point};

use super::paint::linear;

pub(super) fn apply(mesh: &mut [Vertex]) {
    // Original paint colours identify the five used materials in SH028.design.
    let finishes = [
        ([214, 217, 211], [44, 103, 145], false), // ocean-blue hull plating
        ([182, 193, 188], [34, 65, 92], false),   // structural frame
        ([23, 35, 42], [24, 41, 55], false),      // recessed details
        ([125, 144, 152], [73, 97, 109], false),  // exposed metal
        ([141, 221, 210], [255, 146, 24], true),  // original emissive strips
    ]
    .map(|(source, finish, emissive)| (linear(source), linear(finish), emissive));
    for triangle in mesh.as_chunks_mut::<3>().0 {
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
    fn all_five_station_roles_get_their_display_finish() {
        for (source, paint, light) in [
            ([214, 217, 211], [44, 103, 145], 0.808),
            ([182, 193, 188], [34, 65, 92], 0.808),
            ([23, 35, 42], [24, 41, 55], 0.808),
            ([125, 144, 152], [73, 97, 109], 0.808),
            ([141, 221, 210], [255, 146, 24], 1.0),
        ] {
            let mut panel = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]].map(|position| Vertex {
                position,
                color: linear(source),
            });
            apply(&mut panel);
            for corner in panel {
                assert_eq!(corner.color, linear(paint).map(|channel| channel * light));
            }
        }
    }

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
                assert_eq!(after.color, linear([255, 146, 24]));
            } else {
                assert!(after.color[2] > after.color[0]);
            }
        }
        assert_eq!(lights, 24 * 3);
    }
}
