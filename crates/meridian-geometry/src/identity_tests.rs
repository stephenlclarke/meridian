use super::InputIdentityMaps;
use crate::{Budget, Failure, FailureKind, Limits, SourceId};
use std::collections::BTreeSet;

fn maps() -> InputIdentityMaps {
    // Repeated IDs and owners do not create extra owned records.
    InputIdentityMaps {
        ids: BTreeSet::from([
            SourceId {
                component: 0,
                facet: 0,
            },
            SourceId {
                component: 0,
                facet: 1,
            },
            SourceId {
                component: 1,
                facet: 0,
            },
            SourceId {
                component: 0,
                facet: 0,
            },
        ]),
        owners: BTreeSet::from([0, 1, 0]),
    }
}

#[test]
fn completed_identity_audit_releases_only_owned_records_before_later_scratch() -> Result<(), Failure>
{
    // Frozen independent cardinality fixture: N=3, M=2, input=184, release=13.
    let initial = 184;
    let scratch = 19;
    let mut unreleased = Budget::new(Limits {
        storage_memberships: 190,
        ..Limits::default()
    })?;
    unreleased.retain(initial)?;
    let unreleased_maps = maps();
    assert_eq!(
        unreleased.guard(scratch).err().map(|e| e.kind),
        Some(FailureKind::Storage)
    );

    drop(unreleased_maps);
    unreleased.release(initial);
    assert_eq!(unreleased.retained_memberships(), 0);

    let mut exact = Budget::new(Limits {
        storage_memberships: 190,
        ..Limits::default()
    })?;
    exact.retain(initial)?;
    let work = exact.operations();
    assert_eq!(maps().release(&mut exact), 13);
    assert_eq!(exact.operations(), work);
    assert_eq!(exact.retained_memberships(), 171);
    exact.guard(scratch)?;
    assert_eq!(exact.peak_admitted_memberships(), 190);
    exact.release(171);
    assert_eq!(exact.retained_memberships(), 0);

    let mut short = Budget::new(Limits {
        storage_memberships: 189,
        ..Limits::default()
    })?;
    short.retain(initial)?;
    maps().release(&mut short);
    assert_eq!(
        short.guard(scratch).err().map(|e| e.kind),
        Some(FailureKind::Storage)
    );
    assert_eq!(short.retained_memberships(), 171);
    short.release(171);
    assert_eq!(short.retained_memberships(), 0);

    // The initial admission was not reduced. The empty case is also neutral.
    let mut initial_short = Budget::new(Limits {
        storage_memberships: initial - 1,
        ..Limits::default()
    })?;
    assert_eq!(
        initial_short.retain(initial).err().map(|e| e.kind),
        Some(FailureKind::Storage)
    );
    assert_eq!(initial_short.retained_memberships(), 0);
    assert_eq!(
        InputIdentityMaps {
            ids: BTreeSet::new(),
            owners: BTreeSet::new()
        }
        .release(&mut initial_short),
        0
    );
    Ok(())
}
