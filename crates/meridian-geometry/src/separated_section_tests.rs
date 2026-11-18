use super::convex_sections::Rows;
use super::strict_interior::InteriorCertificates;
use super::*;
use crate::{AffineTransform, Limits};
use std::time::Instant;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/separated-section-reference/compiled-publisher-control.rs"
));

fn checked_failure<T>(result: Result<T, Failure>) -> Failure {
    match result {
        Err(failure) => failure,
        Ok(_) => panic!("unexpected success in separated-section refusal test"),
    }
}
fn labelled<T>(
    fixture: &str,
    lane: &str,
    run: impl FnOnce() -> Result<T, Failure>,
) -> Result<T, Failure> {
    match run() {
        Err(failure) => {
            eprintln!(
                "SEPARATED_SECTION_FAILURE fixture={fixture} lane={lane} kind={:?} source={:?} operations={} detail={}",
                failure.kind, failure.source, failure.operations, failure.detail
            );
            Err(failure)
        }
        Ok(value) => Ok(value),
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

fn tetra() -> Result<Vec<Facet>, Failure> {
    mesh(
        &[[0, 0, 0], [2, 0, 0], [0, 2, 0], [0, 0, 2]],
        &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
        0,
        2,
    )
}
fn triangle(points: [[i64; 3]; 3], owner: u32) -> Result<Facet, Failure> {
    let mut b = Budget::new(Limits::default())?;
    Facet::new(
        SourceId {
            component: owner,
            facet: 0,
        },
        points.map(|p| {
            let point = p.map(integer);
            Vertex {
                uv: [point[0].clone(), point[1].clone()],
                point,
            }
        }),
        "test".into(),
        &mut b,
    )
}
fn drop_rows(rows: Rows, b: &mut Budget) {
    let members = rows.memberships();
    drop(rows);
    b.release(members);
}

#[test]
fn source_joint_sections_match_all_signed_patterns_and_keep_endpoint_coordinate_guards()
-> Result<(), Failure> {
    labelled(
        "source_joint_sections_match_all_signed_patterns_and_keep_endpoint_coordinate_guards",
        "case-return",
        || -> Result<(), Failure> {
            let face = triangle([[0, 0, 0], [2, 0, 0], [0, 2, 0]], 0)?;
            for (physical, outward) in [(true, true), (true, false), (false, true), (false, false)]
            {
                let mut old = Budget::new(Limits::default())?;
                let mut new = Budget::new(Limits::default())?;
                let mut old_elapsed = std::time::Duration::ZERO;
                let mut new_elapsed = std::time::Duration::ZERO;
                for a in [-2, 0, 3] {
                    for c in [-2, 0, 3] {
                        for d in [-2, 0, 3] {
                            let other = triangle([[0, 0, a], [2, 0, c], [0, 2, d]], 1)?;
                            let fixture =
                                format!("signed-{a}-{c}-{d}-physical{physical}-outward{outward}");
                            let before_old = old.operations();
                            let before_new = new.operations();
                            let start = Instant::now();
                            let expected = labelled(&fixture, "reference-joint", || {
                                old.with_retained(96, |b| {
                                    sections::joint_section(&face, &other, physical, outward, b)
                                })
                            })?;
                            old_elapsed += start.elapsed();
                            let start = Instant::now();
                            let actual = labelled(&fixture, "candidate-joint", || {
                                new.with_retained(96, |b| {
                                    sections::joint_section_source(
                                        &face, &other, physical, outward, b,
                                    )
                                })
                            })?;
                            new_elapsed += start.elapsed();
                            assert_eq!(
                                actual, expected,
                                "pattern={a},{c},{d} physical={physical} outward={outward}"
                            );
                            if a == c && c == d && a != 0 {
                                assert!(
                                    new.operations() - before_new < old.operations() - before_old
                                );
                            }
                            assert_eq!(old.retained_memberships(), 0);
                            assert_eq!(new.retained_memberships(), 0);
                        }
                    }
                }
                eprintln!(
                    "SEPARATED_SECTION_SIGNED fixture=27patterns physical={physical} outward={outward} oldOperations={} newOperations={} oldWallNanoseconds={} newWallNanoseconds={}",
                    old.operations(),
                    new.operations(),
                    old_elapsed.as_nanos(),
                    new_elapsed.as_nanos()
                );
                assert!(new_elapsed < old_elapsed.saturating_mul(10));
            }
            // A zero-distance endpoint is reused, but its original coordinate check
            // still runs under the current caller's tighter rational limit.
            let other = triangle([[32, 0, 0], [2, 0, 1], [0, 2, 1]], 1)?;
            let mut reference = Budget::new(Limits {
                rational_bits: 4,
                ..Limits::default()
            })?;
            assert_eq!(
                checked_failure(reference.with_retained(96, |b| sections::joint_section(
                    &face, &other, false, true, b
                )))
                .kind,
                FailureKind::Rational
            );
            assert_eq!(reference.retained_memberships(), 0);
            let mut b = Budget::new(Limits {
                rational_bits: 4,
                ..Limits::default()
            })?;
            assert_eq!(
                checked_failure(b.with_retained(96, |b| sections::joint_section_source(
                    &face, &other, false, true, b
                )))
                .kind,
                FailureKind::Rational
            );
            assert_eq!(b.retained_memberships(), 0);
            Ok(())
        },
    )
}

#[test]
fn strict_owner_support_separation_keeps_complete_clipped_constraints_and_equality_contacts()
-> Result<(), Failure> {
    labelled(
        "strict_owner_support_separation_keeps_complete_clipped_constraints_and_equality_contacts",
        "case-return",
        || -> Result<(), Failure> {
            let originals = tetra()?;
            let mut setup = Budget::new(Limits::default())?;
            let index = FacetBoundsIndex::build(&originals, &mut setup)?;
            let owners = component_bounds(&originals, &index.facets, &mut setup)?;
            audit_original_sheets(&originals, &mut setup)?;
            let ids = originals.iter().map(|f| f.id().component).collect();
            audit_original_owner_ranges(&originals, &ids, &mut setup)?;
            let certificates =
                InteriorCertificates::build_sections(&originals, &owners, &mut setup)?.ok_or_else(
                    || setup.fail(FailureKind::InvalidWitness, "missing support certificate"),
                )?;
            for (fixture, points, separated) in [
                ("parallel-gap", [[2, 2, 0], [2, 0, 2], [0, 2, 2]], true),
                ("finite-gap", [[2, 2, 0], [2, 2, 1], [1, 2, 2]], true),
                ("support-equality", [[2, 0, 0], [0, 2, 0], [0, 0, 2]], false),
            ] {
                let face = triangle(points, 99)?;
                let bounds = Bounds::for_facet(&face, &mut setup)?;
                let mut domain = face.domain().to_vec();
                if area(&domain, &mut setup)?.is_negative() {
                    domain.reverse();
                }
                let baseline = Facet::input_memberships(&originals, &mut setup)?
                    + FacetBoundsIndex::memberships(originals.len())
                    + certificates.memberships()
                    + 256;
                let mut old = Budget::new(Limits::default())?;
                old.retain(baseline)?;
                let mut new = Budget::new(Limits::default())?;
                new.retain(baseline)?;
                let start = Instant::now();
                let old_rows = labelled(fixture, "reference-row-build", || {
                    Rows::build(&face, &bounds, Some(&certificates), &mut old)
                })?;
                let (a, ac, af) = labelled(fixture, "reference-collection", || {
                    sections::Frame::collect_rows(
                        &face, &bounds, &originals, &owners, &index, &old_rows, &mut old,
                    )
                })?;
                let mut old_elapsed = start.elapsed();
                let start = Instant::now();
                let new_rows = labelled(fixture, "candidate-row-build", || {
                    Rows::build_separating(&face, &bounds, Some(&certificates), &mut new)
                })?;
                let (c, cc, cf) = labelled(fixture, "candidate-collection", || {
                    sections::Frame::collect_separating(
                        &face, &bounds, &originals, &owners, &index, &new_rows, &mut new,
                    )
                })?;
                let mut new_elapsed = start.elapsed();
                assert_eq!(new_rows.separated(0, &mut new)?, separated);
                if !separated {
                    assert_eq!(a, c);
                    assert_eq!(
                        ac.iter().map(|f| f.id()).collect::<Vec<_>>(),
                        cc.iter().map(|f| f.id()).collect::<Vec<_>>()
                    );
                    assert!(!c.is_empty());
                    assert!(!cc.is_empty());
                }
                let start = Instant::now();
                let expected = labelled(fixture, "reference-arrangement", || {
                    Arrangement::build_source(&domain, &a, &mut old, |p, b| {
                        match old_rows.occupied(p, b)? {
                            Some(value) => Ok(!value),
                            None => Err(b.fail(
                                FailureKind::UncertifiedContact,
                                "unexpected reference boundary in separated fixture",
                            )),
                        }
                    })
                })?;
                old_elapsed += start.elapsed();
                let start = Instant::now();
                let actual = labelled(fixture, "candidate-arrangement", || {
                    Arrangement::build_source(&domain, &c, &mut new, |p, b| {
                        match new_rows.occupied_facet(p, b)? {
                            Some(value) => Ok(!value),
                            None => Err(b.fail(
                                FailureKind::UncertifiedContact,
                                "unexpected candidate boundary in separated fixture",
                            )),
                        }
                    })
                })?;
                new_elapsed += start.elapsed();
                assert_eq!(actual, expected);
                if fixture == "parallel-gap" {
                    assert!(new.operations() < old.operations());
                }
                eprintln!(
                    "SEPARATED_SECTION_CLIPPED fixture={fixture} oldOperations={} newOperations={} oldWallNanoseconds={} newWallNanoseconds={}",
                    old.operations(),
                    new.operations(),
                    old_elapsed.as_nanos(),
                    new_elapsed.as_nanos()
                );
                assert!(new_elapsed < old_elapsed.saturating_mul(10));
                let am = 8 * a.len() + ac.len() + af.memberships();
                let cm = 8 * c.len() + cc.len() + cf.memberships();
                drop((a, ac, af));
                old.release(am);
                drop((c, cc, cf));
                new.release(cm);
                drop_rows(old_rows, &mut old);
                drop_rows(new_rows, &mut new);
                assert_eq!(old.retained_memberships(), baseline);
                assert_eq!(new.retained_memberships(), baseline);
            }
            let members = certificates.memberships();
            drop(certificates);
            setup.release(members);
            assert_eq!(setup.retained_memberships(), 0);
            Ok(())
        },
    )
}

#[test]
fn separated_original_owners_preserve_complete_publisher_fields_witnesses_and_analytic_volume()
-> Result<(), Failure> {
    labelled("opposite-tetrahedra", "case-return", || {
        for level in [0, 2] {
            let fixture = if level == 0 {
                "opposite-primitive-tetrahedra"
            } else {
                "opposite-subdivided-tetrahedra"
            };
            let mut originals = mesh(
                &[[0, 0, 0], [2, 0, 0], [0, 2, 0], [0, 0, 2]],
                &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
                0,
                level,
            )?;
            let mut build = Budget::new(Limits::default())?;
            let transform = AffineTransform {
                linear: [
                    [integer(-1), integer(0), integer(0)],
                    [integer(0), integer(-1), integer(0)],
                    [integer(0), integer(0), integer(-1)],
                ],
                translation: [integer(2), integer(2), integer(2)],
            };
            for face in mesh(
                &[[0, 0, 0], [2, 0, 0], [0, 2, 0], [0, 0, 2]],
                &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
                0,
                level,
            )? {
                let reflected = transform.apply(&face, &mut build)?;
                originals.push(Facet::new(
                    SourceId {
                        component: 1,
                        facet: face.id().facet,
                    },
                    reflected.vertices().clone(),
                    reflected.style().to_owned(),
                    &mut build,
                )?);
            }
            let mut old = Budget::new(Limits::default())?;
            let mut new = Budget::new(Limits::default())?;
            let start = Instant::now();
            let expected = labelled(fixture, "reference-publisher", || {
                reference_separated_section_publisher(&originals, &mut old)
            })?;
            let old_elapsed = start.elapsed();
            let start = Instant::now();
            let actual = labelled(fixture, "candidate-publisher", || {
                reconstruct(&originals, &mut new)
            })?;
            let new_elapsed = start.elapsed();
            assert_eq!(actual.triangles(), expected.triangles());
            assert_eq!(actual.witnesses(), expected.witnesses());
            assert_eq!(actual.exact_volume(), expected.exact_volume());
            assert_eq!(actual.exact_volume(), &(integer(8) / integer(3)));
            assert_eq!(new.retained_memberships(), old.retained_memberships());
            if level == 0 {
                assert!(new.operations() < old.operations());
            }
            eprintln!(
                "SEPARATED_SECTION_COMPLETE fixture=opposite-tetrahedra level={level} oldOperations={} newOperations={} oldPeak={} newPeak={} oldWallNanoseconds={} newWallNanoseconds={}",
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
    })
}

#[test]
fn separated_rows_and_source_section_scratch_keep_exact_refusals_and_cleanup() -> Result<(), Failure>
{
    labelled(
        "separated_rows_and_source_section_scratch_keep_exact_refusals_and_cleanup",
        "case-return",
        || -> Result<(), Failure> {
            let originals = tetra()?;
            let mut setup = Budget::new(Limits::default())?;
            let index = FacetBoundsIndex::build(&originals, &mut setup)?;
            let owners = component_bounds(&originals, &index.facets, &mut setup)?;
            let ids = originals.iter().map(|f| f.id().component).collect();
            audit_original_owner_ranges(&originals, &ids, &mut setup)?;
            audit_original_sheets(&originals, &mut setup)?;
            let cert = InteriorCertificates::build_sections(&originals, &owners, &mut setup)?
                .ok_or_else(|| setup.fail(FailureKind::InvalidWitness, "missing certificate"))?;
            let face = triangle([[2, 2, 0], [2, 0, 2], [0, 2, 2]], 99)?;
            let bounds = Bounds::for_facet(&face, &mut setup)?;
            let baseline = Facet::input_memberships(&originals, &mut setup)?
                + FacetBoundsIndex::memberships(originals.len())
                + cert.memberships()
                + 256;
            let mut measured = Budget::new(Limits::default())?;
            measured.retain(baseline)?;
            let rows = Rows::build_separating(&face, &bounds, Some(&cert), &mut measured)?;
            let cap = measured.peak_memberships();
            drop_rows(rows, &mut measured);
            let mut exact = Budget::new(Limits {
                storage_memberships: cap,
                ..Limits::default()
            })?;
            exact.retain(baseline)?;
            let rows = Rows::build_separating(&face, &bounds, Some(&cert), &mut exact)?;
            drop_rows(rows, &mut exact);
            assert_eq!(exact.retained_memberships(), baseline);
            let mut tight = Budget::new(Limits {
                storage_memberships: cap - 1,
                ..Limits::default()
            })?;
            tight.retain(baseline)?;
            assert_eq!(
                checked_failure(Rows::build_separating(
                    &face,
                    &bounds,
                    Some(&cert),
                    &mut tight
                ))
                .kind,
                FailureKind::Storage
            );
            assert_eq!(tight.retained_memberships(), baseline);
            let rows = Rows::build_separating(&face, &bounds, Some(&cert), &mut measured)?;
            let retained = measured.retained_memberships();
            let mut short = Budget::new(Limits {
                operations: 1,
                ..Limits::default()
            })?;
            short.retain(retained)?;
            assert_eq!(
                checked_failure(sections::Frame::collect_separating(
                    &face, &bounds, &originals, &owners, &index, &rows, &mut short
                ))
                .kind,
                FailureKind::Operations
            );
            assert_eq!(short.retained_memberships(), retained);
            drop_rows(rows, &mut measured);
            assert_eq!(measured.retained_memberships(), baseline);
            let other = triangle([[0, 0, -1], [2, 0, -1], [0, 2, -1]], 1)?;
            let plane = triangle([[0, 0, 0], [2, 0, 0], [0, 2, 0]], 0)?;
            for (cap, expected) in [(104, None), (103, Some(FailureKind::Storage))] {
                let mut b = Budget::new(Limits {
                    storage_memberships: baseline + cap,
                    ..Limits::default()
                })?;
                b.retain(baseline)?;
                let result = b.with_retained(96, |b| {
                    sections::joint_section_source(&plane, &other, true, true, b)
                });
                match expected {
                    None => {
                        assert_eq!(result?, (Vec::new(), false, None));
                    }
                    Some(kind) => assert_eq!(checked_failure(result).kind, kind),
                }
                assert_eq!(b.retained_memberships(), baseline);
            }
            for (operations, expected) in [(4, None), (3, Some(FailureKind::Operations))] {
                let mut b = Budget::new(Limits {
                    operations,
                    ..Limits::default()
                })?;
                b.retain(baseline)?;
                let result = b.with_retained(96, |b| {
                    sections::joint_section_source(&plane, &other, true, true, b)
                });
                match expected {
                    None => assert_eq!(result?, (Vec::new(), false, None)),
                    Some(kind) => assert_eq!(checked_failure(result).kind, kind),
                }
                assert_eq!(b.operations(), 4);
                assert_eq!(b.retained_memberships(), baseline);
            }
            let members = cert.memberships();
            drop(cert);
            setup.release(members);
            assert_eq!(setup.retained_memberships(), 0);
            Ok(())
        },
    )
}
