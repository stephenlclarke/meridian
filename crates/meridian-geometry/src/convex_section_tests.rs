use super::convex_sections::Rows;
use super::strict_interior::InteriorCertificates;
use super::*;
use crate::Limits;
use std::time::Instant;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/convex-section-reference/compiled-publisher-control.rs"
));

fn checked_failure<T>(result: Result<T, Failure>) -> Failure {
    match result {
        Err(failure) => failure,
        Ok(_) => panic!("unexpected success in refusal assertion"),
    }
}

fn failure_context<T>(
    fixture: &str,
    lane: &str,
    run: impl FnOnce() -> Result<T, Failure>,
) -> Result<T, Failure> {
    match run() {
        Err(failure) => {
            eprintln!(
                "CONVEX_SECTION_FAILURE fixture={fixture} lane={lane} kind={:?} source={:?} operations={} detail={}",
                failure.kind, failure.source, failure.operations, failure.detail
            );
            Err(failure)
        }
        Ok(value) => Ok(value),
    }
}

fn face_neighbour() -> Result<Vec<Facet>, Failure> {
    let mut b = Budget::new(Limits::default())?;
    cube(0, 8, 1, 0)?
        .into_iter()
        .map(|face| {
            let mut vertices = face.vertices().clone();
            for vertex in &mut vertices {
                // Translate x only, retaining the same world-affine UV fixture.
                vertex.point[0] += integer(8);
                vertex.uv[0] += integer(8);
            }
            Facet::new(face.id(), vertices, face.style().to_owned(), &mut b)
        })
        .collect()
}

fn corner_refusal<T>(lane: &str, result: Result<T, Failure>) -> Failure {
    match result {
        Err(failure) => failure,
        Ok(_) => panic!("fixture=corner-touch lane={lane} expected certified-contact refusal"),
    }
}

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
                    uv: [point[0].clone(), point[1].clone()],
                    point,
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

fn release_rows(rows: Rows, b: &mut Budget) {
    let members = rows.memberships();
    drop(rows);
    b.release(members);
}

#[test]
fn convex_section_rows_preserve_complete_publisher_fields_contacts_and_volume()
-> Result<(), Failure> {
    failure_context(
        "convex_section_rows_preserve_complete_publisher_fields_contacts_and_volume",
        "case-return",
        || -> Result<(), Failure> {
            for fixture in 0..3 {
                let label = match fixture {
                    0 => "overlapping-boxes",
                    1 => "oblique-tetrahedron",
                    _ => "full-face-contact",
                };
                failure_context(label, "fixture-return", || -> Result<(), Failure> {
                    let mut originals = if fixture == 1 {
                        mesh(
                            &[[0, 0, 0], [8, 0, 0], [0, 8, 0], [0, 0, 8]],
                            &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
                            0,
                            2,
                        )?
                    } else {
                        cube(0, 8, 0, 0)?
                    };
                    originals.extend(match fixture {
                        0 => cube(2, 10, 1, 0)?,
                        1 => cube(1, 4, 1, 0)?,
                        _ => face_neighbour()?,
                    });
                    let mut old = Budget::new(Limits::default())?;
                    let mut new = Budget::new(Limits::default())?;
                    let start = Instant::now();
                    let expected = failure_context(label, "reference-publisher", || {
                        reference_convex_sections_publisher(&originals, &mut old)
                    })?;
                    let old_time = start.elapsed();
                    let start = Instant::now();
                    let actual = failure_context(label, "candidate-publisher", || {
                        reconstruct(&originals, &mut new)
                    })?;
                    let new_time = start.elapsed();
                    assert_eq!(actual.triangles(), expected.triangles());
                    assert_eq!(actual.witnesses(), expected.witnesses());
                    assert_eq!(actual.exact_volume(), expected.exact_volume());
                    assert_eq!(new.retained_memberships(), old.retained_memberships());
                    if fixture == 1 {
                        assert!(new.operations() < old.operations());
                    }
                    eprintln!(
                        "CONVEX_SECTION_COMPLETE fixture={fixture} oldOperations={} newOperations={} oldPeak={} newPeak={} oldWallNanoseconds={} newWallNanoseconds={}",
                        old.operations(),
                        new.operations(),
                        old.peak_memberships(),
                        new.peak_memberships(),
                        old_time.as_nanos(),
                        new_time.as_nanos()
                    );
                    assert!(new_time < old_time.saturating_mul(10));
                    Ok(())
                })?;
            }
            Ok(())
        },
    )
}

#[test]
fn convex_rows_classify_exact_inside_outside_parallel_and_boundary_without_cached_answers()
-> Result<(), Failure> {
    failure_context(
        "convex_rows_classify_exact_inside_outside_parallel_and_boundary_without_cached_answers",
        "case-return",
        || -> Result<(), Failure> {
            let originals = cube(0, 8, 0, 0)?;
            audit(&originals)?;
            let mut b = Budget::new(Limits::default())?;
            let index = FacetBoundsIndex::build(&originals, &mut b)?;
            let owners = component_bounds(&originals, &index.facets, &mut b)?;
            let certificates = InteriorCertificates::build_sections(&originals, &owners, &mut b)?
                .ok_or_else(|| {
                b.fail(FailureKind::InvalidWitness, "missing small-owner proof")
            })?;
            for (z, reverse, expected) in [
                (4, false, Some(true)),
                (0, true, Some(false)),
                (0, false, Some(true)),
                (8, true, Some(true)),
                (8, false, Some(false)),
            ] {
                let mut vertices = [[1, 1, z], [7, 1, z], [1, 7, z]].map(|point| crate::Vertex {
                    point: point.map(integer),
                    uv: [integer(0), integer(0)],
                });
                if reverse {
                    vertices.swap(1, 2);
                }
                // Positive z normal enters the bottom plane and exits the top plane.
                let face = Facet::new(
                    SourceId {
                        component: 99,
                        facet: 0,
                    },
                    vertices,
                    "test".into(),
                    &mut b,
                )?;
                let finite = Bounds::for_facet(&face, &mut b)?;
                let rows = Rows::build(&face, &finite, Some(&certificates), &mut b)?;
                assert_eq!(rows.occupied(&[integer(2), integer(2)], &mut b)?, expected);
                assert_eq!(
                    rows.occupied(&[integer(9), integer(2)], &mut b)?,
                    Some(false)
                );
                if z == 4 {
                    assert_eq!(rows.occupied(&[integer(0), integer(2)], &mut b)?, None);
                }
                assert_eq!(rows.occupied(&[integer(2), integer(2)], &mut b)?, expected);
                release_rows(rows, &mut b);
            }
            let members = certificates.memberships();
            drop(certificates);
            b.release(members);
            assert_eq!(b.retained_memberships(), 0);
            Ok(())
        },
    )
}

#[test]
fn convex_section_proofs_decline_cavities_and_keep_uncertified_original_sections()
-> Result<(), Failure> {
    failure_context(
        "convex_section_proofs_decline_cavities_and_keep_uncertified_original_sections",
        "case-return",
        || -> Result<(), Failure> {
            let mut originals = cube(0, 8, 0, 0)?;
            let mut build = Budget::new(Limits::default())?;
            for (i, face) in cube(2, 6, 0, 0)?.iter().enumerate() {
                let mut vertices = face.vertices().clone();
                vertices.swap(1, 2);
                originals.push(Facet::new(
                    SourceId {
                        component: 0,
                        facet: 12 + i as u32,
                    },
                    vertices,
                    "test".into(),
                    &mut build,
                )?);
            }
            audit(&originals)?;
            let mut b = Budget::new(Limits::default())?;
            let index = FacetBoundsIndex::build(&originals, &mut b)?;
            let owners = component_bounds(&originals, &index.facets, &mut b)?;
            assert!(InteriorCertificates::build_sections(&originals, &owners, &mut b)?.is_none());
            assert_eq!(b.retained_memberships(), 0);
            let rows = Rows::build(&originals[0], &index.facets[0], None, &mut b)?;
            let mut old = Budget::new(Limits::default())?;
            let old_start = Instant::now();
            let (a, ac, af) = sections::Frame::collect(
                &originals[0],
                &index.facets[0],
                &originals,
                &owners,
                &index,
                &mut old,
            )?;
            let old_elapsed = old_start.elapsed();
            let new_start = Instant::now();
            let (c, cc, cf) = sections::Frame::collect_rows(
                &originals[0],
                &index.facets[0],
                &originals,
                &owners,
                &index,
                &rows,
                &mut b,
            )?;
            let new_elapsed = new_start.elapsed();
            eprintln!(
                "CONVEX_SECTION_FALLBACK fixture=cavity-collector oldWallNanoseconds={} newWallNanoseconds={}",
                old_elapsed.as_nanos(),
                new_elapsed.as_nanos()
            );
            assert!(new_elapsed < old_elapsed.saturating_mul(10));
            assert_eq!(a, c);
            assert_eq!(
                ac.iter().map(|f| f.id()).collect::<Vec<_>>(),
                cc.iter().map(|f| f.id()).collect::<Vec<_>>()
            );
            for sample in [[integer(1), integer(1)], [integer(4), integer(4)]] {
                assert_eq!(
                    af.occupied(&sample, &mut old)?,
                    cf.occupied(&sample, &mut b)?
                );
            }
            let am = 8 * a.len() + ac.len() + af.memberships();
            let cm = 8 * c.len() + cc.len() + cf.memberships();
            drop((a, ac, af));
            old.release(am);
            drop((c, cc, cf));
            b.release(cm);
            release_rows(rows, &mut b);
            assert_eq!(b.retained_memberships(), 0);
            assert_eq!(old.retained_memberships(), 0);
            let mut old = Budget::new(Limits::default())?;
            let mut new = Budget::new(Limits::default())?;
            let start = Instant::now();
            let expected = failure_context("cavity", "reference-publisher", || {
                reference_convex_sections_publisher(&originals, &mut old)
            })?;
            let old_elapsed = start.elapsed();
            let start = Instant::now();
            let actual = failure_context("cavity", "candidate-publisher", || {
                reconstruct(&originals, &mut new)
            })?;
            let new_elapsed = start.elapsed();
            assert_eq!(actual.triangles(), expected.triangles());
            assert_eq!(actual.witnesses(), expected.witnesses());
            assert_eq!(actual.exact_volume(), expected.exact_volume());
            assert_eq!(new.retained_memberships(), old.retained_memberships());
            eprintln!(
                "CONVEX_SECTION_COMPLETE fixture=cavity oldOperations={} newOperations={} oldWallNanoseconds={} newWallNanoseconds={}",
                old.operations(),
                new.operations(),
                old_elapsed.as_nanos(),
                new_elapsed.as_nanos()
            );
            assert!(new_elapsed < old_elapsed.saturating_mul(10));
            Ok(())
        },
    )
}

#[test]
fn convex_rows_keep_exact_storage_work_guards_and_collection_error_rollback() -> Result<(), Failure>
{
    failure_context(
        "convex_rows_keep_exact_storage_work_guards_and_collection_error_rollback",
        "case-return",
        || -> Result<(), Failure> {
            let originals = cube(0, 8, 0, 0)?;
            audit(&originals)?;
            let mut setup = Budget::new(Limits::default())?;
            let index = FacetBoundsIndex::build(&originals, &mut setup)?;
            let owners = component_bounds(&originals, &index.facets, &mut setup)?;
            let cert = InteriorCertificates::build_sections(&originals, &owners, &mut setup)?
                .ok_or_else(|| setup.fail(FailureKind::InvalidWitness, "missing proof"))?;
            let mut measured = Budget::new(Limits::default())?;
            measured.retain(2000)?;
            let rows = Rows::build(&originals[0], &index.facets[0], Some(&cert), &mut measured)?;
            let cap = measured.peak_memberships();
            release_rows(rows, &mut measured);
            let mut exact = Budget::new(Limits {
                storage_memberships: cap,
                ..Limits::default()
            })?;
            exact.retain(2000)?;
            let rows = Rows::build(&originals[0], &index.facets[0], Some(&cert), &mut exact)?;
            release_rows(rows, &mut exact);
            assert_eq!(exact.retained_memberships(), 2000);
            let mut tight = Budget::new(Limits {
                storage_memberships: cap - 1,
                ..Limits::default()
            })?;
            tight.retain(2000)?;
            assert_eq!(
                checked_failure(Rows::build(
                    &originals[0],
                    &index.facets[0],
                    Some(&cert),
                    &mut tight
                ))
                .kind,
                FailureKind::Storage
            );
            assert_eq!(tight.retained_memberships(), 2000);
            let mut short = Budget::new(Limits {
                operations: 1,
                ..Limits::default()
            })?;
            short.retain(2000)?;
            assert_eq!(
                checked_failure(InteriorCertificates::build_sections(
                    &originals, &owners, &mut short
                ))
                .kind,
                FailureKind::Operations
            );
            assert_eq!(short.retained_memberships(), 2000);
            let rows = Rows::build(&originals[0], &index.facets[0], Some(&cert), &mut measured)?;
            let mut visit = Budget::new(Limits {
                operations: 1,
                ..Limits::default()
            })?;
            visit.retain(2000 + rows.memberships())?;
            assert_eq!(
                checked_failure(sections::Frame::collect_rows(
                    &originals[0],
                    &index.facets[0],
                    &originals,
                    &owners,
                    &index,
                    &rows,
                    &mut visit
                ))
                .kind,
                FailureKind::Operations
            );
            assert_eq!(visit.retained_memberships(), 2000 + rows.memberships());
            release_rows(rows, &mut measured);
            assert_eq!(measured.retained_memberships(), 2000);
            let members = cert.memberships();
            drop(cert);
            setup.release(members);
            assert_eq!(setup.retained_memberships(), 0);
            Ok(())
        },
    )
}

#[test]
fn corner_touch_retains_independent_reference_and_candidate_contact_star_refusals()
-> Result<(), Failure> {
    failure_context("corner-touch", "case-return", || -> Result<(), Failure> {
        // Keep the exact original positive fixture as an explicit negative case.
        let mut originals = cube(0, 8, 0, 0)?;
        originals.extend(cube(8, 16, 1, 0)?);
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let start = Instant::now();
        let reference = reference_convex_sections_publisher(&originals, &mut old);
        let old_elapsed = start.elapsed();
        let start = Instant::now();
        let candidate = reconstruct(&originals, &mut new);
        let new_elapsed = start.elapsed();
        for (lane, failure) in [
            (
                "reference-publisher",
                corner_refusal("reference-publisher", reference),
            ),
            (
                "candidate-publisher",
                corner_refusal("candidate-publisher", candidate),
            ),
        ] {
            assert_eq!(
                failure.kind,
                FailureKind::UncertifiedContact,
                "fixture=corner-touch lane={lane}"
            );
            assert_eq!(
                failure.source,
                Some(SourceId {
                    component: 1,
                    facet: 11
                }),
                "fixture=corner-touch lane={lane}"
            );
            assert_eq!(
                failure.detail, "disconnected contact-star link",
                "fixture=corner-touch lane={lane}"
            );
            eprintln!(
                "CONVEX_SECTION_EXPECTED_REFUSAL fixture=corner-touch lane={lane} kind={:?} source={:?} operations={} detail={}",
                failure.kind, failure.source, failure.operations, failure.detail
            );
        }
        eprintln!(
            "CONVEX_SECTION_NEGATIVE fixture=corner-touch oldWallNanoseconds={} newWallNanoseconds={}",
            old_elapsed.as_nanos(),
            new_elapsed.as_nanos()
        );
        assert!(new_elapsed < old_elapsed.saturating_mul(10));
        Ok(())
    })
}
