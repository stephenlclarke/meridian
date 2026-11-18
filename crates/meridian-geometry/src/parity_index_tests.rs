use super::*;
use crate::{FailureKind, Limits, integer};
use std::collections::BTreeMap;

fn square(parts: i64, offset: i64) -> Vec<Segment> {
    let mut edges = Vec::new();
    let p = |x, y| [integer(x + offset), integer(y)];
    for i in 0..parts {
        edges.extend([
            [p(i, 0), p(i + 1, 0)],
            [p(parts, i), p(parts, i + 1)],
            [p(parts - i, parts), p(parts - i - 1, parts)],
            [p(0, parts - i), p(0, parts - i - 1)],
        ]);
    }
    edges
}
fn frame(duplicate: bool) -> Frame {
    let mut edges = square(16, 0);
    if duplicate {
        edges.extend(edges.clone());
    }
    Frame {
        owners: BTreeMap::from([(0, edges), (1, square(1, 30))]),
        members: 0,
    }
}
fn compare(frame: &Frame, b: &mut Budget) -> Result<(), Failure> {
    let index = ParityIndex::build(frame, b)?
        .ok_or_else(|| b.fail(FailureKind::InvalidWitness, "missing test index"))?;
    let result = (|| {
        for x in -3..35 {
            for y in -3..20 {
                let p = [integer(x), integer(y)];
                let mut reference = Budget::new(Limits::default())?;
                assert_eq!(
                    index.occupied(&p, b)?,
                    frame.occupied(&p, &mut reference)?,
                    "{p:?}"
                );
            }
        }
        Ok(())
    })();
    let members = index.memberships();
    drop(index);
    b.release(members);
    result
}

#[test]
fn closed_interval_index_preserves_boundary_multiplicity_owner_order_and_total_charged_work()
-> Result<(), Failure> {
    for duplicate in [false, true] {
        let frame = frame(duplicate);
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let index = ParityIndex::build(&frame, &mut new)?
            .ok_or_else(|| new.fail(FailureKind::InvalidWitness, "missing test index"))?;
        for x in -3..35 {
            for y in -3..20 {
                let p = [integer(x), integer(y)];
                assert_eq!(
                    index.occupied(&p, &mut new)?,
                    frame.occupied(&p, &mut old)?,
                    "{p:?}"
                );
            }
        }
        // Include full index construction and sorting, never just query work.
        assert!(new.operations() < old.operations());
        assert_eq!(
            index.occupied(&[integer(8), integer(8)], &mut new)?,
            Some(!duplicate)
        );
        for p in [[0, 0], [8, 0], [16, 8], [16, 16], [30, 0], [31, 1]] {
            let p = p.map(integer);
            assert_eq!(index.occupied(&p, &mut new)?, frame.occupied(&p, &mut old)?);
        }
        let members = index.memberships();
        drop(index);
        new.release(members);
        assert_eq!(new.retained_memberships(), 0);
    }
    let small = Frame {
        owners: BTreeMap::from([(0, square(1, 0))]),
        members: 0,
    };
    let mut b = Budget::new(Limits::default())?;
    assert!(ParityIndex::build(&small, &mut b)?.is_none());
    assert_eq!(b.retained_memberships(), 0);
    Ok(())
}

#[test]
fn interval_index_admits_sort_nodes_and_query_stack_and_rolls_back_each_refusal()
-> Result<(), Failure> {
    let frame = frame(false);
    let mut probe = Budget::new(Limits::default())?;
    probe.retain(2000)?;
    compare(&frame, &mut probe)?;
    let cap = probe.peak_memberships();
    assert_eq!(probe.retained_memberships(), 2000);
    let mut exact = Budget::new(Limits {
        storage_memberships: cap,
        ..Limits::default()
    })?;
    exact.retain(2000)?;
    compare(&frame, &mut exact)?;
    assert_eq!(exact.retained_memberships(), 2000);
    assert_eq!(exact.peak_admitted_memberships(), cap);
    let mut short = Budget::new(Limits {
        storage_memberships: cap - 1,
        ..Limits::default()
    })?;
    short.retain(2000)?;
    assert_eq!(
        compare(&frame, &mut short).err().map(|e| e.kind),
        Some(FailureKind::Storage)
    );
    assert_eq!(short.retained_memberships(), 2000);
    let mut work = Budget::new(Limits {
        operations: 3,
        ..Limits::default()
    })?;
    work.retain(2000)?;
    assert_eq!(
        compare(&frame, &mut work).err().map(|e| e.kind),
        Some(FailureKind::Operations)
    );
    assert_eq!(work.retained_memberships(), 2000);
    Ok(())
}
