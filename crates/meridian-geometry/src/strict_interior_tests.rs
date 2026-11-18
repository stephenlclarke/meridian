use super::*;
use crate::certificate::{
    audit_original_owner_ranges, audit_original_sheets, component_bounds, reconstruct,
};
use crate::{Limits, SourceId, Vertex, integer};
use std::collections::BTreeSet;

use crate::certificate::*;
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/strict-interior-reference/compiled-reconstruct-control.rs"
));

fn mesh(
    points: &[[i64; 3]],
    triangles: &[[usize; 3]],
    owner: u32,
    subdivide: usize,
) -> Result<Vec<Facet>, Failure> {
    let mut b = Budget::new(Limits::default())?;
    let mut faces: Vec<[Point3; 3]> = triangles
        .iter()
        .map(|t| t.map(|i| points[i].map(integer)))
        .collect();
    for _ in 0..subdivide {
        let mut next = Vec::new();
        for [a, c, d] in faces {
            let mid =
                |a: &Point3, c: &Point3| std::array::from_fn(|k| (&a[k] + &c[k]) / integer(2));
            let ac = mid(&a, &c);
            let ad = mid(&a, &d);
            let cd = mid(&c, &d);
            next.extend([
                [a, ac.clone(), ad.clone()],
                [ac.clone(), c, cd.clone()],
                [ad.clone(), cd.clone(), d],
                [ac, cd, ad],
            ]);
        }
        faces = next;
    }
    faces
        .into_iter()
        .enumerate()
        .map(|(i, vertices)| {
            Facet::new(
                SourceId {
                    component: owner,
                    facet: i as u32,
                },
                vertices.map(|point| Vertex {
                    point,
                    uv: [integer(0), integer(0)],
                }),
                "test".into(),
                &mut b,
            )
        })
        .collect()
}
fn cube(lo: i64, hi: i64, owner: u32, subdivide: usize) -> Result<Vec<Facet>, Failure> {
    mesh(
        &[
            [lo, lo, lo],
            [hi, lo, lo],
            [hi, hi, lo],
            [lo, hi, lo],
            [lo, lo, hi],
            [hi, lo, hi],
            [hi, hi, hi],
            [lo, hi, hi],
        ],
        &[
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
        ],
        owner,
        subdivide,
    )
}
fn audit(faces: &[Facet]) -> Result<(), Failure> {
    let mut b = Budget::new(Limits::default())?;
    let owners = faces
        .iter()
        .map(|f| f.id().component)
        .collect::<BTreeSet<_>>();
    audit_original_owner_ranges(faces, &owners, &mut b)?;
    audit_original_sheets(faces, &mut b)
}
fn triangle(points: [[i64; 3]; 3]) -> Result<Facet, Failure> {
    let mut b = Budget::new(Limits::default())?;
    Facet::new(
        SourceId {
            component: 99,
            facet: 0,
        },
        points.map(|p| Vertex {
            point: p.map(integer),
            uv: [integer(0), integer(0)],
        }),
        "test".into(),
        &mut b,
    )
}

#[test]
fn strict_whole_facet_occlusion_matches_frozen_complete_output_and_keeps_equality_contacts()
-> Result<(), Failure> {
    for touching in [false, true] {
        let mut originals = cube(0, 8, 0, 1)?;
        originals.extend(cube(if touching { 0 } else { 2 }, 6, 1, 0)?);
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let old_start = std::time::Instant::now();
        let expected = reference_reconstruct_records(&originals, &mut old)?;
        let old_elapsed = old_start.elapsed();
        let new_start = std::time::Instant::now();
        let actual = reconstruct(&originals, &mut new)?;
        let new_elapsed = new_start.elapsed();
        assert_eq!(actual.triangles(), expected.triangles());
        assert_eq!(actual.witnesses(), expected.witnesses());
        assert_eq!(actual.exact_volume(), expected.exact_volume());
        assert_eq!(new.retained_memberships(), old.retained_memberships());
        if !touching {
            assert!(new.operations() < old.operations());
        }
        eprintln!(
            "STRICT_INTERIOR_COMPLETE touching={touching} oldOperations={} newOperations={} oldPeak={} newPeak={} oldWallNanoseconds={} newWallNanoseconds={}",
            old.operations(),
            new.operations(),
            old.peak_memberships(),
            new.peak_memberships(),
            old_elapsed.as_nanos(),
            new_elapsed.as_nanos()
        );
        assert!(new_elapsed < old_elapsed.saturating_mul(10));
    }
    Ok(())
}

#[test]
fn convex_certificate_checks_all_vertices_and_oblique_planes_and_rejects_nonconvex_owners()
-> Result<(), Failure> {
    let tetra = mesh(
        &[[0, 0, 0], [8, 0, 0], [0, 8, 0], [0, 0, 8]],
        &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
        0,
        2,
    )?;
    audit(&tetra)?;
    let mut b = Budget::new(Limits::default())?;
    let index = FacetBoundsIndex::build(&tetra, &mut b)?;
    let owners = component_bounds(&tetra, &index.facets, &mut b)?;
    let certificates = InteriorCertificates::build(&tetra, &owners, &mut b)?.ok_or_else(|| {
        b.fail(
            crate::FailureKind::InvalidWitness,
            "missing convex certificate",
        )
    })?;
    for (points, expected) in [
        ([[1, 1, 1], [2, 1, 1], [1, 2, 1]], true),
        ([[0, 1, 1], [0, 2, 1], [0, 1, 2]], false),
        ([[5, 5, 5], [6, 5, 5], [5, 6, 5]], false),
    ] {
        let face = triangle(points)?;
        let bounds = Bounds::for_facet(&face, &mut b)?;
        assert_eq!(
            certificates.strictly_covers(&face, &bounds, &mut b)?,
            expected
        );
    }
    let members = certificates.memberships();
    drop(certificates);
    b.release(members);
    assert_eq!(b.retained_memberships(), 0);
    let polygon = [[0, 0], [6, 0], [6, 2], [2, 2], [2, 6], [0, 6]];
    let points: Vec<_> = [0, 4]
        .into_iter()
        .flat_map(|z| polygon.map(|[x, y]| [x, y, z]))
        .collect();
    let mut triangles = Vec::new();
    for [a, c, d] in [[0, 1, 3], [1, 2, 3], [0, 3, 5], [3, 4, 5]] {
        triangles.extend([[a, d, c], [a + 6, c + 6, d + 6]]);
    }
    for i in 0..6 {
        let j = (i + 1) % 6;
        triangles.extend([[i, j, j + 6], [i, j + 6, i + 6]]);
    }
    let nonconvex = mesh(&points, &triangles, 0, 1)?;
    audit(&nonconvex)?;
    let index = FacetBoundsIndex::build(&nonconvex, &mut b)?;
    let owners = component_bounds(&nonconvex, &index.facets, &mut b)?;
    assert!(InteriorCertificates::build(&nonconvex, &owners, &mut b)?.is_none());
    assert_eq!(b.retained_memberships(), 0);
    let mut cavity = cube(0, 8, 0, 1)?;
    for (i, face) in cube(2, 6, 0, 0)?.iter().enumerate() {
        let mut vertices: [Vertex; 3] = std::array::from_fn(|k| face.vertices()[k].clone());
        vertices.swap(1, 2);
        cavity.push(Facet::new(
            SourceId {
                component: 0,
                facet: 48 + i as u32,
            },
            vertices,
            "test".into(),
            &mut b,
        )?);
    }
    audit(&cavity)?;
    let index = FacetBoundsIndex::build(&cavity, &mut b)?;
    let owners = component_bounds(&cavity, &index.facets, &mut b)?;
    assert!(InteriorCertificates::build(&cavity, &owners, &mut b)?.is_none());
    assert_eq!(b.retained_memberships(), 0);
    Ok(())
}
fn admitted(
    faces: &[Facet],
    owners: &BTreeMap<u32, ComponentBounds>,
    face: &Facet,
    bounds: &Bounds,
    b: &mut Budget,
) -> Result<(), Failure> {
    let Some(certificates) = InteriorCertificates::build(faces, owners, b)? else {
        return Err(b.fail(
            crate::FailureKind::InvalidWitness,
            "missing test certificate",
        ));
    };
    let result = certificates.strictly_covers(face, bounds, b);
    let members = certificates.memberships();
    drop(certificates);
    b.release(members);
    assert!(result?);
    Ok(())
}
#[test]
fn convex_proof_tables_and_strict_query_scratch_admit_and_roll_back_at_exact_boundaries()
-> Result<(), Failure> {
    let faces = cube(0, 8, 0, 1)?;
    audit(&faces)?;
    let mut setup = Budget::new(Limits::default())?;
    let index = FacetBoundsIndex::build(&faces, &mut setup)?;
    let owners = component_bounds(&faces, &index.facets, &mut setup)?;
    let face = triangle([[2, 2, 2], [3, 2, 2], [2, 3, 2]])?;
    let bounds = Bounds::for_facet(&face, &mut setup)?;
    let mut probe = Budget::new(Limits::default())?;
    probe.retain(20_000)?;
    admitted(&faces, &owners, &face, &bounds, &mut probe)?;
    let cap = probe.peak_memberships();
    assert_eq!(probe.retained_memberships(), 20_000);
    let mut exact = Budget::new(Limits {
        storage_memberships: cap,
        ..Limits::default()
    })?;
    exact.retain(20_000)?;
    admitted(&faces, &owners, &face, &bounds, &mut exact)?;
    assert_eq!(exact.retained_memberships(), 20_000);
    assert_eq!(exact.peak_admitted_memberships(), cap);
    let mut short = Budget::new(Limits {
        storage_memberships: cap - 1,
        ..Limits::default()
    })?;
    short.retain(20_000)?;
    assert_eq!(
        admitted(&faces, &owners, &face, &bounds, &mut short)
            .err()
            .map(|f| f.kind),
        Some(crate::FailureKind::Storage)
    );
    assert_eq!(short.retained_memberships(), 20_000);
    let mut work = Budget::new(Limits {
        operations: 3,
        ..Limits::default()
    })?;
    work.retain(20_000)?;
    assert_eq!(
        admitted(&faces, &owners, &face, &bounds, &mut work)
            .err()
            .map(|f| f.kind),
        Some(crate::FailureKind::Operations)
    );
    assert_eq!(work.retained_memberships(), 20_000);
    let certificates =
        InteriorCertificates::build(&faces, &owners, &mut setup)?.ok_or_else(|| {
            setup.fail(
                crate::FailureKind::InvalidWitness,
                "missing query certificate",
            )
        })?;
    let members = certificates.memberships();
    let query_cap = 20_000 + members + 32;
    let mut query = Budget::new(Limits {
        storage_memberships: query_cap,
        ..Limits::default()
    })?;
    query.retain(20_000 + members)?;
    assert!(certificates.strictly_covers(&face, &bounds, &mut query)?);
    assert_eq!(query.peak_memberships(), query_cap);
    assert_eq!(query.retained_memberships(), 20_000 + members);
    let mut query_short = Budget::new(Limits {
        storage_memberships: query_cap - 1,
        ..Limits::default()
    })?;
    query_short.retain(20_000 + members)?;
    assert_eq!(
        certificates
            .strictly_covers(&face, &bounds, &mut query_short)
            .err()
            .map(|f| f.kind),
        Some(crate::FailureKind::Storage)
    );
    assert_eq!(query_short.retained_memberships(), 20_000 + members);
    drop(certificates);
    setup.release(members);
    assert_eq!(setup.retained_memberships(), 0);
    Ok(())
}
