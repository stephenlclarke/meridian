//! Original detailed visuals for the Goshawk family. No collision or selection authority.
use super::super::{VisualVertex, decode_visual};
use super::{GEOMETRY_KEY, VERTICES};

pub(super) const MK2_GEOMETRY_KEY: &str = "assets/fleet/sh009/visual.bin";
const SH008_VISUAL: &[u8] = include_bytes!("../../../../../assets/fleet/sh008/visual.bin");
const SH009_VISUAL: &[u8] = include_bytes!("../../../../../assets/fleet/sh009/visual.bin");
const SH009_VERTICES: usize = 3_532 * 3;
const SH008_FNV: u64 = 0x7a17_0c0c_6cdc_d5fd;
const SH009_FNV: u64 = 0xd01c_abc6_c77a_3b19;

// Loader shared by the existing SH008 pass and the pending SH009 registration.
pub(super) fn load(geometry_key: &str) -> Result<Vec<VisualVertex>, &'static str> {
    match geometry_key {
        GEOMETRY_KEY => checked(SH008_VISUAL, b"GOSHVIZ1", VERTICES, SH008_FNV),
        MK2_GEOMETRY_KEY => checked(SH009_VISUAL, b"GOS2VIZ1", SH009_VERTICES, SH009_FNV),
        _ => Err("Unknown Goshawk detailed visual"),
    }
}

fn checked(
    bytes: &[u8],
    magic: &[u8; 8],
    count: usize,
    expected_hash: u64,
) -> Result<Vec<VisualVertex>, &'static str> {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    });
    if hash != expected_hash {
        return Err("Goshawk detailed visual bytes changed");
    }
    decode_visual(bytes, magic, count)
}

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
        for (index, face) in mk2.chunks_exact(3).enumerate() {
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
    }
}
