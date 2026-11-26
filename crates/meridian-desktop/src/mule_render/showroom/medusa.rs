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
    if fnv(POINTS) != 0x1333_358a_3e32_0f44 || fnv(FANS) != 0x2621_a4e7_5eb0_1d92 {
        return Err("Medusa inspection source bytes changed");
    }
    let mut words = POINTS.get(8..).ok_or("Missing Medusa point header")?;
    if POINTS.get(..8) != Some(b"MDU171V1")
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
            let [x, y, z] = point.as_chunks::<8>().0 else {
                unreachable!("Medusa points use three f64 coordinates");
            };
            Vec3::new(
                f64::from_le_bytes(*x),
                f64::from_le_bytes(*y),
                f64::from_le_bytes(*z),
            )
        })
        .collect();
    let mut fans = FANS.get(8..).ok_or("Missing Medusa shield header")?;
    if FANS.get(..8) != Some(b"MDUSHLD1") || word(&mut fans)? != PIECES {
        return Err("Invalid Medusa shield header");
    }

    let mut triangles = Vec::new();
    let mut pieces = Vec::with_capacity(PIECES);
    for _ in 0..PIECES {
        let count = word(&mut fans)?;
        let first = triangles.len();
        let mut low = [f32::INFINITY; 4];
        let mut high = [f32::NEG_INFINITY; 4];
        if count < 4 || count > fans.len() / 12 {
            return Err("Invalid Medusa shield facet count");
        }
        for _ in 0..count {
            let [a, b, c] = [
                point(&points, &mut fans)?,
                point(&points, &mut fans)?,
                point(&points, &mut fans)?,
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
    if !fans.is_empty() {
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
