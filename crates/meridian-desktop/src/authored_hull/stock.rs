//! Authenticated stock packages use the shared presentation loader without entering the local registry.
use super::{AuthoredVisual, HullPackage, ModelPreview, fingerprint, package_presentation};
use std::sync::OnceLock;

pub(crate) const SPARROWHAWK_GEOMETRY_KEY: &str =
    "integration/sh030-sparrowhawk/SPARROWHAWK-STUDIO.json";
const SPARROWHAWK_PACKAGE: &[u8] =
    include_bytes!("../../../../integration/sh030-sparrowhawk/SPARROWHAWK-STUDIO.json");
const SPARROWHAWK_FNV: u64 = 0x23b9_dcfc_ce0b_c49c;
static SPARROWHAWK: OnceLock<Result<AuthoredVisual, String>> = OnceLock::new();

pub(crate) fn stock_matches(model: &ModelPreview) -> bool {
    model.object == "SH030" && model.geometry_key == SPARROWHAWK_GEOMETRY_KEY
}

pub(crate) fn stock_visual_for(
    model: &ModelPreview,
) -> Result<Option<&'static AuthoredVisual>, String> {
    if !stock_matches(model) {
        return Ok(None);
    }
    match SPARROWHAWK.get_or_init(|| load_sparrowhawk(SPARROWHAWK_PACKAGE)) {
        Ok(visual) => Ok(Some(visual)),
        Err(error) => Err(error.clone()),
    }
}

fn load_sparrowhawk(bytes: &[u8]) -> Result<AuthoredVisual, String> {
    if fingerprint(bytes) != SPARROWHAWK_FNV {
        return Err("SH030 immutable stock package changed".into());
    }
    let package = HullPackage::from_json(bytes).map_err(|error| error.to_string())?;
    if package.id() != "SH030" || package.name() != "Sparrowhawk" {
        return Err("SH030 stock package identity changed".into());
    }
    // Same core preflight, pieces, clearance and material/UV encoder as the isolated Studio.
    package_presentation(&package).map(|(visual, _)| visual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docked_screens::{Orientation, Rect};

    fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
        value.unwrap_or_else(|error| panic!("{error:?}"))
    }

    #[test]
    fn stock_sparrowhawk_package_binds_every_original_corner_and_own_support_group() {
        let package = ok(HullPackage::from_json(SPARROWHAWK_PACKAGE));
        let stock = ok(meridian_core::sparrowhawk::SparrowhawkHullSource::bundled());
        assert_eq!(ok(package.pieces()).as_slice(), stock.pieces().as_slice());
        assert!(package.engineering.is_none());
        let visual = ok(load_sparrowhawk(SPARROWHAWK_PACKAGE));
        let original = include_bytes!("../../../../assets/fleet/sh030/visual.bin");
        assert_eq!(&original[..8], b"SPRHVIZ1");
        assert_eq!(visual.bytes().len(), 9324 * 56);
        assert_eq!(original.len(), 16 + visual.bytes().len());
        let number = |record: &[u8], offset: usize| {
            f32::from_le_bytes(
                record[offset..offset + 4]
                    .try_into()
                    .unwrap_or_else(|_| panic!("corner record")),
            )
        };
        let mut unmapped = 0;
        let mut emitters = 0;
        for (actual, source) in visual
            .bytes()
            .chunks_exact(56)
            .zip(original[16..].chunks_exact(56))
        {
            assert_eq!(
                &actual[24..56],
                &source[24..56],
                "original tint/alpha/UV/texture/emission changed"
            );
            for (axis, center) in [0., 1.35, 0.].into_iter().enumerate() {
                let expected = (f64::from(number(source, axis * 4)) - center) / 25.0 * 32.0;
                assert!((f64::from(number(actual, axis * 4)) - expected).abs() < 4e-6);
                assert!(
                    (number(actual, 12 + axis * 4) - number(source, 12 + axis * 4)).abs() < 1e-6
                );
            }
            if actual[48..52] == u32::MAX.to_le_bytes() {
                unmapped += 1;
            }
            if actual[52..56] == 1u32.to_le_bytes() {
                emitters += 1;
            }
        }
        assert_eq!(unmapped, 1216 * 3);
        assert!(emitters > 0);
        let mut changed = SPARROWHAWK_PACKAGE.to_vec();
        changed[0] ^= 1;
        assert!(load_sparrowhawk(&changed).is_err());
        assert!(
            load_sparrowhawk(include_bytes!(
                "../../../../assets/fleet/sh030/native-visual.bin"
            ))
            .is_err()
        );
    }

    #[test]
    fn stock_sparrowhawk_refuses_local_or_crossed_identity_and_geometry_key() {
        let mut stock = ModelPreview {
            object: "SH030".into(),
            geometry_key: SPARROWHAWK_GEOMETRY_KEY.into(),
            bounds: Rect::new(0., 0., 200., 200.),
            orientation: Orientation::default(),
        };
        assert!(ok(stock_visual_for(&stock)).is_some());
        stock.object = "LOCAL:SH030".into();
        assert!(ok(stock_visual_for(&stock)).is_none());
        stock.object = "SH030".into();
        stock.geometry_key = "assets/fleet/sh023/visual.bin".into();
        assert!(ok(stock_visual_for(&stock)).is_none());
        stock.object = "SH023".into();
        stock.geometry_key = SPARROWHAWK_GEOMETRY_KEY.into();
        assert!(ok(stock_visual_for(&stock)).is_none());
    }

    #[test]
    fn stock_and_explicit_local_sparrowhawk_remain_independent_session_identities() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(SPARROWHAWK_GEOMETRY_KEY);
        assert!(
            super::super::AuthoredHulls::load(&[super::super::PackagePath::new(
                path.clone(),
                false
            )])
            .is_err()
        );
        let local = ok(super::super::AuthoredHulls::load(&[
            super::super::PackagePath::new(path, true),
        ]));
        let model_for = |id: &str| {
            let models = local.available_models();
            let selected = models
                .iter()
                .find(|model| model.object == id)
                .unwrap_or_else(|| panic!("missing {id}"));
            ModelPreview {
                object: id.into(),
                geometry_key: selected.geometry_key.into(),
                bounds: Rect::new(0., 0., 200., 200.),
                orientation: Orientation::default(),
            }
        };
        let stock = model_for("SH030");
        let isolated = model_for("LOCAL:SH030");
        assert_ne!(stock.geometry_key, isolated.geometry_key);
        let original = ok(stock_visual_for(&stock)).unwrap_or_else(|| panic!("stock"));
        let inspection = local
            .visual_for(&isolated)
            .unwrap_or_else(|| panic!("local"));
        assert_eq!(original.bytes(), inspection.bytes());
        assert!(ok(stock_visual_for(&isolated)).is_none());
        assert!(local.visual_for(&stock).is_none());
    }
}
