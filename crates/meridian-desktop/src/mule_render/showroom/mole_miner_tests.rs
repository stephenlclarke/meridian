use super::*;

const SOURCE_POINTS: &[u8] = include_bytes!("../../../../../assets/fleet/sh016/source.vertices");
const SOURCE_FACES: &[u8] =
    include_bytes!("../../../../../assets/fleet/sh016/source-triangles.bin");
const NATIVE_VISUAL: &[u8] = include_bytes!("../../../../../assets/fleet/sh016/native-visual.bin");

fn bytes<const N: usize>(source: &[u8], start: usize) -> [u8; N] {
    source[start..start + N]
        .try_into()
        .unwrap_or_else(|_| panic!("incomplete SH016 source record"))
}

fn source_point(index: usize) -> [f32; 3] {
    std::array::from_fn(|axis| {
        f64::from_le_bytes(bytes(SOURCE_POINTS, 16 + index * 24 + axis * 8)) as f32
    })
}

#[test]
fn mole_miner_visual_keeps_original_corners_winding_and_slate_orange_materials() {
    let vertices = Hull::MoleMiner
        .visual()
        .unwrap_or_else(|reason| panic!("{reason}"));
    assert_eq!(&SOURCE_POINTS[..8], b"MSH016V1");
    assert_eq!(u32::from_le_bytes(bytes(SOURCE_POINTS, 8)), 6_760);
    assert_eq!(vertices.len(), 38_508);
    let (faces, tail) = vertices.as_chunks::<3>();
    assert!(tail.is_empty());
    let mut textures = [0; 3];
    let mut emissive = 0;
    for (index, [a, b, c]) in faces.iter().copied().enumerate() {
        for (corner, vertex) in [a, b, c].into_iter().enumerate() {
            let original = u32::from_le_bytes(bytes(SOURCE_FACES, index * 12 + corner * 4));
            assert!(original < 6_760);
            assert_eq!(vertex.position, source_point(original as usize));
            assert_eq!(vertex.normal, a.normal);
            assert_eq!(vertex.color, a.color);
            assert_eq!(vertex.texture, a.texture);
            assert_eq!(vertex.emissive, a.emissive);
        }
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
        textures[a.texture as usize] += 1;
        emissive += a.emissive;
    }
    assert_eq!(textures, [476, 0, 12_360]);
    assert_eq!(emissive, 208);

    // Independent samples from the original resource include slate, orange, repeating UVs and emissivity.
    for (index, colour, layer, emissive, uv) in [
        (
            0,
            0x7d8c92ff_u32,
            0,
            0,
            [
                [0.199808, 0.092037],
                [0.179143, 0.05899],
                [0.146269, 0.030612],
            ],
        ),
        (
            2868,
            0xec9e45ff,
            2,
            0,
            [
                [-0.46835, -0.11475],
                [-0.35785, -0.11475],
                [-0.35785, -0.22525],
            ],
        ),
        (
            980,
            0x8dddd2ff,
            2,
            1,
            [
                [-0.586075, 0.73525],
                [-0.586075, 0.7565],
                [-0.607325, 0.7565],
            ],
        ),
    ] {
        let [a, b, c] = faces[index];
        assert_eq!(a.color, colour.to_be_bytes().map(|c| f32::from(c) / 255.0));
        assert_eq!(a.texture, layer);
        assert_eq!(a.emissive, emissive);
        assert_eq!([a.uv, b.uv, c.uv], uv);
    }
}

#[test]
fn mole_miner_authentication_refuses_native_mutated_and_sibling_visuals() {
    let load = |source: &[u8]| {
        authenticated_visual(
            source,
            MOLE_MINER_VISUAL_FNV,
            b"MOLEVIS1",
            MOLE_MINER_FACES * 3,
        )
    };
    assert!(load(MOLE_MINER_VISUAL).is_ok());
    assert!(load(NATIVE_VISUAL).is_err());
    assert!(load(VISUAL).is_err());
    assert!(load(CUSTOM_VISUAL).is_err());
    assert!(decode_visual(NATIVE_VISUAL, b"MOLEVIS1", MOLE_MINER_FACES * 3).is_err());
    let mut changed = MOLE_MINER_VISUAL.to_vec();
    changed[16 + 40] ^= 1;
    assert!(load(&changed).is_err());
    changed[16 + 48..16 + 52].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_visual(&changed, b"MOLEVIS1", MOLE_MINER_FACES * 3).is_err());
}

#[test]
fn mole_miner_203_shimmer_support_ranges_match_original_membership() {
    let source = MoleMinerHullSource::bundled().unwrap_or_else(|reason| panic!("{reason:?}"));
    let (triangles, pieces) = Hull::MoleMiner
        .supports()
        .unwrap_or_else(|reason| panic!("{reason:?}"));
    assert_eq!(pieces.len(), MOLE_MINER_PIECES);
    assert_eq!(
        source
            .pieces()
            .iter()
            .map(|piece| piece.source_points().len())
            .sum::<usize>(),
        MOLE_MINER_SOURCE_SLOTS
    );
    let mut first = 0;
    for (rendered, original) in pieces.iter().zip(source.pieces()) {
        assert_eq!(rendered.range[0], first);
        assert!(rendered.range[1] > 0);
        first += rendered.range[1];
        let (low, high) = original.source_bounds();
        assert_eq!(rendered.low[..3], vector(low)[..3]);
        assert_eq!(rendered.high[..3], vector(high)[..3]);
        assert_eq!(rendered.low[3], 0.02);
    }
    assert_eq!(first as usize, triangles.len());
}

#[test]
fn mole_miner_model_key_never_matches_mole_prospector_or_other_hulls() {
    let preview = |object: &str, geometry_key: &str| ModelPreview {
        object: object.into(),
        geometry_key: geometry_key.into(),
        bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
        orientation: Orientation::default(),
    };
    assert_eq!(
        hull(&preview("SH016", MOLE_MINER_GEOMETRY_KEY)),
        Some(Hull::MoleMiner)
    );
    for model in [
        preview("SH017", MOLE_MINER_GEOMETRY_KEY),
        preview("SH016", "assets/fleet/sh017/visual.bin"),
        preview("SH016", "assets/fleet/sh016/native-visual.bin"),
        preview("SH016", GEOMETRY_KEY),
        preview("SH016", CUSTOM_GEOMETRY_KEY),
        preview("SH008", MOLE_MINER_GEOMETRY_KEY),
        preview("SH010", MOLE_MINER_GEOMETRY_KEY),
    ] {
        assert_eq!(hull(&model), None);
        assert!(!matches(&model));
    }
}
