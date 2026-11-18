use super::*;
use crate::Limits;

mod reference {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/live-runs-reference/compiled-arrangement-control.rs"
    ));
}

fn fixture(count: i64) -> (Vec<Point2>, Vec<Segment>) {
    let domain = [[0, 0], [100, 0], [100, 10], [0, 10]]
        .map(|p| p.map(integer))
        .to_vec();
    let barriers = (1..=count)
        .map(|x| {
            let p = [integer(x), integer(5)];
            [p.clone(), p]
        })
        .collect();
    (domain, barriers)
}

#[test]
fn live_corners_match_frozen_complete_arrangements_with_protected_run_extensions()
-> Result<(), Failure> {
    for count in [0, 1, 8, 80] {
        let (domain, barriers) = fixture(count);
        for exposed in [true, false] {
            let mut old = Budget::new(Limits::default())?;
            let mut new = Budget::new(Limits::default())?;
            let a =
                Arrangement::build_live_runs_reference(&domain, &barriers, &mut old, |_, b| {
                    b.guard(32)?;
                    Ok(exposed)
                })?;
            let c = Arrangement::build_accounted(&domain, &barriers, &mut new, |_, b| {
                b.guard(32)?;
                Ok(exposed)
            })?;
            assert_eq!(c, a);
            assert_eq!(new.retained_memberships(), 0);
            if exposed && count == 80 {
                assert!(c.work.strips_extended > 0);
                assert!(c.work.point_contact_chords > 0);
                assert!(new.peak_memberships() < old.peak_memberships());
            }
        }
    }
    Ok(())
}

#[test]
fn actual_run_lifetimes_keep_exact_admission_and_refusal_cleanup() -> Result<(), Failure> {
    let (domain, barriers) = fixture(80);
    let mut probe = Budget::new(Limits::default())?;
    probe.retain(2000)?;
    let expected = Arrangement::build_accounted(&domain, &barriers, &mut probe, |_, b| {
        b.guard(32)?;
        Ok(true)
    })?;
    let cap = probe.peak_memberships();
    let mut exact = Budget::new(Limits {
        storage_memberships: cap,
        ..Limits::default()
    })?;
    exact.retain(2000)?;
    let actual = Arrangement::build_accounted(&domain, &barriers, &mut exact, |_, b| {
        b.guard(32)?;
        Ok(true)
    })?;
    assert_eq!(actual, expected);
    assert_eq!(exact.retained_memberships(), 2000);
    assert_eq!(exact.peak_admitted_memberships(), cap);
    let mut tight = Budget::new(Limits {
        storage_memberships: cap - 1,
        ..Limits::default()
    })?;
    tight.retain(2000)?;
    assert_eq!(
        Arrangement::build_accounted(&domain, &barriers, &mut tight, |_, b| {
            b.guard(32)?;
            Ok(true)
        })
        .err()
        .map(|e| e.kind),
        Some(FailureKind::Storage)
    );
    assert_eq!(tight.retained_memberships(), 2000);
    let mut old = Budget::new(Limits {
        storage_memberships: cap,
        ..Limits::default()
    })?;
    old.retain(2000)?;
    assert_eq!(
        Arrangement::build_live_runs_reference(&domain, &barriers, &mut old, |_, b| {
            b.guard(32)?;
            Ok(true)
        })
        .err()
        .map(|e| e.kind),
        Some(FailureKind::Storage)
    );
    assert_eq!(old.retained_memberships(), 2000);
    let mut refused = Budget::new(Limits::default())?;
    refused.retain(2000)?;
    let result = Arrangement::build_accounted(&domain, &barriers, &mut refused, |_, b| {
        Err(b.fail(FailureKind::InvalidWitness, "test callback refusal"))
    });
    assert_eq!(
        result.err().map(|e| e.kind),
        Some(FailureKind::InvalidWitness)
    );
    assert_eq!(refused.retained_memberships(), 2000);
    Ok(())
}

#[test]
fn incremental_memberships_follow_each_protected_collinear_and_duplicate_lifetime()
-> Result<(), Failure> {
    let p = |x, y| [integer(x), integer(y)];
    let mut b = Budget::new(Limits::default())?;
    let mut members = 4;
    let mut lower = vec![p(0, 0), p(1, 0)];
    let mut upper = vec![p(0, 2), p(1, 2)];
    let protected = BTreeSet::from([p(2, 0)]);
    extend(&mut lower, p(2, 0), &protected, &mut members, &mut b)?;
    assert_eq!(lower, vec![p(0, 0), p(2, 0)]);
    assert_eq!(members, 4);
    extend(&mut upper, p(2, 2), &protected, &mut members, &mut b)?;
    assert_eq!(members, 4);
    extend(&mut lower, p(3, 0), &protected, &mut members, &mut b)?;
    assert_eq!(lower, vec![p(0, 0), p(2, 0), p(3, 0)]);
    assert_eq!(members, 5);
    extend(&mut upper, p(3, 2), &protected, &mut members, &mut b)?;
    assert_eq!(members, 5);
    let mut cells = Vec::new();
    let mut work = ArrangementWork::default();
    finish(
        Run { lower, upper },
        &mut cells,
        &mut members,
        &mut work,
        &mut b,
    )?;
    assert_eq!(members, 5);
    // Right apex: adjacent duplicate removed by the original dedup visit.
    members += 4;
    finish(
        Run {
            lower: vec![p(4, 0), p(6, 0)],
            upper: vec![p(4, 2), p(6, 0)],
        },
        &mut cells,
        &mut members,
        &mut work,
        &mut b,
    )?;
    assert_eq!(members, 8);
    // Left apex: the final closing duplicate is dropped after dedup.
    members += 4;
    finish(
        Run {
            lower: vec![p(8, 0), p(10, 0)],
            upper: vec![p(8, 0), p(10, 2)],
        },
        &mut cells,
        &mut members,
        &mut work,
        &mut b,
    )?;
    assert_eq!(members, 11);
    // A separate adjacent duplicate has the same owning-record lifetime.
    members += 5;
    finish(
        Run {
            lower: vec![p(12, 0), p(13, 0), p(13, 0)],
            upper: vec![p(12, 2), p(13, 2)],
        },
        &mut cells,
        &mut members,
        &mut work,
        &mut b,
    )?;
    assert_eq!(members, 15);
    assert_eq!(members, cells.iter().map(Vec::len).sum::<usize>());
    assert!(cells[0].contains(&p(2, 0)));
    assert_eq!(work.retained_cells, 4);
    assert_eq!(b.retained_memberships(), 0);
    Ok(())
}
