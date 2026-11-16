use meridian_geometry::{
    AffineTransform, Arrangement, Budget, Facet, Failure, FailureKind, Limits, Point2, Point3,
    Rational, SourceId, Vertex, fraction, integer, reconstruct,
};
use num_bigint::BigInt;

fn budget() -> Result<Budget, Failure> {
    Budget::new(Limits::default())
}
fn p2(x: i64, y: i64) -> Point2 {
    [integer(x), integer(y)]
}
fn p3(x: i64, y: i64, z: i64) -> Point3 {
    [integer(x), integer(y), integer(z)]
}
fn cube(bounds: [i64; 6], owner: u32) -> Result<Vec<Facet>, Failure> {
    let [x, y, z, a, c, d] = bounds;
    let points = [
        p3(x, y, z),
        p3(a, y, z),
        p3(a, c, z),
        p3(x, c, z),
        p3(x, y, d),
        p3(a, y, d),
        p3(a, c, d),
        p3(x, c, d),
    ];
    // Independent closed box fixture from the frozen analytic source definition.
    let faces = [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    let mut b = budget()?;
    faces
        .iter()
        .enumerate()
        .map(|(i, face)| {
            let vertices = face.map(|index| {
                let p = &points[index];
                Vertex {
                    point: p.clone(),
                    uv: [&p[0] + integer(2) * &p[1], &p[2] - &p[1]],
                }
            });
            Facet::new(
                SourceId {
                    component: owner,
                    facet: i as u32,
                },
                vertices,
                "material0/texture0".into(),
                &mut b,
            )
        })
        .collect()
}

#[test]
fn analytic_closed_originals_preserve_volume_source_fields_and_contact_witnesses()
-> Result<(), Failure> {
    let cases = [
        (vec![[0, 0, 0, 1, 1, 1]], 1),
        (vec![[0, 0, 0, 2, 2, 2], [1, 1, 1, 3, 3, 3]], 15),
        (vec![[0, 0, 0, 3, 3, 3], [1, 1, 1, 2, 2, 2]], 27),
        (vec![[0, 0, 0, 1, 1, 1], [1, 0, 0, 2, 1, 1]], 2),
        (vec![[0, 0, 0, 1, 1, 1], [2, 0, 0, 3, 1, 1]], 2),
    ];
    for (boxes, volume) in cases {
        let mut faces = Vec::new();
        for (i, bounds) in boxes.into_iter().enumerate() {
            faces.extend(cube(bounds, i as u32)?);
        }
        let mut b = budget()?;
        let hull = reconstruct(&faces, &mut b)?;
        assert_eq!(hull.exact_volume(), &integer(volume));
        assert!(hull.triangles().len() <= 512);
        for triangle in hull.triangles() {
            let original = faces.iter().find(|v| v.id() == triangle.id());
            assert!(original.is_some());
            if let Some(original) = original {
                assert_eq!(original.style(), triangle.style());
                for vertex in triangle.vertices() {
                    assert_eq!(original.uv_at(&vertex.point, &mut b)?, vertex.uv);
                }
            }
        }
        for witness in hull.witnesses() {
            if let Some(original) = faces.iter().find(|v| v.id() == witness.source) {
                witness.verify(original, &mut b)?;
            }
        }
    }
    Ok(())
}

#[test]
fn transformed_component_ids_orientation_and_affine_uv_are_exact() -> Result<(), Failure> {
    let faces = cube([0, 0, 0, 1, 1, 1], 19)?;
    let transform = AffineTransform {
        linear: [
            [integer(-1), integer(0), integer(1)],
            [integer(0), integer(1), integer(0)],
            [integer(0), integer(0), integer(1)],
        ],
        translation: p3(7, 3, 1),
    };
    let mut b = budget()?;
    let transformed: Vec<_> = faces
        .iter()
        .map(|face| transform.apply(face, &mut b))
        .collect::<Result<_, _>>()?;
    let hull = reconstruct(&transformed, &mut b)?;
    assert_eq!(hull.exact_volume(), &integer(1));
    assert!(hull.triangles().iter().all(|f| f.id().component == 19));
    for (before, after) in faces.iter().zip(&transformed) {
        assert_eq!(before.id(), after.id());
        assert_eq!(before.vertices()[0].uv, after.vertices()[0].uv);
        assert_eq!(before.vertices()[1].uv, after.vertices()[2].uv);
    }
    Ok(())
}

#[test]
fn finite_point_and_vertical_contacts_survive_classification_and_noding() -> Result<(), Failure> {
    let domain = vec![p2(0, 0), p2(8, 0), p2(0, 8)];
    let barriers = vec![[p2(2, 2), p2(2, 2)], [p2(3, 1), p2(3, 4)]];
    let mut b = budget()?;
    let arrangement = Arrangement::build(&domain, &barriers, &mut b, |_, _| Ok(true))?;
    assert!(arrangement.protected.contains(&p2(2, 2)));
    assert!(arrangement.protected.contains(&p2(3, 1)));
    assert!(arrangement.cells.iter().flatten().any(|p| p == &p2(2, 2)));
    assert!(arrangement.cells.iter().flatten().any(|p| p == &p2(3, 4)));
    assert!(arrangement.work.point_contact_chords > 0);
    assert!(arrangement.work.vertical_contact_chords > 0);
    // Independent shoelace area, with a known analytic triangle total of 32.
    let mut twice = integer(0);
    for cell in arrangement.cells {
        for i in 0..cell.len() {
            twice += &cell[i][0] * &cell[(i + 1) % cell.len()][1]
                - &cell[i][1] * &cell[(i + 1) % cell.len()][0];
        }
    }
    assert_eq!(twice, integer(64));
    Ok(())
}

#[test]
fn incomplete_or_mutated_witness_is_rejected() -> Result<(), Failure> {
    let faces = cube([0, 0, 0, 1, 1, 1], 0)?;
    let mut b = budget()?;
    let hull = reconstruct(&faces, &mut b)?;
    let mut witness = hull.witnesses()[0].clone();
    let source = &faces
        .iter()
        .find(|f| f.id() == witness.source)
        .ok_or(Failure {
            kind: FailureKind::InvalidWitness,
            source: None,
            operations: 0,
            detail: "test source lookup",
        })?;
    witness.output_triangles.clear();
    let failure = witness.verify(source, &mut b).err();
    assert!(failure.is_some_and(|f| f.kind == FailureKind::InvalidWitness));
    let mut witness = hull.witnesses()[0].clone();
    witness.contact_stars.clear();
    assert!(witness.verify(source, &mut b).is_err());
    Ok(())
}

#[test]
fn invalid_originals_and_singular_links_refuse_without_candidate() -> Result<(), Failure> {
    let mut b = budget()?;
    assert!(reconstruct(&[], &mut b).is_err());
    let faces = cube([0, 0, 0, 1, 1, 1], 0)?;
    let mut duplicate = faces.clone();
    duplicate.push(faces[0].clone());
    assert_eq!(
        reconstruct(&duplicate, &mut budget()?)
            .err()
            .map(|f| f.kind),
        Some(FailureKind::InvalidInput)
    );
    assert_eq!(
        reconstruct(&faces[1..], &mut budget()?)
            .err()
            .map(|f| f.kind),
        Some(FailureKind::NonManifold)
    );
    let mut touching = faces.clone();
    touching.extend(cube([1, 1, 1, 2, 2, 2], 1)?);
    assert!(reconstruct(&touching, &mut budget()?).is_err());
    let mut conflict = faces.clone();
    for face in cube([0, 0, 0, 1, 1, 1], 1)? {
        let mut vertices = face.vertices().clone();
        for vertex in &mut vertices {
            vertex.uv[0] += integer(1);
        }
        conflict.push(Facet::new(
            face.id(),
            vertices,
            face.style().to_owned(),
            &mut b,
        )?);
    }
    assert_eq!(
        reconstruct(&conflict, &mut budget()?).err().map(|f| f.kind),
        Some(FailureKind::ConflictingFields)
    );
    Ok(())
}

#[test]
fn original_budget_boundaries_and_refusals_remain_explicit() -> Result<(), Failure> {
    let mut exact = Budget::new(Limits {
        operations: 4,
        storage_memberships: 5,
        ..Limits::default()
    })?;
    exact.step(4)?;
    exact.guard(5)?;
    assert_eq!(
        exact.step(1).err().map(|f| f.kind),
        Some(FailureKind::Operations)
    );
    assert_eq!(
        exact.guard(6).err().map(|f| f.kind),
        Some(FailureKind::Storage)
    );
    assert!(
        Budget::new(Limits {
            triangles: 513,
            ..Limits::default()
        })
        .is_err()
    );
    let mut b = budget()?;
    assert!(fraction("1", "0", &mut b).is_err());
    let at_bound = BigInt::from(1) << 4095usize;
    assert!(fraction(&at_bound.to_string(), "1", &mut b).is_ok());
    let beyond = BigInt::from(1) << 4096usize;
    assert_eq!(
        fraction(&beyond.to_string(), "1", &mut b)
            .err()
            .map(|f| f.kind),
        Some(FailureKind::Rational)
    );
    let original = cube([0, 0, 0, 1, 1, 1], 0)?;
    let mut limited = Budget::new(Limits {
        triangles: 1,
        ..Limits::default()
    })?;
    assert_eq!(
        reconstruct(&original, &mut limited).err().map(|f| f.kind),
        Some(FailureKind::TriangleLimit)
    );
    let mut full = budget()?;
    reconstruct(&original, &mut full)?;
    let count = full.operations();
    let mut exact = Budget::new(Limits {
        operations: count,
        ..Limits::default()
    })?;
    reconstruct(&original, &mut exact)?;
    let mut short = Budget::new(Limits {
        operations: count - 1,
        ..Limits::default()
    })?;
    assert_eq!(
        reconstruct(&original, &mut short).err().map(|f| f.kind),
        Some(FailureKind::Operations)
    );
    Ok(())
}

#[test]
fn dense_original_derived_fixture_retains_a_bounded_failure_expectation() -> Result<(), Failure> {
    // Frozen pre-port exposed 4x4 grid. Its original Python control has 396
    // triangles and 422624 charged operations. The conservative retained
    // arrangement must return an explicit guard refusal, preserving this open limit.
    let mut faces = cube([0, 0, 0, 13, 13, 3], 0)?;
    for i in 0..4 {
        for j in 0..4 {
            faces.extend(cube(
                [3 * i + 1, 3 * j + 1, 2, 3 * i + 2, 3 * j + 2, 4],
                1 + (i * 4 + j) as u32,
            )?);
        }
    }
    let mut b = budget()?;
    let result = reconstruct(&faces, &mut b);
    assert!(result.is_err());
    if let Err(failure) = result {
        assert!(matches!(
            failure.kind,
            FailureKind::Operations | FailureKind::Storage | FailureKind::TriangleLimit
        ));
    }
    Ok(())
}

#[test]
fn out_of_facet_and_nonfinite_source_substitutions_refuse() -> Result<(), Failure> {
    let faces = cube([0, 0, 0, 1, 1, 1], 0)?;
    let mut b = budget()?;
    assert!(faces[0].uv_at(&p3(2, 2, 0), &mut b).is_err());
    assert!(fraction("NaN", "1", &mut b).is_err());
    assert!(fraction("0.5", "1", &mut b).is_err());
    let singular = AffineTransform {
        linear: std::array::from_fn(|_| p3(0, 0, 0)),
        translation: p3(0, 0, 0),
    };
    assert!(singular.apply(&faces[0], &mut b).is_err());
    let zero = Vertex {
        point: p3(0, 0, 0),
        uv: p2(0, 0),
    };
    assert!(
        Facet::new(
            SourceId {
                component: 0,
                facet: 0
            },
            [zero.clone(), zero.clone(), zero],
            String::new(),
            &mut b
        )
        .is_err()
    );
    let _: Rational = integer(1);
    Ok(())
}
