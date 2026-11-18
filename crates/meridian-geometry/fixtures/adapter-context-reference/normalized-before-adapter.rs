use crate::{Budget, Facet, Failure, FailureKind, SourceId, Vertex, fraction};
use std::sync::Arc;

/// The sealed original component input for the five still-unqualified hulls.
/// An asset selects immutable byte equality, never an admission entitlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalAsset {
    Sh010,
    Sh011,
    Sh016,
    Sh017,
    Sh166,
}
impl OriginalAsset {
    pub fn id(self) -> &'static str {
        match self {
            Self::Sh010 => "SH010",
            Self::Sh011 => "SH011",
            Self::Sh016 => "SH016",
            Self::Sh017 => "SH017",
            Self::Sh166 => "SH166",
        }
    }
    pub fn pinned_bytes(self) -> &'static [u8] {
        match self {
            Self::Sh010 => include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/SH010.facets")),
            Self::Sh011 => include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/SH011.facets")),
            Self::Sh016 => include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/SH016.facets")),
            Self::Sh017 => include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/SH017.facets")),
            Self::Sh166 => include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/SH166.facets")),
        }
    }
    pub fn original_triangle_count(self) -> usize {
        match self {
            Self::Sh010 | Self::Sh011 => 1012,
            Self::Sh016 | Self::Sh017 => 1100,
            Self::Sh166 => 988,
        }
    }
    /// Decode only authenticated input. Reconstruction remains an explicit caller operation.
    pub fn decode(self, bytes: &[u8], b: &mut Budget) -> Result<Vec<Facet>, Failure> {
        // Full byte equality with the compiled source asset also binds canonical style
        // encoding, all exact source/UV pairs, transformed points and component ranges.
        b.step(bytes.len() as u64)?;
        if bytes != self.pinned_bytes() {
            return Err(b.fail(FailureKind::InvalidInput, "changed bound original asset"));
        }
        let text = std::str::from_utf8(bytes)
            .map_err(|_| b.fail(FailureKind::InvalidInput, "invalid source asset encoding"))?;
        let mut lines = text.lines();
        let header = format!(
            "meridian-exact-facets-v1 {}",
            self.original_triangle_count()
        );
        if lines.next() != Some(header.as_str()) {
            return Err(b.fail(FailureKind::InvalidInput, "source header mismatch"));
        }
        let mut result = Vec::new();
        // Only this decode owns the pool. Facets keep immutable handles after
        // the pool drops; exact canonical text remains the field authority.
        let mut styles: Vec<Arc<str>> = Vec::new();
        let mut style_members = 0;
        for line in lines {
            b.guard(40 * (result.len() + 1) + style_members + styles.len() + line.len())?;
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() != 18 {
                return Err(b.fail(
                    FailureKind::InvalidInput,
                    "incomplete source triangle record",
                ));
            }
            let component = fields[0]
                .parse::<u32>()
                .map_err(|_| b.fail(FailureKind::InvalidInput, "invalid component identity"))?;
            let facet = fields[1]
                .parse::<u32>()
                .map_err(|_| b.fail(FailureKind::InvalidInput, "invalid facet identity"))?;
            let text_style = decode_style(fields[2], b)?;
            let mut shared_style = None;
            for style in &styles {
                b.step(1)?;
                if style.as_ref() == text_style.as_str() {
                    shared_style = Some(Arc::clone(style));
                    break;
                }
            }
            let style = match shared_style {
                Some(style) => style,
                None => {
                    b.guard(
                        40 * (result.len() + 1)
                            + style_members
                            + text_style.len()
                            + 2
                            + styles.len()
                            + 1
                            + line.len(),
                    )?;
                    style_members += text_style.len() + 2;
                    let style: Arc<str> = Arc::from(text_style);
                    styles.push(Arc::clone(&style));
                    style
                }
            };
            let mut values = Vec::with_capacity(15);
            for value in &fields[3..] {
                let (n, d) = value
                    .split_once('/')
                    .ok_or_else(|| b.fail(FailureKind::InvalidInput, "incomplete exact scalar"))?;
                values.push(fraction(n, d, b)?);
            }
            let vertex = |i: usize| Vertex {
                point: [
                    values[i].clone(),
                    values[i + 1].clone(),
                    values[i + 2].clone(),
                ],
                uv: [values[i + 3].clone(), values[i + 4].clone()],
            };
            result.push(Facet::with_shared_style(
                SourceId { component, facet },
                [vertex(0), vertex(5), vertex(10)],
                style,
                b,
            )?);
        }
        if result.len() != self.original_triangle_count() {
            return Err(b.fail(FailureKind::InvalidInput, "lost original triangles"));
        }
        Ok(result)
    }
}
fn decode_style(text: &str, b: &mut Budget) -> Result<String, Failure> {
    if !text.len().is_multiple_of(2) {
        return Err(b.fail(FailureKind::InvalidInput, "incomplete style byte"));
    }
    let digit = |v: u8| match v {
        b'0'..=b'9' => Some(v - b'0'),
        b'a'..=b'f' => Some(v - b'a' + 10),
        _ => None,
    };
    let mut result = Vec::new();
    for pair in text.as_bytes().as_chunks::<2>().0 {
        b.step(1)?;
        let first = digit(pair[0])
            .ok_or_else(|| b.fail(FailureKind::InvalidInput, "invalid style digit"))?;
        let second = digit(pair[1])
            .ok_or_else(|| b.fail(FailureKind::InvalidInput, "invalid style digit"))?;
        result.push(first * 16 + second);
    }
    String::from_utf8(result).map_err(|_| b.fail(FailureKind::InvalidInput, "invalid style UTF-8"))
}
