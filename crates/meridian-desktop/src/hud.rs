//! Original code-drawn glyphs: no imported bitmap, font or texture assets.
use crate::mesh::Vertex;

pub const MAX_LINES: usize = 12;
pub const MAX_COLUMNS: usize = 96;
pub const MAX_VERTICES: usize = 6 + MAX_LINES * MAX_COLUMNS * 35 * 6;

fn glyph(character: char) -> [u8; 7] {
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
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        '=' => [0, 0, 31, 0, 31, 0, 0],
        ',' => [0, 0, 0, 0, 4, 4, 8],
        '!' => [4, 4, 4, 4, 4, 0, 4],
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
    output.clear();
    if width == 0 || height == 0 {
        return;
    }
    let size = [width as f32, height as f32];
    let scale = text_scale(width);
    let count = lines.len().min(line_budget(width, height));
    let top = (size[1] - count as f32 * 9.0 * scale - 16.0).max(0.0);
    quad(
        output,
        [0.0, top, size[0], size[1]],
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
}
