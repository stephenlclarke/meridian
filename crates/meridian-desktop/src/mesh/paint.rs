//! Display finishes for original hull paints. Colour keys are authored sRGB bytes.
pub(crate) fn ship_srgb(source: [u8; 3]) -> [u8; 3] {
    match source {
        // Peregrine: lapis and ochre.
        [70, 108, 157] => [35, 91, 166],
        [52, 84, 125] => [27, 57, 104],
        [215, 204, 175] => [183, 118, 35],
        // Mule: petrol, steel and turquoise.
        [214, 217, 211] => [45, 112, 124],
        [182, 193, 188] => [43, 76, 86],
        [36, 139, 140] => [10, 156, 148],
        // Goshawk family: pine green and blued steel.
        [75, 117, 101] => [39, 126, 83],
        [54, 87, 73] => [28, 84, 56],
        [188, 198, 193] => [88, 112, 124],
        // Exposed metal, glass and running lights shared across families.
        [125, 144, 152] => [74, 95, 111],
        [103, 175, 195] => [29, 128, 170],
        [141, 221, 210] => [15, 208, 176],
        // Other original palettes keep their hue. Pull pale paint down towards
        // its dominant colour without crushing already dark panel details.
        [r, g, b] => {
            let floor = r.min(g).min(b);
            let excess = floor.saturating_sub(100);
            source.map(|c| c - excess)
        }
    }
}

pub(super) fn linear(rgb: [u8; 3]) -> [f32; 3] {
    rgb.map(|channel| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            (value / 12.92) as f32
        } else {
            ((value + 0.055) / 1.055).powf(2.4) as f32
        }
    })
}

pub(super) fn ship_linear(source: [f32; 3]) -> [f32; 3] {
    let srgb = source.map(|value| {
        let encoded = if value <= 0.0031308 {
            12.92 * value
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        };
        (encoded * 255.0).round() as u8
    });
    linear(ship_srgb(srgb))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peregrine_source_mesh_uses_lapis_and_ochre_finishes() {
        let mesh = super::super::load_ship().unwrap_or_else(|error| panic!("{error}"));
        for rgb in [[35, 91, 166], [183, 118, 35], [15, 208, 176]] {
            assert!(mesh.iter().any(|v| v.color == linear(rgb)));
        }
        assert!(!mesh.iter().any(|v| v.color == linear([215, 204, 175])));
    }

    #[test]
    fn other_palette_removes_pale_floor_and_preserves_dark_details() {
        assert_eq!(ship_srgb([211, 183, 117]), [194, 166, 100]);
        assert_eq!(ship_srgb([23, 35, 42]), [23, 35, 42]);
    }
}
