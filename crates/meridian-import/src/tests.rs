use crate::{
    Artifact, ImportError, Model, Result, encode, import_all, import_ship, import_station,
    palette::LINEAR_BITS,
    sha256,
    source::{self, Design, Material, Point, Port},
};

const SHIP: &[u8] = include_bytes!("../fixtures/SH025.toml");
const STATION: &[u8] = include_bytes!("../fixtures/SH028.toml");

fn mesh() -> Design {
    Design {
        vertices: vec![[-0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        faces: vec![[0, 1, 2]],
        face_materials: vec![0],
        materials: vec![Material { colour: 0xff8000ff }],
        license: "MIT".to_owned(),
        appearance_license: "MIT".to_owned(),
    }
}

fn port() -> Port {
    Port {
        center: Point {
            x: 0.0,
            y: 0.0,
            z: 10.0,
        },
        outward: Point {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        up: Point {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        tunnel_depth: 2.0,
        aperture: vec![
            Point {
                x: -1.0,
                y: -1.0,
                z: 0.0,
            },
            Point {
                x: 1.0,
                y: -1.0,
                z: 0.0,
            },
            Point {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        ],
    }
}

#[test]
fn all_five_conversions_equal_independently_committed_binary_fixtures() -> Result<()> {
    let batch = import_all(SHIP, STATION)?;
    let fixtures: [(&str, &[u8]); 5] = [
        (
            "sh025.mesh",
            include_bytes!("../../../assets/legacy/sh025.mesh"),
        ),
        (
            "sh028.mesh",
            include_bytes!("../../../assets/legacy/sh028.mesh"),
        ),
        (
            "sh025.physics",
            include_bytes!("../../../assets/legacy/sh025.physics"),
        ),
        (
            "sh028.physics",
            include_bytes!("../../../assets/legacy/sh028.physics"),
        ),
        (
            "sh028.port",
            include_bytes!("../../../assets/legacy/sh028.port"),
        ),
    ];
    assert_eq!(batch.artifacts().len(), 5);
    for (name, expected) in fixtures {
        let actual = batch
            .artifacts()
            .iter()
            .find(|a| a.name() == name)
            .ok_or(ImportError::Geometry("Missing artifact"))?;
        assert_eq!(actual.bytes(), expected, "{name}");
    }
    Ok(())
}

#[test]
fn ship_and_station_modes_keep_their_original_output_sets() -> Result<()> {
    let ship = import_ship(SHIP)?;
    let station = import_station(SHIP, STATION)?;
    assert_eq!(
        ship.artifacts()
            .iter()
            .map(Artifact::name)
            .collect::<Vec<_>>(),
        ["sh025.mesh"]
    );
    assert_eq!(
        station
            .artifacts()
            .iter()
            .map(Artifact::name)
            .collect::<Vec<_>>(),
        ["sh028.mesh", "sh028.physics", "sh025.physics", "sh028.port"]
    );
    Ok(())
}

#[test]
fn source_hashes_refuse_foreign_bytes_before_decoding() {
    for raw in [b"not TOML".as_slice(), b"\xff\x00".as_slice()] {
        assert!(matches!(import_ship(raw), Err(ImportError::Identity(_))));
        assert!(matches!(
            import_station(SHIP, raw),
            Err(ImportError::Identity(_))
        ));
    }
    let mut same_size = SHIP.to_vec();
    same_size[0] ^= 1;
    assert!(matches!(
        import_ship(&same_size),
        Err(ImportError::Identity(_))
    ));
    assert!(import_all(STATION, SHIP).is_err());
}

#[test]
fn malformed_utf8_toml_and_missing_schema_are_refused() {
    for raw in [
        b"\xff".as_slice(),
        b"id = [".as_slice(),
        b"id = 'SH025'\n".as_slice(),
    ] {
        assert!(source::parse_document(raw).is_err());
    }
}

#[test]
fn malformed_numeric_face_material_and_vertex_shapes_are_refused() {
    let minimal = "id='x'\nname='x'\n[design]\nlicense='MIT'\nappearanceLicense='MIT'\nvertices=[[0.0,0.0,0.0],[1.0,0.0,0.0],[0.0,1.0,0.0]]\nfaces=[[0,1,2]]\nfaceMaterials=[0]\nmaterials=[{colour=255}]\n[nativeDesign]\nlicense='MIT'\nappearanceLicense='MIT'\nvertices=[[0.0,0.0,0.0]]\nfaces=[[0,0,0]]\nfaceMaterials=[0]\nmaterials=[{colour=255}]\n";
    assert!(source::parse_document(minimal.as_bytes()).is_ok());
    for (before, after) in [
        ("faces=[[0,1,2]]", "faces=[[0,1]]"),
        ("faces=[[0,1,2]]", "faces=[[0,1,true]]"),
        ("faces=[[0,1,2]]", "faces=[[0,1,2.5]]"),
        ("faces=[[0,1,2]]", "faces=[[0,1,-1]]"),
        ("colour=255", "colour=4294967296"),
        ("colour=255", "colour=-1"),
        ("vertices=[[0.0,0.0,0.0]", "vertices=[[0.0,0.0]"),
    ] {
        assert!(
            source::parse_document(minimal.replace(before, after).as_bytes()).is_err(),
            "{after}"
        );
    }
}

#[test]
fn parsed_identity_and_selected_geometry_counts_cannot_be_substituted() -> Result<()> {
    let original = source::parse_document(SHIP)?;
    let mut wrong = original.clone();
    wrong.id = "SH028".to_owned();
    assert!(source::validate_identity(Model::Sh025, &wrong).is_err());
    let mut wrong = original.clone();
    wrong.name = "Foreign".to_owned();
    assert!(source::validate_identity(Model::Sh025, &wrong).is_err());
    let mut wrong = original.clone();
    wrong.design.faces.pop();
    assert!(source::validate_identity(Model::Sh025, &wrong).is_err());
    let mut wrong = original;
    wrong.native_design.vertices.pop();
    assert!(source::validate_identity(Model::Sh025, &wrong).is_err());
    Ok(())
}

#[test]
fn both_source_notice_declarations_are_required() {
    let mut design = mesh();
    design.license = "unknown".to_owned();
    assert!(matches!(design.validate(), Err(ImportError::Identity(_))));
    let mut design = mesh();
    design.appearance_license = "unknown".to_owned();
    assert!(matches!(design.validate(), Err(ImportError::Identity(_))));
}

#[test]
fn all_vertices_including_unused_positions_must_be_finite_and_bounded() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0e13] {
        let mut design = mesh();
        design.vertices.push([invalid, 0.0, 0.0]);
        assert!(design.validate().is_err());
        assert!(encode::display(&design).is_err());
        assert!(encode::physical(&design).is_err());
    }
    let mut design = mesh();
    design.vertices.clear();
    assert!(design.validate().is_err());
    let mut design = mesh();
    design.faces.clear();
    assert!(design.validate().is_err());
}

#[test]
fn triangle_indices_cannot_escape_the_source_vertices() {
    for bad in [3, u32::MAX] {
        let mut design = mesh();
        design.faces[0][1] = bad;
        assert!(encode::display(&design).is_err());
        assert!(encode::physical(&design).is_err());
    }
}

#[test]
fn missing_extra_and_out_of_range_face_materials_refuse() {
    let mut design = mesh();
    design.face_materials.clear();
    assert!(design.validate().is_err());
    let mut design = mesh();
    design.face_materials.push(0);
    assert!(design.validate().is_err());
    let mut design = mesh();
    design.face_materials[0] = 1;
    assert!(design.validate().is_err());
    let mut design = mesh();
    design.materials.clear();
    assert!(design.validate().is_err());
}

#[test]
fn binary_layout_preserves_little_endian_values_counts_and_signed_zero() -> Result<()> {
    let physical = encode::physical(&mesh())?;
    assert_eq!(&physical[..8], b"MPHYS001");
    assert_eq!(&physical[8..16], &[3, 0, 0, 0, 1, 0, 0, 0]);
    assert_eq!(&physical[16..24], &(-0.0f64).to_le_bytes());
    assert_eq!(physical.len(), 16 + 3 * 24 + 12);
    assert_eq!(
        &physical[physical.len() - 12..],
        &[0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]
    );
    let display = encode::display(&mesh())?;
    assert_eq!(display.len(), 72);
    assert_eq!(&display[..4], &(-0.0f32).to_le_bytes());
    assert_eq!(&display[12..16], &1.0f32.to_le_bytes());
    Ok(())
}

#[test]
fn palette_is_the_frozen_256_entry_python_formula_not_platform_pow() {
    let bytes: Vec<_> = LINEAR_BITS
        .iter()
        .flat_map(|bits| bits.to_le_bytes())
        .collect();
    assert_eq!(
        sha256(&bytes),
        "48a8f05136456237aae2c5349e5f0199a182daa19b7c099eb542058f794b9c5b"
    );
    assert_eq!(LINEAR_BITS[0], 0);
    assert_eq!(LINEAR_BITS[255], 1.0f32.to_bits());
    assert!(
        LINEAR_BITS
            .windows(2)
            .all(|pair| f32::from_bits(pair[0]) < f32::from_bits(pair[1]))
    );
}

#[test]
fn display_keeps_source_triangle_order_and_existing_alpha_omission() -> Result<()> {
    let first = encode::display(&mesh())?;
    let mut changed = mesh();
    changed.materials[0].colour = 0xff800000;
    assert_eq!(encode::display(&changed)?, first);
    changed.faces[0] = [2, 1, 0];
    let reversed = encode::display(&changed)?;
    assert_eq!(&reversed[..24], &first[48..72]);
    assert_eq!(&reversed[48..72], &first[..24]);
    Ok(())
}

#[test]
fn port_layout_has_original_frame_depth_and_aperture_without_fallback() -> Result<()> {
    let bytes = encode::port(&port())?;
    assert_eq!(&bytes[..8], b"MPORT001");
    assert_eq!(&bytes[72..80], &0.0f64.to_le_bytes());
    assert_eq!(&bytes[80..88], &2.0f64.to_le_bytes());
    assert_eq!(&bytes[88..92], &[3, 0, 0, 0]);
    assert_eq!(bytes.len(), 92 + 3 * 24);
    Ok(())
}

#[test]
fn nonfinite_short_and_nonpositive_station_ports_refuse() {
    for depth in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut p = port();
        p.tunnel_depth = depth;
        assert!(encode::port(&p).is_err());
    }
    let mut p = port();
    p.center.x = f64::NAN;
    assert!(encode::port(&p).is_err());
    let mut p = port();
    p.aperture[0].x = f64::INFINITY;
    assert!(encode::port(&p).is_err());
    let mut p = port();
    p.aperture.truncate(2);
    assert!(encode::port(&p).is_err());
}

#[test]
fn malformed_port_axes_refuse_instead_of_getting_normalized() {
    let mut p = port();
    p.outward.z = 2.0;
    assert!(encode::port(&p).is_err());
    let mut p = port();
    p.up = p.outward.clone();
    assert!(encode::port(&p).is_err());
}

#[test]
fn selected_station_cannot_drop_its_original_port() -> Result<()> {
    let mut station = source::parse_document(STATION)?;
    station.docking = None;
    assert!(source::validate_document(Model::Sh028, &station).is_err());
    Ok(())
}

#[test]
fn selected_git_specs_are_fixed_revision_paths() {
    assert_eq!(
        Model::Sh025.git_spec(),
        "a11fe16924d87d06021816f9037952afcbac08db:Sources/L337Core/Resources/Ships/SH025.toml"
    );
    assert_eq!(
        Model::Sh028.git_spec(),
        "a11fe16924d87d06021816f9037952afcbac08db:Sources/L337Core/Resources/Ships/SH028.toml"
    );
}

#[test]
fn converted_output_hash_guard_refuses_unknown_or_altered_artifacts() {
    assert!(Artifact::verified("foreign.mesh", vec![]).is_err());
    assert!(Artifact::verified("sh025.mesh", vec![0; 250848]).is_err());
}

#[test]
fn rust_receipt_is_deterministic_and_distinct_from_legacy_producer_records() -> Result<()> {
    let one = import_all(SHIP, STATION)?;
    let two = import_all(SHIP, STATION)?;
    assert_eq!(one.receipt(), two.receipt());
    let json: serde_json::Value =
        serde_json::from_slice(one.receipt()).map_err(|e| ImportError::Decode(e.to_string()))?;
    assert_eq!(json["producer"]["name"], "meridian-import");
    assert_eq!(
        json["sourceRevision"],
        "a11fe16924d87d06021816f9037952afcbac08db"
    );
    assert_eq!(json["artifacts"].as_array().map(Vec::len), Some(5));
    assert_eq!(
        json["sourceNotice"]["sha256"],
        "033b7058d73f22c3c6b8a5825eb866ca2d67088f3ee94dd5f3e46da59d624766"
    );
    assert!(json.get("timestamp").is_none());
    assert!(
        json["legacyRecipeReferences"]
            .get("tools/import_ship.py")
            .is_some()
    );
    Ok(())
}
