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
