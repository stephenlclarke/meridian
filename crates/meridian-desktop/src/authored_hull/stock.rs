//! Authenticated stock packages use the shared presentation loader without entering the local registry.
use super::{
    AuthoredVisual, AvailableModel, HullPackage, ModelPreview, fingerprint, package_presentation,
};
use std::sync::OnceLock;

pub(crate) const SPARROWHAWK_GEOMETRY_KEY: &str =
    "integration/sh030-sparrowhawk/SPARROWHAWK-STUDIO.json";
const SPARROWHAWK_PACKAGE: &[u8] =
    include_bytes!("../../../../integration/sh030-sparrowhawk/SPARROWHAWK-STUDIO.json");
const SPARROWHAWK_FNV: u64 = 0x23b9_dcfc_ce0b_c49c;
pub(crate) const KITE_GEOMETRY_KEY: &str =
    "integration/sh031-kite-interceptor/KITE-INTERCEPTOR-STUDIO.json";
const KITE_PACKAGE: &[u8] =
    include_bytes!("../../../../integration/sh031-kite-interceptor/KITE-INTERCEPTOR-STUDIO.json");
const KITE_FNV: u64 = 0x9209_a322_acda_f89d;

struct StockDescriptor {
    model: AvailableModel<'static>,
    name: &'static str,
    package: &'static [u8],
    fnv: u64,
}

// One descriptor registers a checked-in schema package throughout catalogue, visual lookup and cache.
const DESCRIPTORS: &[StockDescriptor] = &[
    StockDescriptor {
        model: AvailableModel {
            object: "SH030",
            geometry_key: SPARROWHAWK_GEOMETRY_KEY,
        },
        name: "Sparrowhawk",
        package: SPARROWHAWK_PACKAGE,
        fnv: SPARROWHAWK_FNV,
    },
    StockDescriptor {
        model: AvailableModel {
            object: "SH031",
            geometry_key: KITE_GEOMETRY_KEY,
        },
        name: "Kite Interceptor",
        package: KITE_PACKAGE,
        fnv: KITE_FNV,
    },
];
pub(crate) const STOCK_MODELS: [AvailableModel<'static>; DESCRIPTORS.len()] = {
    let mut models = [DESCRIPTORS[0].model; DESCRIPTORS.len()];
    let mut index = 0;
    while index < DESCRIPTORS.len() {
        models[index] = DESCRIPTORS[index].model;
        index += 1;
    }
    models
};

type StockCache = OnceLock<Result<AuthoredVisual, String>>;
static STOCK_CACHE: OnceLock<Vec<StockCache>> = OnceLock::new();

fn descriptor_for(model: &ModelPreview) -> Option<(usize, &'static StockDescriptor)> {
    DESCRIPTORS.iter().enumerate().find(|(_, descriptor)| {
        descriptor.model.object == model.object
            && descriptor.model.geometry_key == model.geometry_key
    })
}

pub(crate) fn stock_matches(model: &ModelPreview) -> bool {
    descriptor_for(model).is_some()
}

pub(crate) fn stock_visual_for(
    model: &ModelPreview,
) -> Result<Option<&'static AuthoredVisual>, String> {
    let Some((index, descriptor)) = descriptor_for(model) else {
        return Ok(None);
    };
    let cache =
        STOCK_CACHE.get_or_init(|| (0..DESCRIPTORS.len()).map(|_| OnceLock::new()).collect());
    match cache[index].get_or_init(|| load_stock(descriptor, descriptor.package)) {
        Ok(visual) => Ok(Some(visual)),
        Err(error) => Err(error.clone()),
    }
}

fn load_stock(descriptor: &StockDescriptor, bytes: &[u8]) -> Result<AuthoredVisual, String> {
    if fingerprint(bytes) != descriptor.fnv {
        return Err(format!(
            "{} immutable stock package changed",
            descriptor.model.object
        ));
    }
    let package = HullPackage::from_json(bytes).map_err(|error| error.to_string())?;
    if package.id() != descriptor.model.object || package.name() != descriptor.name {
        return Err(format!(
            "{} stock package identity changed",
            descriptor.model.object
        ));
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

    fn descriptor(id: &str) -> &'static StockDescriptor {
        DESCRIPTORS
            .iter()
            .find(|entry| entry.model.object == id)
            .unwrap_or_else(|| panic!("missing {id}"))
    }
    fn load_sparrowhawk(bytes: &[u8]) -> Result<AuthoredVisual, String> {
        load_stock(descriptor("SH030"), bytes)
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
        assert_original_corners(&visual, original, [0., 1.35, 0.], 25.0, 9324, 1216);
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

    fn assert_original_corners(
        visual: &AuthoredVisual,
        original: &[u8],
        center: [f64; 3],
        extent: f64,
        corners: usize,
        unmapped_faces: usize,
    ) {
        assert_eq!(visual.bytes().len(), corners * 56);
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
            for (axis, center) in center.into_iter().enumerate() {
                let expected = (f64::from(number(source, axis * 4)) - center) / extent * 32.0;
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
        assert_eq!(unmapped, unmapped_faces * 3);
        assert!(emitters > 0);
    }

    #[test]
    fn stock_kite_package_binds_every_original_corner_own_support_group_and_refuses_peer_payloads()
    {
        let package = ok(HullPackage::from_json(KITE_PACKAGE));
        let original = ok(meridian_core::kite_interceptor::KiteInterceptorHullSource::bundled());
        assert_eq!(
            ok(package.pieces()).as_slice(),
            original.pieces().as_slice()
        );
        assert!(package.engineering.is_none());
        let kite = ok(load_stock(descriptor("SH031"), KITE_PACKAGE));
        let packed = include_bytes!("../../../../assets/fleet/sh031/visual.bin");
        assert_eq!(&packed[..8], b"KITEVIZ1");
        assert_original_corners(&kite, packed, [0.; 3], 39.0, 9324, 1216);
        assert!(load_stock(descriptor("SH031"), SPARROWHAWK_PACKAGE).is_err());
        assert!(load_stock(descriptor("SH030"), KITE_PACKAGE).is_err());
        assert!(
            load_stock(
                descriptor("SH031"),
                include_bytes!("../../../../assets/fleet/sh031/native-visual.bin")
            )
            .is_err()
        );
        let mut changed = KITE_PACKAGE.to_vec();
        changed[0] ^= 1;
        assert!(load_stock(descriptor("SH031"), &changed).is_err());
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
        assert_stock_and_local("SH030", SPARROWHAWK_GEOMETRY_KEY);
    }

    #[test]
    fn stock_and_explicit_local_kite_remain_independent_and_peer_cache_never_crosses() {
        assert_stock_and_local("SH031", KITE_GEOMETRY_KEY);
        let make = |id: &str, key: &str| ModelPreview {
            object: id.into(),
            geometry_key: key.into(),
            bounds: Rect::new(0., 0., 200., 200.),
            orientation: Orientation::default(),
        };
        let sparrow = make("SH030", SPARROWHAWK_GEOMETRY_KEY);
        let kite = make("SH031", KITE_GEOMETRY_KEY);
        let a = ok(stock_visual_for(&sparrow)).unwrap_or_else(|| panic!("Sparrowhawk"));
        let b = ok(stock_visual_for(&kite)).unwrap_or_else(|| panic!("Kite"));
        assert_ne!(a.bytes(), b.bytes());
        assert!(std::ptr::eq(
            a,
            ok(stock_visual_for(&sparrow)).unwrap_or_else(|| panic!("Sparrowhawk return"))
        ));
        assert!(std::ptr::eq(
            b,
            ok(stock_visual_for(&kite)).unwrap_or_else(|| panic!("Kite return"))
        ));
        for crossed in [
            make("SH030", KITE_GEOMETRY_KEY),
            make("SH031", SPARROWHAWK_GEOMETRY_KEY),
            make("LOCAL:SH031", KITE_GEOMETRY_KEY),
        ] {
            assert!(ok(stock_visual_for(&crossed)).is_none());
        }
    }

    fn assert_stock_and_local(id: &str, key: &str) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(key);
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
        let stock = model_for(id);
        let isolated = model_for(&format!("LOCAL:{id}"));
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
