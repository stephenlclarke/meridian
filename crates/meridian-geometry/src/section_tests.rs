//! Frozen parent-ray comparison and independent analytic section contracts.
mod before {
    include!("../fixtures/section-reference/before-certificate.rs");
}
use crate::{
    AffineTransform, Budget, Facet, Failure, FailureKind, Limits, SourceId, Vertex, integer,
};

fn mesh(
    points: &[[i64; 3]],
    triangles: &[[usize; 3]],
    owner: u32,
    first: u32,
) -> Result<Vec<Facet>, Failure> {
    let mut b = Budget::new(Limits::default())?;
    triangles
        .iter()
        .enumerate()
        .map(|(i, triangle)| {
            Facet::new(
                SourceId {
                    component: owner,
                    facet: first + i as u32,
                },
                triangle.map(|j| {
                    let point = points[j].map(integer);
                    Vertex {
                        uv: [&point[0] + integer(2) * &point[1], &point[2] - &point[1]],
                        point,
                    }
                }),
                "material0/texture0".into(),
                &mut b,
            )
        })
        .collect()
}
fn cube(bounds: [i64; 6], owner: u32, first: u32, reverse: bool) -> Result<Vec<Facet>, Failure> {
    let [x, y, z, a, c, d] = bounds;
    let points = [
        [x, y, z],
        [a, y, z],
        [a, c, z],
        [x, c, z],
        [x, y, d],
        [a, y, d],
        [a, c, d],
        [x, c, d],
    ];
    let mut triangles = [
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
    if reverse {
        for triangle in &mut triangles {
            triangle.swap(1, 2);
        }
    }
    mesh(&points, &triangles, owner, first)
}
fn prism() -> Result<Vec<Facet>, Failure> {
    let polygon = [[0, 0], [6, 0], [6, 2], [2, 2], [2, 6], [0, 6]];
    let points: Vec<_> = [0, 4]
        .into_iter()
        .flat_map(|z| polygon.map(|[x, y]| [x, y, z]))
        .collect();
    let caps = [[0, 1, 3], [1, 2, 3], [0, 3, 5], [3, 4, 5]];
    let mut triangles = Vec::new();
    for [a, c, d] in caps {
        triangles.push([a, d, c]);
        triangles.push([a + 6, c + 6, d + 6]);
    }
    for i in 0..6 {
        let j = (i + 1) % 6;
        triangles.push([i, j, j + 6]);
        triangles.push([i, j + 6, i + 6]);
    }
    mesh(&points, &triangles, 0, 0)
}
fn cavity() -> Result<Vec<Facet>, Failure> {
    let mut faces = cube([0, 0, 0, 6, 6, 6], 0, 0, false)?;
    faces.extend(cube([2, 2, 2, 4, 4, 4], 0, 12, true)?);
    Ok(faces)
}
fn compare(id: &str, originals: &[Facet], volume: i64) -> Result<(), Failure> {
    let mut old = Budget::new(Limits::default())?;
    let mut new = Budget::new(Limits::default())?;
    let old_start = std::time::Instant::now();
    let expected = before::reconstruct(originals, &mut old)?;
    let mut old_elapsed = old_start.elapsed();
    let new_start = std::time::Instant::now();
    let actual = crate::reconstruct(originals, &mut new)?;
    let mut new_elapsed = new_start.elapsed();
    assert_eq!(expected.exact_volume(), &integer(volume), "{id}");
    assert_eq!(actual.exact_volume(), expected.exact_volume(), "{id}");
    assert_eq!(actual.triangles(), expected.triangles(), "{id}");
    assert_eq!(actual.witnesses().len(), expected.witnesses().len(), "{id}");
    for (a, e) in actual.witnesses().iter().zip(expected.witnesses()) {
        assert_eq!(a.source, e.source, "{id}");
        assert_eq!(a.classified_cells, e.classified_cells, "{id}");
        assert_eq!(a.physical_segments, e.physical_segments, "{id}");
        assert_eq!(a.protected_points, e.protected_points, "{id}");
        assert_eq!(a.output_triangles, e.output_triangles, "{id}");
        assert_eq!(a.twice_area, e.twice_area, "{id}");
        let astars: Vec<_> = a
            .contact_stars
            .iter()
            .map(|s| (&s.point, &s.sectors, s.boundary_path))
            .collect();
        let estars: Vec<_> = e
            .contact_stars
            .iter()
            .map(|s| (&s.point, &s.sectors, s.boundary_path))
            .collect();
        assert_eq!(astars, estars, "{id}");
        let source = originals
            .iter()
            .find(|f| f.id() == a.source)
            .ok_or_else(|| Failure {
                kind: FailureKind::InvalidWitness,
                source: Some(a.source),
                operations: new.operations(),
                detail: "test witness has no source",
            })?;
        let start = std::time::Instant::now();
        a.verify(source, &mut new)?;
        new_elapsed += start.elapsed();
        let start = std::time::Instant::now();
        e.verify(source, &mut old)?;
        old_elapsed += start.elapsed();
    }
    eprintln!(
        "sectionDifferential id={id} oldOperations={} newOperations={} oldPeak={} newPeak={} oldAdmitted={} newAdmitted={} oldRetained={} newRetained={} oldWallNanoseconds={} newWallNanoseconds={}",
        old.operations(),
        new.operations(),
        old.peak_memberships(),
        new.peak_memberships(),
        old.peak_admitted_memberships(),
        new.peak_admitted_memberships(),
        old.retained_memberships(),
        new.retained_memberships(),
        old_elapsed.as_nanos(),
        new_elapsed.as_nanos()
    );
    assert!(
        new_elapsed < old_elapsed.saturating_mul(10),
        "{id}: parity timing regression old={old_elapsed:?} new={new_elapsed:?}"
    );
    Ok(())
}

#[test]
fn one_sided_sections_preserve_complete_nonconvex_cavity_and_oblique_hulls() -> Result<(), Failure>
{
    let mut overlap = prism()?;
    overlap.extend(cube([1, 1, 1, 4, 4, 3], 1, 0, false)?);
    let mut filled = cavity()?;
    filled.extend(cube([1, 1, 1, 5, 5, 5], 1, 0, false)?);
    for (id, faces, volume) in [
        ("L_prism", prism()?, 80),
        ("L_overlap", overlap, 88),
        ("cavity", cavity()?, 208),
        ("cavity_filled", filled, 216),
    ] {
        compare(id, &faces, volume)?;
        let transform = AffineTransform {
            linear: [
                [integer(-1), integer(1), integer(1)],
                [integer(0), integer(1), integer(1)],
                [integer(0), integer(0), integer(1)],
            ],
            translation: [integer(7), integer(-3), integer(2)],
        };
        let mut setup = Budget::new(Limits::default())?;
        let oblique = faces
            .iter()
            .map(|face| transform.apply(face, &mut setup))
            .collect::<Result<Vec<_>, _>>()?;
        compare(&format!("{id}_shear_mirror"), &oblique, volume)?;
    }
    Ok(())
}

#[test]
fn one_sided_sections_preserve_original_contact_and_field_refusals() -> Result<(), Failure> {
    for (id, second) in [
        ("point", [1, 1, 1, 2, 2, 2]),
        ("edge", [1, 1, 0, 2, 2, 1]),
        ("face", [1, 0, 0, 2, 1, 1]),
    ] {
        let mut faces = cube([0, 0, 0, 1, 1, 1], 0, 0, false)?;
        faces.extend(cube(second, 1, 0, false)?);
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        match (
            before::reconstruct(&faces, &mut old),
            crate::reconstruct(&faces, &mut new),
        ) {
            (Ok(_), Ok(_)) => compare(id, &faces, 2)?,
            (Err(a), Err(e)) => assert_eq!(a.kind, e.kind, "{id}"),
            _ => panic!("{id}: candidate/refusal mismatch"),
        }
    }
    // A co-oriented duplicate with a different complete source affine field
    // must keep the semantic conflict, rather than disappear as occupancy.
    let mut faces = cube([0, 0, 0, 1, 1, 1], 0, 0, false)?;
    let duplicate = cube([0, 0, 0, 1, 1, 1], 1, 0, false)?;
    let mut setup = Budget::new(Limits::default())?;
    for face in duplicate {
        let mut vertices = face.vertices().clone();
        for vertex in &mut vertices {
            vertex.uv[0] += integer(1);
        }
        faces.push(Facet::new(
            face.id(),
            vertices,
            face.style().into(),
            &mut setup,
        )?);
    }
    let mut old = Budget::new(Limits::default())?;
    let mut new = Budget::new(Limits::default())?;
    assert_eq!(
        before::reconstruct(&faces, &mut old).err().map(|e| e.kind),
        Some(FailureKind::ConflictingFields)
    );
    assert_eq!(
        crate::reconstruct(&faces, &mut new).err().map(|e| e.kind),
        Some(FailureKind::ConflictingFields)
    );
    Ok(())
}
