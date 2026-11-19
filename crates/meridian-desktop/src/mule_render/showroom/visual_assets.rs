//! Bound detailed visuals for original Showroom hulls. No collision or selection authority.
use super::super::{VisualTexturePolicy, VisualVertex, decode_visual, decode_visual_layers};
use super::{
    CUSTOM_DETAILED_FACES, CUSTOM_GEOMETRY_KEY, GAZELLE_DETAILED_FACES, GAZELLE_GEOMETRY_KEY,
    GEOMETRY_KEY, MK2_GEOMETRY_KEY, SABLE_GEOMETRY_KEY, VERTICES,
};

const SH008_VISUAL: &[u8] = include_bytes!("../../../../../assets/fleet/sh008/visual.bin");
const SH009_VISUAL: &[u8] = include_bytes!("../../../../../assets/fleet/sh009/visual.bin");
const SH010_VISUAL: &[u8] = include_bytes!("../../../../../assets/fleet/sh010/visual.bin");
const SH012_VISUAL: &[u8] = include_bytes!("../../../../../assets/fleet/sh012/visual.bin");
const SH019_VISUAL: &[u8] = include_bytes!("../../../../../assets/fleet/sh019/visual.bin");
const SH009_VERTICES: usize = 3_532 * 3;
const SH012_VERTICES: usize = 4_496 * 3;
const SH008_FNV: u64 = 0x7a17_0c0c_6cdc_d5fd;
const SH009_FNV: u64 = 0xd01c_abc6_c77a_3b19;
const SH010_FNV: u64 = 0xfe21_3755_8ea3_8ccb;
const SH012_FNV: u64 = 0xba25_8fe1_88c3_f200;
const GAZELLE_VISUAL_FNV: u64 = 0xdd3d_f8b0_db38_0fd2;

// Loader shared by authenticated original showroom hulls.
pub(super) fn load(geometry_key: &str) -> Result<Vec<VisualVertex>, &'static str> {
    match geometry_key {
        GEOMETRY_KEY => checked(SH008_VISUAL, b"GOSHVIZ1", VERTICES, SH008_FNV),
        MK2_GEOMETRY_KEY => checked(SH009_VISUAL, b"GOS2VIZ1", SH009_VERTICES, SH009_FNV),
        CUSTOM_GEOMETRY_KEY => checked(
            SH010_VISUAL,
            b"SH010VZ1",
            CUSTOM_DETAILED_FACES * 3,
            SH010_FNV,
        ),
        SABLE_GEOMETRY_KEY => {
            verify_hash(SH012_VISUAL, SH012_FNV)?;
            decode_visual_layers(
                SH012_VISUAL,
                b"SBLRVIZ1",
                SH012_VERTICES,
                VisualTexturePolicy::SableLiner,
            )
        }
        GAZELLE_GEOMETRY_KEY => {
            verify_hash(SH019_VISUAL, GAZELLE_VISUAL_FNV)?;
            decode_visual_layers(
                SH019_VISUAL,
                b"SH019VZ1",
                GAZELLE_DETAILED_FACES * 3,
                VisualTexturePolicy::Gazelle,
            )
        }
        _ => Err("Unknown original detailed visual"),
    }
}

fn checked(
    bytes: &[u8],
    magic: &[u8; 8],
    count: usize,
    expected_hash: u64,
) -> Result<Vec<VisualVertex>, &'static str> {
    verify_hash(bytes, expected_hash)?;
    decode_visual(bytes, magic, count)
}

fn verify_hash(bytes: &[u8], expected_hash: u64) -> Result<(), &'static str> {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    });
    if hash != expected_hash {
        return Err("Original detailed visual bytes changed");
    }
    Ok(())
}

#[cfg(test)]
mod sable_liner_tests;

#[cfg(test)]
mod tests {
    use super::*;

    const POINTS: &[u8] = include_bytes!("../../../../../assets/fleet/sh009/source.vertices");
    const FACES: &[u8] = include_bytes!("../../../../../assets/fleet/sh009/source-triangles.bin");

    fn array<const N: usize>(bytes: &[u8], start: usize) -> [u8; N] {
        bytes[start..start + N]
            .try_into()
            .unwrap_or_else(|_| panic!("source range is incomplete"))
    }

    fn point(index: usize) -> [f32; 3] {
        std::array::from_fn(|axis| {
            f64::from_le_bytes(array(POINTS, 16 + index * 24 + axis * 8)) as f32
        })
    }

    #[test]
    fn mk2_uses_every_original_detailed_corner_and_its_own_header() {
        assert_eq!(&POINTS[..8], b"MSH009V1");
        assert_eq!(u32::from_le_bytes(array(POINTS, 8)), 2_104);
        let mk2 = load(MK2_GEOMETRY_KEY).unwrap_or_else(|error| panic!("{error}"));
        let explorer = load(GEOMETRY_KEY).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(mk2.len(), 10_596);
        assert_eq!(explorer.len(), 10_452);
        let mut textures = [0; 3];
        let mut emissive = 0;
        let (triangles, remainder) = mk2.as_chunks::<3>();
        assert!(remainder.is_empty());
        for (index, face) in triangles.iter().enumerate() {
            for (corner, vertex) in face.iter().enumerate() {
                let source_index = u32::from_le_bytes(array(FACES, index * 12 + corner * 4));
                assert_eq!(vertex.position, point(source_index as usize));
                assert_eq!(vertex.normal, face[0].normal);
                assert_eq!(vertex.texture, face[0].texture);
                assert_eq!(vertex.color, face[0].color);
                assert_eq!(vertex.emissive, face[0].emissive);
            }
            textures[face[0].texture as usize] += 1;
            emissive += face[0].emissive;
        }
        assert_eq!(textures, [1_356, 0, 2_176]);
        assert_eq!(emissive, 184);
    }

    #[test]
    fn unknown_hull_changed_bytes_and_wrong_format_refuse() {
        assert!(load("assets/fleet/sh001/visual.bin").is_err());
        let mut changed = SH009_VISUAL.to_vec();
        changed[16 + 40] ^= 1;
        assert!(checked(&changed, b"GOS2VIZ1", SH009_VERTICES, SH009_FNV).is_err());
        assert!(checked(SH009_VISUAL, b"GOSHVIZ1", SH009_VERTICES, SH009_FNV).is_err());
        assert!(checked(SH008_VISUAL, b"GOS2VIZ1", SH009_VERTICES, SH009_FNV).is_err());
        assert!(checked(SH010_VISUAL, b"GOSHVIZ1", VERTICES, SH008_FNV).is_err());
        assert!(
            checked(
                SH008_VISUAL,
                b"SH010VZ1",
                CUSTOM_DETAILED_FACES * 3,
                SH010_FNV
            )
            .is_err()
        );
        assert!(
            checked(
                SH009_VISUAL,
                b"SH010VZ1",
                CUSTOM_DETAILED_FACES * 3,
                SH010_FNV
            )
            .is_err()
        );
        assert!(checked(SH010_VISUAL, b"GOS2VIZ1", SH009_VERTICES, SH009_FNV).is_err());
    }

    #[test]
    fn all_goshawk_visuals_still_reject_untextured_and_unknown_layers() {
        for (visual, magic, count) in [
            (SH008_VISUAL, b"GOSHVIZ1", VERTICES),
            (SH009_VISUAL, b"GOS2VIZ1", SH009_VERTICES),
            (SH010_VISUAL, b"SH010VZ1", CUSTOM_DETAILED_FACES * 3),
        ] {
            assert!(decode_visual(visual, magic, count).is_ok());
            let mut changed = visual.to_vec();
            for layer in [3_u32, u32::MAX] {
                changed[16 + 48..16 + 52].copy_from_slice(&layer.to_le_bytes());
                assert!(decode_visual(&changed, magic, count).is_err());
            }
        }
    }

    #[test]
    fn joined_visual_policies_reject_crossed_markers_and_textured_only_hulls() {
        for (source, magic, count, policy, own, crossed) in [
            (
                SH012_VISUAL,
                b"SBLRVIZ1",
                SH012_VERTICES,
                VisualTexturePolicy::SableLiner,
                u32::MAX,
                3,
            ),
            (
                SH019_VISUAL,
                b"SH019VZ1",
                GAZELLE_DETAILED_FACES * 3,
                VisualTexturePolicy::Gazelle,
                3,
                u32::MAX,
            ),
        ] {
            assert!(decode_visual_layers(source, magic, count, policy).is_ok());
            assert!(decode_visual(source, magic, count).is_err());
            for invalid in [crossed, 4, 5, u32::MAX - 1] {
                let mut changed = source.to_vec();
                changed[16 + 48..16 + 52].copy_from_slice(&invalid.to_le_bytes());
                assert!(decode_visual_layers(&changed, magic, count, policy).is_err());
            }
            let mut allowed = source.to_vec();
            allowed[16 + 48..16 + 52].copy_from_slice(&own.to_le_bytes());
            assert!(decode_visual_layers(&allowed, magic, count, policy).is_ok());
        }
        assert!(
            decode_visual_layers(
                SH012_VISUAL,
                b"SBLRVIZ1",
                SH012_VERTICES,
                VisualTexturePolicy::Gazelle
            )
            .is_err()
        );
        assert!(
            decode_visual_layers(
                SH019_VISUAL,
                b"SH019VZ1",
                GAZELLE_DETAILED_FACES * 3,
                VisualTexturePolicy::SableLiner
            )
            .is_err()
        );
        for (bytes, magic, count, hash) in [
            (SH012_VISUAL, b"SBLRVIZ1", SH012_VERTICES, SH012_FNV),
            (
                SH019_VISUAL,
                b"SH019VZ1",
                GAZELLE_DETAILED_FACES * 3,
                GAZELLE_VISUAL_FNV,
            ),
        ] {
            assert!(verify_hash(bytes, hash).is_ok());
            assert!(
                verify_hash(
                    bytes,
                    if magic == b"SBLRVIZ1" {
                        GAZELLE_VISUAL_FNV
                    } else {
                        SH012_FNV
                    }
                )
                .is_err()
            );
            assert!(checked(bytes, magic, count, hash).is_err());
        }
        let mut mule = super::super::super::VISUAL.to_vec();
        for layer in [3_u32, u32::MAX, 4] {
            mule[16 + 48..16 + 52].copy_from_slice(&layer.to_le_bytes());
            assert!(decode_visual(&mule, b"MULEVIS1", super::super::super::VERTEX_COUNT).is_err());
        }
        assert!(
            decode_visual_layers(
                SH008_VISUAL,
                b"GOSHVIZ1",
                VERTICES,
                VisualTexturePolicy::SableLiner
            )
            .is_err()
        );
        assert!(
            decode_visual_layers(
                SH010_VISUAL,
                b"SH010VZ1",
                CUSTOM_DETAILED_FACES * 3,
                VisualTexturePolicy::Gazelle
            )
            .is_err()
        );
    }
}
