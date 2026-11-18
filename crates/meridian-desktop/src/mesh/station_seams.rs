//! Local display joins between the SH028 ring rim and the back of the aperture crosspieces.
use super::Vertex;

// Offsets into the original expanded sh028.mesh stream. Each rim chain uses the
// inner edges of the ring at source Z=-80, stopping at the central rib.
const UPPER_RIMS: [[usize; 3]; 2] = [[1040, 968, 896], [752, 680, 608]];
const LOWER_RIMS: [[usize; 3]; 2] = [[2336, 2408, 2480], [2624, 2696, 2768]];
// Rear outer edges of the broad rails at Y=±64.4 and Z=82.8. The port ends at
// Y=±50.8. The attachment stays on the exterior side of each rail.
const UPPER_RAIL: [usize; 2] = [29388, 29389];
const LOWER_RAIL: [usize; 2] = [29357, 29354];
const ORIGINAL_VERTICES: usize = 30_168;

pub(super) fn append(vertices: &mut Vec<Vertex>) -> Result<(), String> {
    if vertices.len() != ORIGINAL_VERTICES {
        return Err("Unexpected SH028 display triangle count".into());
    }
    for (rims, rail) in [(UPPER_RIMS, UPPER_RAIL), (LOWER_RIMS, LOWER_RAIL)] {
        let [left, right] = rail.map(|index| vertices[index]);
        debug_assert_eq!(left.position[1..], right.position[1..]);
        let rail_at = |x: f32| Vertex {
            position: [x, left.position[1], left.position[2]],
            color: left.color,
        };
        for rim in rims {
            let points = rim.map(|index| vertices[index].position);
            for (a, b) in points.into_iter().zip(points.into_iter().skip(1)) {
                let c = rail_at(b[0]);
                let d = rail_at(a[0]);
                let a = Vertex {
                    position: a,
                    color: left.color,
                };
                let b = Vertex {
                    position: b,
                    color: left.color,
                };
                // Retain a forward-facing triangle order for both crosspieces.
                if left.position[1] > 0.0 {
                    vertices.extend([a, d, c, a, c, b]);
                } else {
                    vertices.extend([a, b, c, a, c, d]);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
