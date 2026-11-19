//! Independently pinned no-map source face ranges, used by portable and Metal evidence.
use super::{Hull, VisualVertex, visual_assets};

const GLASS: [u8; 3] = [116, 191, 204];
const EMITTER: [u8; 3] = [185, 235, 217];
type SourceRange = (usize, usize, [u8; 3], u32);
// Half-open ranges extracted from original faceTextures/faceMaterials, not from visual.bin.
const SABLE: &[SourceRange] = &[
    (2156, 2168, GLASS, 0),
    (2436, 2564, EMITTER, 1),
    (2948, 2960, GLASS, 0),
    (3228, 3356, EMITTER, 1),
    (3356, 4316, GLASS, 0),
];
const GAZELLE: &[SourceRange] = &[
    (3532, 3660, EMITTER, 1),
    (4312, 4440, EMITTER, 1),
    (4440, 5400, GLASS, 0),
];

pub(super) fn material_faces(hull: Hull) -> Vec<(usize, [VisualVertex; 3], [u8; 3])> {
    let (ranges, layer) = match hull {
        Hull::SableLiner => (SABLE, u32::MAX),
        Hull::Gazelle => (GAZELLE, 3),
        _ => panic!("not a material-only hull"),
    };
    let vertices =
        visual_assets::load(hull.geometry_key()).unwrap_or_else(|error| panic!("{error}"));
    let (triangles, tail) = vertices.as_chunks::<3>();
    assert!(tail.is_empty());
    let mut result = Vec::new();
    for (index, triangle) in triangles.iter().copied().enumerate() {
        let expected = ranges
            .iter()
            .find(|(start, end, _, _)| (*start..*end).contains(&index));
        if let Some((_, _, rgb, emissive)) = expected {
            let colour = [rgb[0], rgb[1], rgb[2], 255].map(|byte| f32::from(byte) / 255.0);
            for vertex in triangle {
                assert_eq!(vertex.texture, layer);
                assert_eq!(vertex.color, colour);
                assert_eq!(vertex.emissive, *emissive);
            }
            result.push((index, triangle, *rgb));
        } else {
            assert!(triangle.iter().all(|vertex| vertex.texture < 3));
        }
    }
    assert_eq!(
        result.len(),
        ranges
            .iter()
            .map(|(start, end, _, _)| end - start)
            .sum::<usize>()
    );
    result
}

#[test]
fn joined_material_only_faces_match_independent_original_raw_colours() {
    for (hull, glass_count) in [(Hull::SableLiner, 984), (Hull::Gazelle, 960)] {
        let faces = material_faces(hull);
        assert_eq!(
            faces.iter().filter(|(_, _, rgb)| *rgb == GLASS).count(),
            glass_count
        );
        assert_eq!(
            faces.iter().filter(|(_, _, rgb)| *rgb == EMITTER).count(),
            256
        );
    }
}
