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
        b.with_phase(crate::BudgetPhase::Decode, |b| {
            self.decode_records(bytes, b)
        })
    }
    fn decode_records(self, bytes: &[u8], b: &mut Budget) -> Result<Vec<Facet>, Failure> {
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
        let mut styles: Vec<(&str, Arc<str>)> = Vec::new();
        let mut style_members = 0;
        for line in lines {
            b.guard(40 * (result.len() + 1) + style_members + 4 * styles.len() + line.len())?;
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
            b.state([
                result.len(),
                styles.len(),
                style_members,
                fields.len(),
                line.len(),
                0,
                component as usize,
                facet as usize,
            ]);
            let mut shared_style = None;
            for (encoded, style) in &styles {
                b.step(1)?;
                if *encoded == fields[2] {
                    shared_style = Some(Arc::clone(style));
                    break;
                }
            }
            let style = match shared_style {
                Some(style) => style,
                None => {
                    let text_style = decode_style(fields[2], b)?;
                    b.guard(
                        40 * (result.len() + 1)
                            + style_members
                            + text_style.len()
                            + 2
                            + 4 * (styles.len() + 1)
                            + line.len(),
                    )?;
                    style_members += text_style.len() + 2;
                    let style: Arc<str> = Arc::from(text_style);
                    styles.push((fields[2], Arc::clone(&style)));
                    style
                }
            };
            let mut values = Vec::with_capacity(15);
            for value in &fields[3..] {
                b.state([
                    result.len(),
                    styles.len(),
                    style_members,
                    fields.len(),
                    line.len(),
                    values.len(),
                    component as usize,
                    facet as usize,
                ]);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Limits;
    use std::collections::BTreeSet;

    #[test]
    fn original_style_pool_preserves_exact_text_and_counts_only_shared_allocations()
    -> Result<(), Box<dyn std::error::Error>> {
        for asset in [
            OriginalAsset::Sh010,
            OriginalAsset::Sh011,
            OriginalAsset::Sh016,
            OriginalAsset::Sh017,
            OriginalAsset::Sh166,
        ] {
            let mut b = Budget::new(Limits::default())?;
            let faces = asset.decode(asset.pinned_bytes(), &mut b)?;
            // Independent pinned-byte oracle, not the pool or footprint helper.
            let text = std::str::from_utf8(asset.pinned_bytes())?;
            let mut expected_styles = BTreeSet::new();
            for (face, line) in faces.iter().zip(text.lines().skip(1)) {
                let hex = line
                    .split_whitespace()
                    .nth(2)
                    .ok_or("missing bound style")?;
                let (pairs, remainder) = hex.as_bytes().as_chunks::<2>();
                assert!(remainder.is_empty());
                let mut bytes = Vec::new();
                for pair in pairs {
                    bytes.push(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?);
                }
                let style = String::from_utf8(bytes)?;
                assert_eq!(face.style(), style);
                expected_styles.insert(style);
            }
            assert_eq!(expected_styles.len(), 6);
            let style_members = expected_styles.iter().map(String::len).sum::<usize>();
            assert_eq!(style_members, 228);
            let actual = Facet::input_memberships(&faces, &mut b)?;
            assert_eq!(
                actual,
                40 * faces.len() + 64 + style_members + 2 * expected_styles.len()
            );
            let before =
                40 * faces.len() + 64 + faces.iter().map(|f| f.style().len()).sum::<usize>();
            assert!(actual < before);
            eprintln!(
                "STYLE_INPUT_STORAGE {{\"fixture\":\"{}\",\"before\":{before},\"after\":{actual},\"operations\":{}}}",
                asset.id(),
                b.operations()
            );
        }
        Ok(())
    }

    #[test]
    fn equal_unshared_styles_and_shared_clones_have_distinct_storage_ownership()
    -> Result<(), Failure> {
        let mut b = Budget::new(Limits::default())?;
        let faces = OriginalAsset::Sh010.decode(OriginalAsset::Sh010.pinned_bytes(), &mut b)?;
        let face = &faces[0];
        let independent = Facet::new(
            face.id(),
            face.vertices().clone(),
            face.style().to_owned(),
            &mut b,
        )?;
        assert_eq!(face, &independent);
        let owned_clones = [independent.clone(), independent.clone()];
        assert_eq!(
            Facet::input_memberships(&owned_clones, &mut b)?,
            40 * 2 + 64 + 2 * face.style().len()
        );
        let separate_shared = Facet::with_shared_style(
            face.id(),
            face.vertices().clone(),
            Arc::from(face.style()),
            &mut b,
        )?;
        let separate_arcs = [face.clone(), separate_shared];
        assert_eq!(
            Facet::input_memberships(&separate_arcs, &mut b)?,
            40 * 2 + 64 + 2 * (face.style().len() + 2)
        );
        let unshared = [face.clone(), independent];
        let shared = [face.clone(), face.clone()];
        let base = 40 * 2 + 64;
        assert_eq!(
            Facet::input_memberships(&unshared, &mut b)?,
            base + 2 * face.style().len() + 2
        );
        assert_eq!(
            Facet::input_memberships(&shared, &mut b)?,
            base + face.style().len() + 2
        );
        let mut exact = Budget::new(Limits {
            storage_memberships: base + face.style().len() + 2 + 1,
            ..Limits::default()
        })?;
        assert_eq!(
            Facet::input_memberships(&shared, &mut exact)?,
            base + face.style().len() + 2
        );
        let mut short = Budget::new(Limits {
            storage_memberships: base + face.style().len() + 2,
            ..Limits::default()
        })?;
        assert_eq!(
            Facet::input_memberships(&shared, &mut short)
                .err()
                .map(|error| error.kind),
            Some(FailureKind::Storage)
        );
        assert_eq!(short.retained_memberships(), 0);
        assert!(short.peak_admitted_memberships() <= short.limits().storage_memberships);
        Ok(())
    }
}
