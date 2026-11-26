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
pub(crate) const STAG_GEOMETRY_KEY: &str = "integration/sh035-stag-liner/STAG-LINER-STUDIO.json";
const STAG_PACKAGE: &[u8] =
    include_bytes!("../../../../integration/sh035-stag-liner/STAG-LINER-STUDIO.json");
const STAG_FNV: u64 = 0x42ef_f6ef_984e_79ae;
const STAG_EXPORT_FNV: u64 = 0x8004_d37c_b9ba_bc90;

struct StockDescriptor {
    model: AvailableModel<'static>,
    name: &'static str,
    package: &'static [u8],
    fnv: u64,
    /// Some installed Studio exports exactly omit the reviewed source's terminal LF.
    /// When pinned, authenticate both representations and consume the actual export bytes.
    export_fnv: Option<u64>,
}

macro_rules! stock_package {
    ($id:literal, $name:literal, $path:literal, $fnv:literal) => {
        StockDescriptor {
            model: AvailableModel {
                object: $id,
                geometry_key: $path,
            },
            name: $name,
            package: include_bytes!(concat!("../../../../", $path)),
            fnv: $fnv,
            export_fnv: None,
        }
    };
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
        export_fnv: None,
    },
    StockDescriptor {
        model: AvailableModel {
            object: "SH031",
            geometry_key: KITE_GEOMETRY_KEY,
        },
        name: "Kite Interceptor",
        package: KITE_PACKAGE,
        fnv: KITE_FNV,
        export_fnv: None,
    },
    StockDescriptor {
        model: AvailableModel {
            object: "SH035",
            geometry_key: STAG_GEOMETRY_KEY,
        },
        name: "Stag Liner",
        package: STAG_PACKAGE,
        fnv: STAG_FNV,
        export_fnv: Some(STAG_EXPORT_FNV),
    },
    stock_package!(
        "SH042",
        "Saker",
        "integration/sh042-saker/SAKER-STUDIO.json",
        0xa5a7_2dc4_6ce5_3390
    ),
    stock_package!(
        "SH043",
        "Eagle Cruiser",
        "integration/sh043-eagle-cruiser/EAGLE-CRUISER-STUDIO.json",
        0x9d7f_ab16_fabc_7bb9
    ),
    stock_package!(
        "SH044",
        "Harpy Assault",
        "integration/sh044-harpy-assault/HARPY-ASSAULT-STUDIO.json",
        0x8c13_9724_2d88_54f2
    ),
    stock_package!(
        "SH045",
        "Erne Corvette",
        "integration/sh045-erne-corvette/ERNE-CORVETTE-STUDIO.json",
        0x3dcd_bebd_ec10_ccf7
    ),
    stock_package!(
        "SH046",
        "Harpy Dropship",
        "integration/sh046-harpy-dropship/HARPY-DROPSHIP-STUDIO.json",
        0x331a_7266_78b7_b074
    ),
    stock_package!(
        "SH047",
        "Harpy Gunship",
        "integration/sh047-harpy-gunship/HARPY-GUNSHIP-STUDIO.json",
        0x4e7e_88c1_58fb_fe30
    ),
    stock_package!(
        "SH048",
        "Lanner",
        "integration/sh048-lanner/LANNER-STUDIO.json",
        0x3055_5234_66fe_7509
    ),
    stock_package!(
        "SH049",
        "Hyena",
        "integration/sh049-hyena/HYENA-STUDIO.json",
        0x9e1e_d894_aee8_e15d
    ),
    stock_package!(
        "SH050",
        "Yak",
        "integration/sh050-yak/YAK-STUDIO.json",
        0x7351_3510_2967_69e9
    ),
    stock_package!(
        "SH051",
        "Jackal",
        "integration/sh051-jackal/JACKAL-STUDIO.json",
        0x6ca4_5ddf_d1d7_2231
    ),
    stock_package!(
        "SH052",
        "Banteng",
        "integration/sh052-banteng/BANTENG-STUDIO.json",
        0x1aca_d5cc_9eaf_7610
    ),
    stock_package!(
        "SH053",
        "Caracal Scout",
        "integration/sh053-caracal-scout/CARACAL-SCOUT-STUDIO.json",
        0xe6a5_ce33_45de_3a30
    ),
    stock_package!(
        "SH054",
        "Hobby Fighter",
        "integration/sh054-hobby-fighter/HOBBY-FIGHTER-STUDIO.json",
        0x415b_5ee1_58af_22cd
    ),
    stock_package!(
        "SH055",
        "Condor",
        "integration/sh055-condor/CONDOR-STUDIO.json",
        0x6381_baf1_98ec_4a1f
    ),
    stock_package!(
        "SH056",
        "Aplomado Interceptor",
        "integration/sh056-aplomado-interceptor/APLOMADO-INTERCEPTOR-STUDIO.json",
        0xe90c_aa74_abfd_b501
    ),
    stock_package!(
        "SH057",
        "Falconet",
        "integration/sh057-falconet/FALCONET-STUDIO.json",
        0x9b2d_5ada_1eb9_7374
    ),
    stock_package!(
        "SH058",
        "Muskox",
        "integration/sh058-muskox/MUSKOX-STUDIO.json",
        0xe1ec_ad3b_644e_bb3d
    ),
    stock_package!(
        "SH059",
        "Barbary",
        "integration/sh059-barbary/BARBARY-STUDIO.json",
        0x07ba_a797_995b_3ed8
    ),
    stock_package!(
        "SH060",
        "Wombat",
        "integration/sh060-wombat/WOMBAT-STUDIO.json",
        0xe341_60a7_3c44_79f0
    ),
    stock_package!(
        "SH061",
        "Saiga",
        "integration/sh061-saiga/SAIGA-STUDIO.json",
        0xe099_121c_2673_bb02
    ),
    stock_package!(
        "SH062",
        "Moose Clipper",
        "integration/sh062-moose-clipper/MOOSE-CLIPPER-STUDIO.json",
        0xbd45_df1a_e94b_0861
    ),
    stock_package!(
        "SH063",
        "Seriema Courier",
        "integration/sh063-seriema-courier/SERIEMA-COURIER-STUDIO.json",
        0x4870_3225_5433_2199
    ),
    stock_package!(
        "SH064",
        "Arna Cutter",
        "integration/sh064-arna-cutter/ARNA-CUTTER-STUDIO.json",
        0x5e86_bbc7_340b_ce59
    ),
    stock_package!(
        "SH065",
        "Vulture Raider",
        "integration/sh065-vulture-raider/VULTURE-RAIDER-STUDIO.json",
        0xed14_2b3f_c558_8f3e
    ),
    stock_package!(
        "SH066",
        "Rabbit Explorer",
        "integration/sh066-rabbit-explorer/RABBIT-EXPLORER-STUDIO.json",
        0xe187_53ed_92e9_a9b4
    ),
    stock_package!(
        "SH067",
        "Rhinoceros Trader",
        "integration/sh067-rhinoceros-trader/RHINOCEROS-TRADER-STUDIO.json",
        0xdbe0_0910_1625_6a8f
    ),
    stock_package!(
        "SH068",
        "Badger",
        "integration/sh068-badger/BADGER-STUDIO.json",
        0x96f6_fae5_829e_7274
    ),
    stock_package!(
        "SH069",
        "Baza Fighter",
        "integration/sh069-baza-fighter/BAZA-FIGHTER-STUDIO.json",
        0x6792_feef_26de_7571
    ),
    stock_package!(
        "SH070",
        "Baza Mk II",
        "integration/sh070-baza-mk-ii/BAZA-MK-II-STUDIO.json",
        0xcc38_b57c_bd7a_aaee
    ),
    stock_package!(
        "SH071",
        "Griffon",
        "integration/sh071-griffon/GRIFFON-STUDIO.json",
        0x5cc3_7883_3c9e_2d84
    ),
    stock_package!(
        "SH072",
        "Griffon Mk II",
        "integration/sh072-griffon-mk-ii/GRIFFON-MK-II-STUDIO.json",
        0x4f50_51bc_d510_ec9e
    ),
    stock_package!(
        "SH073",
        "Griffon Spectre",
        "integration/sh073-griffon-spectre/GRIFFON-SPECTRE-STUDIO.json",
        0xf80f_0554_1e38_ec7e
    ),
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

fn authenticated_package_bytes<'a>(
    descriptor: &StockDescriptor,
    bytes: &'a [u8],
) -> Result<&'a [u8], String> {
    if fingerprint(bytes) != descriptor.fnv {
        return Err(format!(
            "{} immutable stock package changed",
            descriptor.model.object
        ));
    }
    if let Some(expected) = descriptor.export_fnv {
        let export = bytes.strip_suffix(b"\n").ok_or_else(|| {
            format!(
                "{} qualified stock export projection missing",
                descriptor.model.object
            )
        })?;
        if fingerprint(export) != expected {
            return Err(format!(
                "{} qualified stock export changed",
                descriptor.model.object
            ));
        }
        return Ok(export);
    }
    Ok(bytes)
}

fn load_stock(descriptor: &StockDescriptor, bytes: &[u8]) -> Result<AuthoredVisual, String> {
    let package = HullPackage::from_json(authenticated_package_bytes(descriptor, bytes)?)
        .map_err(|error| error.to_string())?;
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
    fn every_stock_descriptor_authenticates_its_own_source_and_real_preview() {
        let catalogue = ok(crate::docked_screens::Catalogue::bundled());
        assert_eq!(STOCK_MODELS.len(), 35);
        let mut identities = std::collections::BTreeSet::new();
        let mut keys = std::collections::BTreeSet::new();
        for entry in DESCRIPTORS {
            let id = entry.model.object;
            assert!(identities.insert(id), "duplicate stock identity {id}");
            assert!(
                keys.insert(entry.model.geometry_key),
                "duplicate stock geometry for {id}"
            );
            let catalogue_record = catalogue
                .object(id)
                .unwrap_or_else(|| panic!("missing catalogue identity {id}"));
            assert_eq!(catalogue_record.id, id);
            let raw = ok(authenticated_package_bytes(entry, entry.package));
            let package = ok(HullPackage::from_json(raw));
            assert_eq!(package.id(), id);
            assert_eq!(package.name(), entry.name);
            let preview = ModelPreview {
                object: id.into(),
                geometry_key: entry.model.geometry_key.into(),
                bounds: Rect::new(0., 0., 200., 200.),
                orientation: Orientation::default(),
            };
            let visual = stock_visual_for(&preview)
                .unwrap_or_else(|error| panic!("{id}: {error}"))
                .unwrap_or_else(|| panic!("missing {id} visual"));
            assert_eq!(
                visual.bytes().len(),
                package.detail.triangles.len() * 3 * 56,
                "{id}"
            );
            let mut crossed = preview;
            crossed.object = format!("LOCAL:{id}");
            assert!(ok(stock_visual_for(&crossed)).is_none());
        }
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
    fn stock_stag_binds_qualified_export_original_corners_own_pieces_and_refuses_peer_payloads() {
        let entry = descriptor("SH035");
        let export = ok(authenticated_package_bytes(entry, STAG_PACKAGE));
        assert_eq!(export.len(), 451470);
        assert_eq!(fingerprint(export), STAG_EXPORT_FNV);
        assert_eq!(
            export,
            STAG_PACKAGE
                .strip_suffix(b"\n")
                .unwrap_or_else(|| panic!("missing source LF"))
        );
        let package = ok(HullPackage::from_json(export));
        assert_eq!(package, ok(HullPackage::from_json(STAG_PACKAGE)));
        assert!(package.engineering.is_none());
        let original = ok(meridian_core::stag_liner::StagLinerHullSource::bundled());
        assert_eq!(
            ok(package.pieces()).as_slice(),
            original.pieces().as_slice()
        );
        let visual = ok(load_stock(entry, STAG_PACKAGE));
        let packed = include_bytes!("../../../../assets/fleet/sh035/visual.bin");
        assert_eq!(&packed[..8], b"STAGVIZ1");
        assert_original_corners(&visual, packed, [0., 4.25, -1.545], 69.09, 10476, 1240);
        for peer in ["SH030", "SH031"] {
            let d = descriptor(peer);
            assert!(load_stock(entry, d.package).is_err());
            assert!(load_stock(d, STAG_PACKAGE).is_err());
        }
        assert!(
            load_stock(
                entry,
                include_bytes!("../../../../assets/fleet/sh035/native-visual.bin")
            )
            .is_err()
        );
        assert!(
            load_stock(entry, export).is_err(),
            "raw source pin must hold even for raw export bytes"
        );
        let mut changed = STAG_PACKAGE.to_vec();
        changed[0] ^= 1;
        assert!(load_stock(entry, &changed).is_err());
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

    #[test]
    fn stock_and_explicit_local_stag_remain_independent_from_all_stock_peers() {
        assert_stock_and_local("SH035", STAG_GEOMETRY_KEY);
        let make = |id: &str, key: &str| ModelPreview {
            object: id.into(),
            geometry_key: key.into(),
            bounds: Rect::new(0., 0., 200., 200.),
            orientation: Orientation::default(),
        };
        let stag = make("SH035", STAG_GEOMETRY_KEY);
        let original = ok(stock_visual_for(&stag)).unwrap_or_else(|| panic!("Stag"));
        for (id, key) in [
            ("SH030", SPARROWHAWK_GEOMETRY_KEY),
            ("SH031", KITE_GEOMETRY_KEY),
        ] {
            let peer = make(id, key);
            assert_ne!(
                original.bytes(),
                ok(stock_visual_for(&peer))
                    .unwrap_or_else(|| panic!("peer"))
                    .bytes()
            );
            assert!(ok(stock_visual_for(&make("SH035", key))).is_none());
            assert!(ok(stock_visual_for(&make(id, STAG_GEOMETRY_KEY))).is_none());
            assert!(std::ptr::eq(
                original,
                ok(stock_visual_for(&stag)).unwrap_or_else(|| panic!("Stag return"))
            ));
        }
        assert!(ok(stock_visual_for(&make("LOCAL:SH035", STAG_GEOMETRY_KEY))).is_none());
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
