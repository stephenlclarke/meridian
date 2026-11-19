use super::*;

fn package() -> HullPackage {
    let mesh = Mesh {
        vertices: vec![[0., 0., 0.], [2., 0., 0.], [0., 2., 0.], [0., 0., 2.]],
        triangles: vec![[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]],
        groups: vec![123_456; 4],
    };
    HullPackage {
        schema: 1,
        id: "my-new-hull".into(),
        name: "New hull".into(),
        metres_per_unit: 2.0,
        rounding_metres: 0.02,
        detail: mesh.clone(),
        native: mesh,
        materials: vec![Material {
            name: "blue".into(),
            rgb: [10, 20, 30],
            alpha: 255,
            emissive: false,
        }],
        face_materials: vec![0; 4],
        face_textures: None,
        engineering: Some(Engineering {
            mass_kg: 50.,
            center_of_mass: [1., 1., 1.],
            inertia: [[10., 0., 0.], [0., 12., 0.], [0., 0., 13.]],
        }),
        flight: Flight {
            max_speed: Some(20.),
        },
        clearances: vec![Clearance {
            name: "clear".into(),
            start: [-1., -2., 0.],
            end: [5., -2., 0.],
            radius_metres: 0.5,
        }],
        provenance: None,
    }
}

#[test]
fn arbitrary_id_and_local_groups_share_real_metres_across_lods() {
    let mut source = package();
    source.native.groups.fill(9);
    let report = validate_package(&source);
    assert!(report.valid, "{:?}", report.diagnostics);
    assert_eq!(
        report.clearances,
        vec![ClearanceResult {
            name: "clear".into(),
            passed: true
        }]
    );
    assert_eq!(
        report
            .support_groups()
            .iter()
            .map(|g| g.id)
            .collect::<Vec<_>>(),
        [9, 123_456]
    );
    assert_eq!(report.support_groups()[0].points()[1], [4., 0., 0.]);
    source.native.groups.fill(123_456);
    let report = validate_package(&source);
    assert!(report.valid);
    assert_eq!(report.support_groups()[0].points().len(), 8);
    assert_eq!(source.id(), "my-new-hull");
    assert_eq!(source.name(), "New hull");
    let Ok(pieces) = source.pieces() else {
        panic!("valid source must expose pieces")
    };
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].source_points().len(), 8);
}

#[test]
fn continuous_route_rejects_midpath_and_radius_contact() {
    let mut source = package();
    source.clearances[0].start = [-1., 0.5, 0.5];
    source.clearances[0].end = [5., 0.5, 0.5];
    let report = validate_package(&source);
    assert!(!report.valid && !report.clearances[0].passed);
    assert!(!report.support_groups().is_empty());
    assert!(source.pieces().is_err());
    source.clearances[0].start = [-1., -0.3, 0.];
    source.clearances[0].end = [5., -0.3, 0.];
    assert!(!validate_package(&source).valid);
    source.clearances[0].radius_metres = 0.1;
    assert!(validate_package(&source).valid);
    source.clearances[0].start = [0., 0., 0.];
    source.clearances[0].end = [0., 0., 0.];
    source.clearances[0].radius_metres = 0.;
    assert!(!validate_package(&source).valid);
}

#[test]
fn malformed_geometry_groups_and_materials_fail_closed() {
    let mut bad = package();
    bad.detail.triangles[0][2] = 99;
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    bad.native.groups[0] = 1;
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    bad.native.groups.pop();
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    bad.detail.vertices[0][0] = f64::NAN;
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    bad.detail.triangles[0] = [0, 0, 1];
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    bad.detail.vertices[2] = [1., 0., 0.];
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    bad.face_materials[0] = 1;
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    bad.native.triangles = vec![[0, 1, 2]; 513];
    assert!(!validate_package(&bad).valid);
}

#[test]
fn invalid_engineering_and_schema_never_authorize() {
    let mut bad = package();
    if let Some(engineering) = &mut bad.engineering {
        engineering.mass_kg = 0.;
    }
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    if let Some(engineering) = &mut bad.engineering {
        engineering.inertia[0][1] = 0.01;
    }
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    if let Some(engineering) = &mut bad.engineering {
        engineering.inertia[0][0] = -1.;
    }
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    if let Some(engineering) = &mut bad.engineering {
        engineering.inertia = [[50., 0., 0.], [0., 10., 0.], [0., 0., 10.]];
    }
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    bad.flight.max_speed = Some(f64::INFINITY);
    assert!(!validate_package(&bad).valid);
    bad.flight.max_speed = None;
    assert!(validate_package(&bad).valid);
    bad.engineering = None;
    assert!(validate_package(&bad).valid);
    let mut bad = package();
    bad.rounding_metres = 0.03;
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    bad.schema = 2;
    assert!(!validate_package(&bad).valid);
    let mut bad = package();
    bad.clearances.push(bad.clearances[0].clone());
    assert!(!validate_package(&bad).valid);
}

#[test]
fn optional_textures_round_trip_only_known_atlas_and_finite_uvs() -> Result<(), serde_json::Error> {
    let mut source = package();
    source.face_textures = Some(vec![
        FaceTexture {
            texture: Some(Texture::Strike),
            uv: [[0., 0.], [1., 0.], [0., 1.]]
        };
        4
    ]);
    source.materials[0].alpha = 127;
    source.materials[0].emissive = true;
    let json = serde_json::to_string(&source)?;
    let decoded: HullPackage = serde_json::from_str(&json)?;
    assert_eq!(source, decoded);
    assert_eq!(Texture::Strike.atlas_index(), 1);
    assert!(validate_package(&decoded).valid);
    assert!(
        serde_json::from_str::<HullPackage>(&json.replace("l337-strike-diffuse", "../local-image"))
            .is_err()
    );
    let mut bad = decoded.clone();
    if let Some(faces) = &mut bad.face_textures {
        faces[0].uv[0][0] = f64::NAN;
    }
    assert!(!validate_package(&bad).valid);
    let mut bad = decoded;
    if let Some(faces) = &mut bad.face_textures {
        faces.pop();
    }
    assert!(!validate_package(&bad).valid);
    Ok(())
}
