//! Original code-drawn glyphs: no imported bitmap, font or texture assets.
use crate::mesh::Vertex;
pub mod cockpit;

pub const MAX_LINES: usize = 14;
pub const MAX_COLUMNS: usize = 96;
pub const MAX_VERTICES: usize =
    cockpit::MAX_VERTICES + 6 + MAX_LINES * MAX_COLUMNS * 35 * 6 + crate::help::MAX_VERTICES;

pub(crate) fn glyph(character: char) -> [u8; 7] {
    match character.to_ascii_uppercase() {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 25, 21, 19, 19, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 27, 17],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '.' => [0, 0, 0, 0, 0, 4, 4],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        ';' => [0, 4, 4, 0, 4, 4, 8],
        '\'' => [4, 4, 8, 0, 0, 0, 0],
        '"' => [10, 10, 0, 0, 0, 0, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        '=' => [0, 0, 31, 0, 31, 0, 0],
        ',' => [0, 0, 0, 0, 4, 4, 8],
        '!' => [4, 4, 4, 4, 4, 0, 4],
        '|' => [4; 7],
        '[' => [14, 8, 8, 8, 8, 8, 14],
        ']' => [14, 2, 2, 2, 2, 2, 14],
        '>' => [16, 8, 4, 2, 4, 8, 16],
        ' ' => [0; 7],
        _ => [14, 17, 1, 2, 4, 0, 4],
    }
}

fn quad(output: &mut Vec<Vertex>, rect: [f32; 4], color: [f32; 3], size: [f32; 2]) {
    let [left, top, right, bottom] = rect;
    let point = |x, y| Vertex {
        position: [2.0 * x / size[0] - 1.0, 1.0 - 2.0 * y / size[1], 0.0],
        color,
    };
    output.extend_from_slice(&[
        point(left, top),
        point(left, bottom),
        point(right, top),
        point(right, top),
        point(left, bottom),
        point(right, bottom),
    ]);
}

fn text_scale(width: u32) -> f32 {
    (width as f32 / (MAX_COLUMNS as f32 * 6.0 + 16.0)).clamp(1.0, 2.0)
}

pub fn line_budget(width: u32, height: u32) -> usize {
    if width == 0 || height == 0 {
        return 0;
    }
    ((height.saturating_sub(16) as f32 / (9.0 * text_scale(width))) as usize).min(MAX_LINES)
}

/// Bounded, reusable CPU buffer; text is presentation only and cannot issue commands.
pub fn vertices(lines: &[String], width: u32, height: u32, output: &mut Vec<Vertex>) {
    vertices_with_cockpit(lines, width, height, None, output);
}

pub fn vertices_with_cockpit(
    lines: &[String],
    width: u32,
    height: u32,
    cockpit: Option<&cockpit::Cockpit>,
    output: &mut Vec<Vertex>,
) {
    output.clear();
    if width == 0 || height == 0 {
        return;
    }
    let size = [width as f32, height as f32];
    let scale = text_scale(width);
    let bottom = if cockpit.is_some() && width >= 320 && height >= 240 {
        cockpit::Layout::new(width, height).top
    } else {
        size[1]
    };
    let count = lines.len().min(line_budget(width, bottom as u32));
    let top = (bottom - count as f32 * 9.0 * scale - 16.0).max(0.0);
    quad(
        output,
        [0.0, top, size[0], bottom],
        [0.006, 0.012, 0.025],
        size,
    );
    let columns = MAX_COLUMNS.min(((size[0] - 16.0).max(0.0) / (6.0 * scale)) as usize);
    for (line_index, line) in lines.iter().take(count).enumerate() {
        for (column, character) in line.chars().take(columns).enumerate() {
            for (row, bits) in glyph(character).into_iter().enumerate() {
                for bit in 0..5 {
                    if bits & (1 << (4 - bit)) == 0 {
                        continue;
                    }
                    let x = 8.0 + (column * 6 + bit) as f32 * scale;
                    let y = top + 8.0 + (line_index * 9 + row) as f32 * scale;
                    quad(output, [x, y, x + scale, y + scale], [0.7, 0.9, 0.95], size);
                }
            }
        }
    }
    if let Some(c) = cockpit {
        cockpit::append(c, width, height, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipped_tiny_and_zero_views_are_finite_bounded_and_reuse_storage() {
        let lines = vec!["W".repeat(MAX_COLUMNS + 100); MAX_LINES + 20];
        let mut output = Vec::with_capacity(MAX_VERTICES);
        let capacity = output.capacity();
        for size in [[1920, 1080], [1280, 100], [640, 360], [1, 1], [0, 720]] {
            vertices(&lines, size[0], size[1], &mut output);
            assert!(output.len() <= MAX_VERTICES);
            assert_eq!(output.capacity(), capacity);
            assert!(output.iter().flat_map(|v| v.position).all(f32::is_finite));
            assert!(
                output
                    .iter()
                    .all(|v| v.position[0].abs() <= 1.000001 && v.position[1].abs() <= 1.000001)
            );
        }
        assert!(output.is_empty());
        assert_eq!(line_budget(1280, 100), 4);
        assert_eq!(line_budget(640, 360), MAX_LINES);
    }
    #[test]
    fn original_letters_are_legible_nonempty_and_lowercase_maps_identically() {
        for c in 'A'..='Z' {
            assert!(glyph(c).iter().any(|row| *row != 0));
            assert_eq!(glyph(c), glyph(c.to_ascii_lowercase()));
        }
        assert_eq!(glyph(' '), [0; 7]);
        assert_eq!(glyph('\u{2603}'), glyph('?'));
    }
    #[test]
    fn market_controls_and_all_original_names_have_drawable_glyphs() {
        let mut world = meridian_core::World::new_commander(7001).unwrap_or_else(|e| panic!("{e}"));
        let mut session = crate::session::Session::default();
        session
            .command(&mut world, crate::session::Action::ToggleMarket)
            .unwrap_or_else(|e| panic!("{e}"));
        for _ in 0..17 {
            for line in session.lines(&world, true, Some("MARKET")) {
                for character in line.chars() {
                    assert_ne!(
                        glyph(character),
                        glyph('?'),
                        "missing glyph {character:?} in {line}"
                    );
                }
            }
            session
                .command(&mut world, crate::session::Action::NextCommodity)
                .unwrap_or_else(|e| panic!("{e}"));
        }
    }
}

/// Draw a centered help card after the normal station/cockpit pass.
pub fn append_help(panel: &crate::help::Panel, width: u32, height: u32, output: &mut Vec<Vertex>) {
    if width < 48 || height < 48 {
        return;
    }
    let size = [width as f32, height as f32];
    let scale = (size[0] / 600.0).floor().clamp(1.0, 2.0);
    let columns = (((size[0] - 48.0) / (6.0 * scale)) as usize).min(crate::help::MAX_COLUMNS);
    let rows = (((size[1] - 48.0) / (9.0 * scale)) as usize).min(crate::help::MAX_LINES);
    let lines = std::iter::once(panel.title.as_str())
        .chain(panel.lines.iter().map(String::as_str))
        .chain(std::iter::once("Escape / H / ? Close help"))
        .collect::<Vec<_>>();
    let count = lines.len().min(rows);
    let card_width = (columns as f32 * 6.0 + 16.0) * scale;
    let card_height = (count as f32 * 9.0 + 16.0) * scale;
    let left = (size[0] - card_width) / 2.0;
    let top = (size[1] - card_height) / 2.0;
    quad(
        output,
        [left, top, left + card_width, top + card_height],
        [0.015, 0.045, 0.065],
        size,
    );
    for (row, line) in lines.into_iter().take(count).enumerate() {
        for (column, character) in line.chars().take(columns).enumerate() {
            for (y, bits) in glyph(character).into_iter().enumerate() {
                for x in 0..5 {
                    if bits & (1 << (4 - x)) == 0 {
                        continue;
                    }
                    let px = left + (8 + column * 6 + x) as f32 * scale;
                    let py = top + (8 + row * 9 + y) as f32 * scale;
                    quad(
                        output,
                        [px, py, px + scale, py + scale],
                        [0.75, 0.94, 0.96],
                        size,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod help_tests {
    use super::*;
    #[test]
    fn modal_card_stays_finite_and_bounded_across_compact_retina_and_tiny_views() {
        for screen in crate::docked_screens::Screen::ALL {
            let panel = crate::help::Panel::station(screen, Some("Review"), true, true);
            for [width, height] in [
                [0, 0],
                [20, 20],
                [48, 48],
                [640, 360],
                [1280, 720],
                [2560, 1600],
            ] {
                let mut vertices = Vec::with_capacity(crate::help::MAX_VERTICES);
                append_help(&panel, width, height, &mut vertices);
                assert!(vertices.len() <= crate::help::MAX_VERTICES);
                for vertex in vertices {
                    assert!(vertex.position.iter().all(|value| value.is_finite()));
                    assert!((-1.0..=1.0).contains(&vertex.position[0]));
                    assert!((-1.0..=1.0).contains(&vertex.position[1]));
                }
            }
        }
    }
}
