use super::*;
use crate::certificate::strict_interior::InteriorCertificates;
use crate::{Limits, Segment};
use std::time::{Duration, Instant};

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/source-work-reference/compiled-publisher-control.rs"
));

fn checked_failure<T>(result: Result<T, Failure>) -> Failure {
    match result {
        Err(failure) => failure,
        Ok(_) => panic!("unexpected success in refusal assertion"),
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

fn tetra() -> Result<Vec<Facet>, Failure> {
    mesh(
        &[[0, 0, 0], [8, 0, 0], [0, 8, 0], [0, 0, 8]],
        &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
        0,
        2,
    )
}
fn timing(label: &str, old: Duration, new: Duration) {
    eprintln!(
        "SOURCE_WORK_TIMING fixture={label} oldWallNanoseconds={} newWallNanoseconds={}",
        old.as_nanos(),
        new.as_nanos()
    );
    assert!(new < old.saturating_mul(10));
}
fn p(x: i64, y: i64) -> Point2 {
    [integer(x), integer(y)]
}
fn planar() -> (Vec<Point2>, Vec<Segment>) {
    (
        vec![p(0, 0), p(8, 0), p(8, 8), p(0, 8)],
        vec![
            [p(1, 1), p(7, 7)],
            [p(-2, 4), p(10, 4)],
            [p(-1, -1), p(-2, -2)],
            [p(4, -2), p(4, 4)],
            [p(2, 8), p(6, 8)],
            [p(3, 3), p(3, 3)],
            [p(6, 2), p(2, 6)],
        ],
    )
}

#[test]
fn certified_source_support_keeps_foreign_physical_contacts_and_full_section_parity()
-> Result<(), Failure> {
    let mut originals = tetra()?;
    originals.extend(cube(1, 4, 1, 0)?);
    audit(&originals)?;
    let mut setup = Budget::new(Limits::default())?;
    let index = FacetBoundsIndex::build(&originals, &mut setup)?;
    let owners = component_bounds(&originals, &index.facets, &mut setup)?;
    let mut old = Budget::new(Limits::default())?;
    let mut new = Budget::new(Limits::default())?;
    old.retain(10000)?;
    new.retain(10000)?;
    // Both publishers already construct the same immutable proof. Include its
    // complete cost and live reservation on both sides of this delta comparison.
    let started = Instant::now();
    let old_certificates = InteriorCertificates::build(&originals, &owners, &mut old)?
        .ok_or_else(|| old.fail(FailureKind::InvalidWitness, "missing reference support"))?;
    let mut old_elapsed = started.elapsed();
    let started = Instant::now();
    let certificates = InteriorCertificates::build(&originals, &owners, &mut new)?
        .ok_or_else(|| new.fail(FailureKind::InvalidWitness, "missing source support"))?;
    let mut new_elapsed = started.elapsed();
    assert!(certificates.source_outward_empty(&originals[48], &mut new)?);
    let mut non_support = triangle([[1, 1, 2], [2, 1, 2], [1, 2, 2]])?;
    // Reconstruct with the certified owner's ID but an interior, non-support plane.
    non_support = Facet::new(
        SourceId {
            component: 0,
            facet: 999,
        },
        non_support.vertices().clone(),
        "test".into(),
        &mut new,
    )?;
    assert!(!certificates.source_outward_empty(&non_support, &mut new)?);
    let mut contacts = 0;
    for (ordinal, face) in originals.iter().enumerate().take(64).skip(48) {
        let started = Instant::now();
        let (a, ac, af) = sections::Frame::collect(
            face,
            &index.facets[ordinal],
            &originals,
            &owners,
            &index,
            &mut old,
        )?;
        old_elapsed += started.elapsed();
        let started = Instant::now();
        let (c, cc, cf) = sections::Frame::collect_certified(
            face,
            &index.facets[ordinal],
            &originals,
            &owners,
            &index,
            &certificates,
            &mut new,
        )?;
        new_elapsed += started.elapsed();
        assert_eq!(a, c);
        assert_eq!(
            ac.iter().map(|f| f.id()).collect::<Vec<_>>(),
            cc.iter().map(|f| f.id()).collect::<Vec<_>>()
        );
        assert_eq!(af.memberships(), cf.memberships());
        contacts += c.len();
        for sample in [p(1, 1), p(2, 3), p(4, 4), p(9, 9)] {
            assert_eq!(
                af.occupied(&sample, &mut old)?,
                cf.occupied(&sample, &mut new)?
            );
        }
        let am = 8 * a.len() + ac.len() + af.memberships();
        let cm = 8 * c.len() + cc.len() + cf.memberships();
        drop((a, ac, af));
        old.release(am);
        drop((c, cc, cf));
        new.release(cm);
    }
    assert!(contacts > 0);
    let baseline = 10000 + certificates.memberships();
    let mut query_tight = Budget::new(Limits {
        storage_memberships: baseline + 31,
        ..Limits::default()
    })?;
    query_tight.retain(baseline)?;
    assert_eq!(
        checked_failure(certificates.source_outward_empty(&originals[48], &mut query_tight)).kind,
        FailureKind::Storage
    );
    assert_eq!(query_tight.retained_memberships(), baseline);
    let mut visit_tight = Budget::new(Limits {
        operations: 1,
        ..Limits::default()
    })?;
    visit_tight.retain(baseline)?;
    assert_eq!(
        checked_failure(sections::Frame::collect_certified(
            &originals[48],
            &index.facets[48],
            &originals,
            &owners,
            &index,
            &certificates,
            &mut visit_tight
        ))
        .kind,
        FailureKind::Operations
    );
    assert_eq!(visit_tight.retained_memberships(), baseline);
    let members = certificates.memberships();
    drop(certificates);
    new.release(members);
    let old_members = old_certificates.memberships();
    drop(old_certificates);
    old.release(old_members);
    assert_eq!(old.retained_memberships(), 10000);
    assert_eq!(new.retained_memberships(), 10000);
    assert!(new.operations() < old.operations());
    eprintln!(
        "SOURCE_WORK_SECTIONS oldOperations={} newOperations={}",
        old.operations(),
        new.operations()
    );
    timing("oblique-sections-including-proof", old_elapsed, new_elapsed);
    Ok(())
}

#[test]
fn source_arrangement_matches_independent_complete_oracle_with_less_actual_endpoint_work()
-> Result<(), Failure> {
    let (square, barriers) = planar();
    for (label, domain) in [
        ("square", square),
        ("triangle", vec![p(0, 0), p(8, 0), p(0, 8)]),
    ] {
        for exposed in [true, false] {
            let mut old = Budget::new(Limits::default())?;
            let mut new = Budget::new(Limits::default())?;
            let started = Instant::now();
            let expected =
                Arrangement::build_accounted(&domain, &barriers, &mut old, |_, _| Ok(exposed))?;
            let old_elapsed = started.elapsed();
            let started = Instant::now();
            let actual =
                Arrangement::build_source(&domain, &barriers, &mut new, |_, _| Ok(exposed))?;
            let new_elapsed = started.elapsed();
            let mut independent = Budget::new(Limits::default())?;
            let oracle =
                Arrangement::build(&domain, &barriers, &mut independent, |_, _| Ok(exposed))?;
            assert_eq!(actual, expected);
            assert_eq!(actual, oracle);
            assert_eq!(old.retained_memberships(), new.retained_memberships());
            assert!(new.operations() < old.operations());
            eprintln!(
                "SOURCE_WORK_ARRANGEMENT fixture={label} exposed={exposed} oldOperations={} newOperations={}",
                old.operations(),
                new.operations()
            );
            timing(label, old_elapsed, new_elapsed);
        }
    }
    Ok(())
}

#[test]
fn source_arrangement_keeps_invalid_input_rational_and_exact_live_callback_guards()
-> Result<(), Failure> {
    let (domain, barriers) = planar();
    for invalid in [
        vec![p(0, 0), p(4, 0), p(1, 1), p(0, 4)],
        vec![p(0, 0), p(0, 4), p(4, 0)],
        vec![p(0, 0), p(4, 0), p(4, 0)],
    ] {
        let mut a = Budget::new(Limits::default())?;
        let mut c = Budget::new(Limits::default())?;
        assert_eq!(
            checked_failure(Arrangement::build_source(&invalid, &[], &mut a, |_, _| Ok(
                true
            )))
            .kind,
            checked_failure(Arrangement::build_accounted(
                &invalid,
                &[],
                &mut c,
                |_, _| Ok(true)
            ))
            .kind
        );
    }
    let huge = Rational::from_integer(num_bigint::BigInt::from(1) << 4096usize);
    let mut b = Budget::new(Limits::default())?;
    assert_eq!(
        checked_failure(Arrangement::build_source(
            &domain,
            &[[[huge, integer(1)], p(2, 2)]],
            &mut b,
            |_, _| Ok(true)
        ))
        .kind,
        FailureKind::Rational
    );
    let mut measured = Budget::new(Limits::default())?;
    measured.retain(2000)?;
    let expected = Arrangement::build_source(&domain, &barriers, &mut measured, |_, b| {
        b.guard(31)?;
        Ok(true)
    })?;
    let cap = measured.peak_memberships();
    let mut exact = Budget::new(Limits {
        storage_memberships: cap,
        ..Limits::default()
    })?;
    exact.retain(2000)?;
    assert_eq!(
        Arrangement::build_source(&domain, &barriers, &mut exact, |_, b| {
            b.guard(31)?;
            Ok(true)
        })?,
        expected
    );
    let mut tight = Budget::new(Limits {
        storage_memberships: cap - 1,
        ..Limits::default()
    })?;
    tight.retain(2000)?;
    assert_eq!(
        checked_failure(Arrangement::build_source(
            &domain,
            &barriers,
            &mut tight,
            |_, b| {
                b.guard(31)?;
                Ok(true)
            }
        ))
        .kind,
        FailureKind::Storage
    );
    assert_eq!(tight.retained_memberships(), 2000);
    let mut refused = Budget::new(Limits::default())?;
    refused.retain(2000)?;
    assert_eq!(
        checked_failure(Arrangement::build_source(
            &domain,
            &barriers,
            &mut refused,
            |_, b| Err(b.fail(FailureKind::InvalidWitness, "test callback refusal"))
        ))
        .kind,
        FailureKind::InvalidWitness
    );
    assert_eq!(refused.retained_memberships(), 2000);
    Ok(())
}

#[test]
fn source_work_complete_publisher_matches_frozen_output_fields_contacts_and_volume()
-> Result<(), Failure> {
    let mut originals = tetra()?;
    originals.extend(cube(1, 4, 1, 0)?);
    let mut old = Budget::new(Limits::default())?;
    let mut new = Budget::new(Limits::default())?;
    let started = Instant::now();
    let expected = reference_source_work_publisher(&originals, &mut old)?;
    let old_elapsed = started.elapsed();
    let started = Instant::now();
    let actual = reconstruct(&originals, &mut new)?;
    let new_elapsed = started.elapsed();
    assert_eq!(actual.triangles(), expected.triangles());
    assert_eq!(actual.witnesses(), expected.witnesses());
    assert_eq!(actual.exact_volume(), expected.exact_volume());
    assert_eq!(new.retained_memberships(), old.retained_memberships());
    assert!(new.operations() < old.operations());
    eprintln!(
        "SOURCE_WORK_COMPLETE oldOperations={} newOperations={} oldPeak={} newPeak={}",
        old.operations(),
        new.operations(),
        old.peak_memberships(),
        new.peak_memberships()
    );
    timing("oblique-full-publisher", old_elapsed, new_elapsed);
    Ok(())
}
