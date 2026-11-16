use crate::{Budget, Facet, Failure, FailureKind, SourceId, Vertex, fraction};

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
            Self::Sh010 => include_bytes!("../fixtures/SH010.facets"),
            Self::Sh011 => include_bytes!("../fixtures/SH011.facets"),
            Self::Sh016 => include_bytes!("../fixtures/SH016.facets"),
            Self::Sh017 => include_bytes!("../fixtures/SH017.facets"),
            Self::Sh166 => include_bytes!("../fixtures/SH166.facets"),
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
        for line in lines {
            b.guard(40 * (result.len() + 1) + line.len())?;
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
            let style = decode_style(fields[2], b)?;
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
            result.push(Facet::new(
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
    for pair in text.as_bytes().chunks_exact(2) {
        b.step(1)?;
        let first = digit(pair[0])
            .ok_or_else(|| b.fail(FailureKind::InvalidInput, "invalid style digit"))?;
        let second = digit(pair[1])
            .ok_or_else(|| b.fail(FailureKind::InvalidInput, "invalid style digit"))?;
        result.push(first * 16 + second);
    }
    String::from_utf8(result).map_err(|_| b.fail(FailureKind::InvalidInput, "invalid style UTF-8"))
}
