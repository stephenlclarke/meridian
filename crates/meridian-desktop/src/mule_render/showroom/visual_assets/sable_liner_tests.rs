use super::*;

const SH012_POINTS: &[u8] = include_bytes!("../../../../../../assets/fleet/sh012/source.vertices");
const SH012_FACES: &[u8] =
    include_bytes!("../../../../../../assets/fleet/sh012/source-triangles.bin");

fn bytes<const N: usize>(data: &[u8], start: usize) -> [u8; N] {
    data[start..start + N]
        .try_into()
        .unwrap_or_else(|_| panic!("incomplete original source record"))
}

fn position(index: usize) -> [f32; 3] {
    std::array::from_fn(|axis| {
        f64::from_le_bytes(bytes(SH012_POINTS, 16 + index * 24 + axis * 8)) as f32
    })
}

#[test]
fn sable_liner_preserves_all_original_winding_and_material_assignments() {
    let packed = load(SABLE_GEOMETRY_KEY).unwrap_or_else(|reason| panic!("{reason}"));
    assert_eq!(packed.len(), 13_488);
    let mut material_faces = [0; 4];
    for (i, face) in packed.as_chunks::<3>().0.iter().enumerate() {
        let [a, b, c] = *face;
        let expected: [[f32; 3]; 3] = std::array::from_fn(|corner| {
            let vertex = u32::from_le_bytes(bytes(SH012_FACES, i * 12 + corner * 4));
            position(vertex as usize)
        });
        assert_eq!([a.position, b.position, c.position], expected);
        assert_eq!(a.color, b.color);
        assert_eq!(a.color, c.color);
        assert_eq!(a.texture, b.texture);
        assert_eq!(a.texture, c.texture);
        assert_eq!(a.normal, b.normal);
        assert_eq!(a.normal, c.normal);

        let [x, y, z] = std::array::from_fn(|axis| b.position[axis] - a.position[axis]);
        let [u, v, w] = std::array::from_fn(|axis| c.position[axis] - a.position[axis]);
        let outward = [y * w - z * v, z * u - x * w, x * v - y * u];
        assert!(
            outward
                .iter()
                .zip(a.normal)
                .map(|(x, y)| x * y)
                .sum::<f32>()
                > 0.0
        );
        let slot = if a.texture == u32::MAX {
            3
        } else {
            a.texture as usize
        };
        material_faces[slot] += 1;
    }
    assert_eq!(material_faces, [912, 280, 2_064, 1_240]);

    // Independent face values extracted directly from the authenticated original TOML.
    let references = [
        (
            32,
            0x7866_7bff_u32,
            0,
            [[-3.68, 0.48], [-3.68, 0.5436775], [-2.2816, 0.6672867]],
        ),
        (
            104,
            0xd3b7_75ff,
            2,
            [
                [0.7127636, -2.2816],
                [0.5600148, -2.2816],
                [0.523334, -1.6947942],
            ],
        ),
        (
            2172,
            0x5143_57ff,
            1,
            [[0.0, 0.52], [0.0, 0.76], [-1.92, 0.76]],
        ),
        (
            2156,
            0x74bf_ccff,
            u32::MAX,
            [[-2.28, 0.888], [-2.28, 0.616], [-2.328, 0.616]],
        ),
    ];
    for (i, rgba, layer, uv) in references {
        let [a, b, c] = packed.as_chunks::<3>().0[i];
        assert_eq!(
            a.color,
            rgba.to_be_bytes().map(|value| f32::from(value) / 255.0)
        );
        assert_eq!(a.texture, layer);
        assert_eq!([a.uv, b.uv, c.uv], uv);
        assert_eq!([a.emissive, b.emissive, c.emissive], [0; 3]);
    }
}

#[test]
fn only_the_explicit_untextured_sentinel_extends_the_sh012_material_policy() {
    assert!(decode_visual(SH012_VISUAL, b"SBLRVIZ1", SH012_VERTICES).is_err());
    let mut wrong = SH012_VISUAL.to_vec();
    wrong[16 + 48..16 + 52].copy_from_slice(&3u32.to_le_bytes());
    assert!(decode_visual_layers(&wrong, b"SBLRVIZ1", SH012_VERTICES, true).is_err());
    assert!(verify_hash(&wrong, SH012_FNV).is_err());
    assert!(decode_visual_layers(SH012_VISUAL, b"GOSHVIZ1", SH012_VERTICES, true).is_err());
}
