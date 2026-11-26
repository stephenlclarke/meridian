//! Source-bound Medusa artwork and inspection shield.
use super::{Piece, Triangle, Vec3, VisualVertex, authenticated_visual, fnv};
use crate::mule_render::{triangle, vector};

pub(super) const DETAIL_FACES: usize = 162_548;
pub(super) const PIECES: usize = 729;
const DETAIL_POINTS: usize = 82_436;
const NATIVE_POINTS: usize = 39_276;
const ROUNDING_METRES: f32 = 0.02;
const FAN_ERROR_METRES: f32 = 0.000_001;
const VISUAL: &[u8] = include_bytes!("../../../../../assets/fleet/sh171/visual.bin");
const POINTS: &[u8] = include_bytes!("../../../../../assets/fleet/sh171/source.vertices");
const FANS: &[u8] = include_bytes!("../../../../../assets/fleet/sh171/shield-fans.bin");

pub(super) fn visual() -> Result<Vec<VisualVertex>, &'static str> {
    authenticated_visual(VISUAL, 0xb4e6_2c61_7475_7d63, b"MEDUVIZ1", DETAIL_FACES * 3)
}

pub(super) fn supports() -> Result<(Vec<Triangle>, Vec<Piece>), &'static str> {
    authenticated_supports(POINTS, FANS)
}

fn authenticated_supports(
    points: &[u8],
    fans: &[u8],
) -> Result<(Vec<Triangle>, Vec<Piece>), &'static str> {
    if fnv(points) != 0x1333_358a_3e32_0f44 || fnv(fans) != 0x2621_a4e7_5eb0_1d92 {
        return Err("Medusa inspection source bytes changed");
    }
    decode_supports(points, fans)
}

fn decode_supports(
    points: &[u8],
    fans: &[u8],
) -> Result<(Vec<Triangle>, Vec<Piece>), &'static str> {
    let mut words = points.get(8..).ok_or("Missing Medusa point header")?;
    if points.get(..8) != Some(b"MDU171V1")
        || word(&mut words)? != DETAIL_POINTS
        || word(&mut words)? != NATIVE_POINTS
    {
        return Err("Invalid Medusa source point header");
    }
    let (encoded_points, trailing) = words.as_chunks::<24>();
    if !trailing.is_empty() || encoded_points.len() != DETAIL_POINTS + NATIVE_POINTS {
        return Err("Invalid Medusa source point count");
    }
    let points: Vec<_> = encoded_points
        .iter()
        .map(|point| {
            let coordinate =
                |start| f64::from_le_bytes(std::array::from_fn(|offset| point[start + offset]));
            Vec3::new(coordinate(0), coordinate(8), coordinate(16))
        })
        .collect();
    let mut words = fans.get(8..).ok_or("Missing Medusa shield header")?;
    if fans.get(..8) != Some(b"MDUSHLD1") || word(&mut words)? != PIECES {
        return Err("Invalid Medusa shield header");
    }

    let mut triangles = Vec::new();
    let mut pieces = Vec::with_capacity(PIECES);
    for _ in 0..PIECES {
        let count = word(&mut words)?;
        let first = triangles.len();
        let mut low = [f32::INFINITY; 4];
        let mut high = [f32::NEG_INFINITY; 4];
        if count < 4 || count > words.len() / 12 {
            return Err("Invalid Medusa shield facet count");
        }
        for _ in 0..count {
            let [a, b, c] = [
                point(&points, &mut words)?,
                point(&points, &mut words)?,
                point(&points, &mut words)?,
            ];
            for vertex in [a, b, c] {
                for (axis, coordinate) in vector(vertex)[..3].iter().enumerate() {
                    low[axis] = low[axis].min(*coordinate);
                    high[axis] = high[axis].max(*coordinate);
                }
            }
            triangles.push(triangle(a, b, c)?);
        }
        low[3] = ROUNDING_METRES;
        high[3] = FAN_ERROR_METRES;
        pieces.push(Piece {
            range: [first as u32, count as u32, 0, 0],
            low,
            high,
        });
    }
    if !words.is_empty() {
        return Err("Trailing Medusa shield facets");
    }
    Ok((triangles, pieces))
}

fn word(bytes: &mut &[u8]) -> Result<usize, &'static str> {
    let (value, rest) = bytes
        .split_first_chunk::<4>()
        .ok_or("Truncated Medusa source")?;
    *bytes = rest;
    Ok(u32::from_le_bytes(*value) as usize)
}

fn point(points: &[Vec3], bytes: &mut &[u8]) -> Result<Vec3, &'static str> {
    let index = word(bytes)?;
    points
        .get(index)
        .copied()
        .ok_or("Invalid Medusa shield point")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn altered_inspection_assets_are_refused_before_payload_decode() {
        let authentication = Some("Medusa inspection source bytes changed");
        assert_eq!(
            authenticated_supports(&POINTS[1..], FANS).err(),
            authentication
        );
        assert_eq!(
            authenticated_supports(POINTS, &FANS[..FANS.len() - 1]).err(),
            authentication
        );
    }

    #[test]
    fn malformed_point_magic_and_count_fail_closed() {
        let mut wrong_magic = POINTS[..16].to_vec();
        wrong_magic[0] ^= 1;
        assert_eq!(
            decode_supports(&wrong_magic, FANS).err(),
            Some("Invalid Medusa source point header")
        );
        assert_eq!(
            decode_supports(&POINTS[..POINTS.len() - 1], FANS).err(),
            Some("Invalid Medusa source point count")
        );
    }

    #[test]
    fn invalid_shield_groups_indices_and_trailing_facets_fail_closed() {
        assert_eq!(
            decode_supports(POINTS, b"MDUSHLD1\0\0\0\0").err(),
            Some("Invalid Medusa shield header")
        );
        let mut bad_facets = FANS[..16].to_vec();
        bad_facets[12..16].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            decode_supports(POINTS, &bad_facets).err(),
            Some("Invalid Medusa shield facet count")
        );

        let mut bad_index = FANS.to_vec();
        bad_index[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            decode_supports(POINTS, &bad_index).err(),
            Some("Invalid Medusa shield point")
        );

        let mut trailing = FANS.to_vec();
        trailing.extend_from_slice(&[0; 12]);
        assert_eq!(
            decode_supports(POINTS, &trailing).err(),
            Some("Trailing Medusa shield facets")
        );
    }
}
