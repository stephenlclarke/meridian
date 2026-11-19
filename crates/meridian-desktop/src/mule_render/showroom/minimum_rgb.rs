//! Independent full-target RGB measurements, separate from GPU rendering.
use crate::docked_screens::Rect;

const GUTTER_LOGICAL: f64 = 2.0;

#[derive(Debug, PartialEq)]
pub(super) struct PaneMeasurement {
    pub nonblack: usize,
    pub minimum_gutter_logical: f64,
}

fn pixels(rgb: &[u8], size: [u32; 2]) -> Result<&[[u8; 3]], &'static str> {
    let count = (size[0] as usize)
        .checked_mul(size[1] as usize)
        .ok_or("invalid target size")?;
    let (pixels, tail) = rgb.as_chunks::<3>();
    if count == 0 || !tail.is_empty() || pixels.len() != count {
        return Err("invalid full-target RGB size");
    }
    Ok(pixels)
}

fn active(pixel: &[u8; 3]) -> bool {
    *pixel != [0; 3]
}

/// Require an actual nonblack isolated model, and test every pixel against its own pane
/// and the same two-logical-pixel gutter used by the source projection qualification.
pub(super) fn isolated_pane(
    rgb: &[u8],
    size: [u32; 2],
    scale: [f64; 2],
    pane: Rect,
) -> Result<PaneMeasurement, &'static str> {
    if ![pane.x, pane.y, pane.width, pane.height, scale[0], scale[1]]
        .into_iter()
        .all(f64::is_finite)
        || scale.iter().any(|&v| v <= 0.0)
        || pane.x < 0.0
        || pane.y < 0.0
        || pane.width <= 2.0 * GUTTER_LOGICAL
        || pane.height <= 2.0 * GUTTER_LOGICAL
        || (pane.x + pane.width) * scale[0] > f64::from(size[0])
        || (pane.y + pane.height) * scale[1] > f64::from(size[1])
    {
        return Err("invalid own pane");
    }
    let pixels = pixels(rgb, size)?;
    let mut minimum = f64::INFINITY;
    let mut count = 0;
    for (index, pixel) in pixels.iter().enumerate().filter(|(_, p)| active(p)) {
        let x = ((index % size[0] as usize) as f64 + 0.5) / scale[0];
        let y = ((index / size[0] as usize) as f64 + 0.5) / scale[1];
        let gutter = (x - pane.x)
            .min(pane.x + pane.width - x)
            .min(y - pane.y)
            .min(pane.y + pane.height - y);
        if gutter < GUTTER_LOGICAL {
            return Err("nonblack RGB outside own padded pane");
        }
        minimum = minimum.min(gutter);
        count += 1;
    }
    if count == 0 {
        return Err("isolated pane is black");
    }
    Ok(PaneMeasurement {
        nonblack: count,
        minimum_gutter_logical: minimum,
    })
}

/// Compare the entire combined render target to the disjoint union of both isolated
/// targets, including all black gutter and outside-pane pixels.
pub(super) fn exact_pane_union(
    combined: &[u8],
    isolated: [&[u8]; 2],
    size: [u32; 2],
) -> Result<(), &'static str> {
    let combined = pixels(combined, size)?;
    let left = pixels(isolated[0], size)?;
    let right = pixels(isolated[1], size)?;
    for ((actual, a), b) in combined.iter().zip(left).zip(right) {
        if active(a) && active(b) {
            return Err("isolated model pixels overlap");
        }
        let wanted = if active(a) { a } else { b };
        if actual != wanted {
            return Err("combined target differs from exact isolated RGB union");
        }
    }
    Ok(())
}

/// Bind a cached capture to the independent same-pane original model draw.
pub(super) fn exact_original(
    cached: &[u8],
    original: &[u8],
    size: [u32; 2],
) -> Result<(), &'static str> {
    if pixels(cached, size)? != pixels(original, size)? {
        return Err("cached target differs from its direct original hull");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_pane_and_full_union_reject_black_gutter_wrong_pane_and_rgb() {
        let size = [20, 10];
        let scale = [1.0, 1.0];
        let left = Rect::new(0.0, 0.0, 10.0, 10.0);
        let right = Rect::new(10.0, 0.0, 10.0, 10.0);
        let mut a = vec![0_u8; 20 * 10 * 3];
        let mut b = a.clone();
        let place = |rgb: &mut [u8], x: usize, y: usize, color: [u8; 3]| {
            let start = (y * 20 + x) * 3;
            rgb[start..start + 3].copy_from_slice(&color);
        };
        place(&mut a, 4, 4, [11, 0, 3]);
        place(&mut b, 14, 4, [0, 12, 0]);
        assert_eq!(
            isolated_pane(&a, size, scale, left),
            Ok(PaneMeasurement {
                nonblack: 1,
                minimum_gutter_logical: 4.5
            })
        );
        assert!(isolated_pane(&b, size, scale, right).is_ok());
        assert_eq!(
            isolated_pane(&vec![0; a.len()], size, scale, left),
            Err("isolated pane is black")
        );
        assert_eq!(
            isolated_pane(&a, size, scale, right),
            Err("nonblack RGB outside own padded pane")
        );
        assert_eq!(
            isolated_pane(&b, size, scale, left),
            Err("nonblack RGB outside own padded pane")
        );
        let mut spill = a.clone();
        place(&mut spill, 9, 4, [1, 0, 0]);
        assert_eq!(
            isolated_pane(&spill, size, scale, left),
            Err("nonblack RGB outside own padded pane")
        );
        let mut combined = a.clone();
        place(&mut combined, 14, 4, [0, 12, 0]);
        assert!(exact_pane_union(&combined, [&a, &b], size).is_ok());
        let mut bad_rgb = combined.clone();
        place(&mut bad_rgb, 14, 4, [0, 11, 0]);
        assert!(exact_pane_union(&bad_rgb, [&a, &b], size).is_err());
        let mut wrong_hull = a.clone();
        place(&mut wrong_hull, 4, 4, [0, 12, 0]);
        assert!(isolated_pane(&wrong_hull, size, scale, left).is_ok());
        assert_eq!(
            exact_original(&wrong_hull, &a, size),
            Err("cached target differs from its direct original hull")
        );
        assert!(exact_original(&a, &a, size).is_ok());
        assert!(exact_original(&b, &a, size).is_err());
        place(&mut combined, 0, 0, [0, 0, 1]);
        assert!(exact_pane_union(&combined, [&a, &b], size).is_err());
        let mut overlap = b.clone();
        place(&mut overlap, 4, 4, [1, 0, 0]);
        assert_eq!(
            exact_pane_union(&combined, [&a, &overlap], size),
            Err("combined target differs from exact isolated RGB union")
        );
        combined[0..3].fill(0);
        assert_eq!(
            exact_pane_union(&combined, [&a, &overlap], size),
            Err("isolated model pixels overlap")
        );
    }
}
