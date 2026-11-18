mod before {
    include!("../fixtures/adapter-context-reference/normalized-before-adapter.rs");
}
use crate::{Budget, BudgetPhase, Failure, FailureKind, Limits, OriginalAsset};
use std::collections::BTreeSet;

#[test]
fn authenticated_encoded_styles_match_frozen_decoding_and_remove_only_repeated_hex_work()
-> Result<(), Box<dyn std::error::Error>> {
    for (asset, reference, saving) in [
        (OriginalAsset::Sh010, before::OriginalAsset::Sh010, 38228),
        (OriginalAsset::Sh011, before::OriginalAsset::Sh011, 38228),
        (OriginalAsset::Sh016, before::OriginalAsset::Sh016, 41572),
        (OriginalAsset::Sh017, before::OriginalAsset::Sh017, 41572),
        (OriginalAsset::Sh166, before::OriginalAsset::Sh166, 37316),
    ] {
        assert_eq!(asset.id(), reference.id());
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let old_start = std::time::Instant::now();
        let expected = reference.decode(reference.pinned_bytes(), &mut old)?;
        let old_elapsed = old_start.elapsed();
        let new_start = std::time::Instant::now();
        let actual = asset.decode(asset.pinned_bytes(), &mut new)?;
        let new_elapsed = new_start.elapsed();
        assert_eq!(actual, expected);
        assert_eq!(old.operations() - new.operations(), saving);
        assert_eq!(
            new.phase_operations()[BudgetPhase::Decode as usize],
            new.operations()
        );
        assert_eq!(new.retained_memberships(), 0);
        let text = std::str::from_utf8(asset.pinned_bytes())?;
        let encodings = text
            .lines()
            .skip(1)
            .map(|line| line.split_whitespace().nth(2).ok_or("missing encoding"))
            .collect::<Result<BTreeSet<_>, _>>()?;
        assert_eq!(encodings.len(), 6);
        let mut old_input = Budget::new(Limits::default())?;
        let mut new_input = Budget::new(Limits::default())?;
        assert_eq!(
            crate::Facet::input_memberships(&actual, &mut new_input)?,
            crate::Facet::input_memberships(&expected, &mut old_input)?
        );
        assert!(new_elapsed < old_elapsed.saturating_mul(10));
        eprintln!(
            "adapterStyles id={} referenceOperations={} candidateOperations={} referencePeak={} candidatePeak={} referenceWallNs={} candidateWallNs={}",
            asset.id(),
            old.operations(),
            new.operations(),
            old.peak_memberships(),
            new.peak_memberships(),
            old_elapsed.as_nanos(),
            new_elapsed.as_nanos()
        );

        let mut rejected = Budget::new(Limits {
            operations: asset.pinned_bytes().len() as u64 - 1,
            ..Limits::default()
        })?;
        assert_eq!(
            asset
                .decode(asset.pinned_bytes(), &mut rejected)
                .err()
                .map(|e| e.kind),
            Some(FailureKind::Operations)
        );
        let snapshot = rejected
            .rejection()
            .ok_or("missing byte-authentication refusal")?;
        assert_eq!(snapshot.phase, BudgetPhase::Decode);
        assert_eq!(snapshot.operations_before, 0);
        assert_eq!(
            snapshot.attempted_operations,
            asset.pinned_bytes().len() as u64
        );
        assert!(snapshot.file.ends_with("adapter.rs"));
        assert_eq!(snapshot.retained_before, 0);
    }
    Ok(())
}

#[test]
fn rejected_budget_snapshot_survives_scope_cleanup_with_exact_stage_charges() -> Result<(), Failure>
{
    let mut storage = Budget::new(Limits {
        storage_memberships: 100,
        ..Limits::default()
    })?;
    let failure = storage
        .with_phase(BudgetPhase::Packing, |b| {
            b.with_retained(18, |b| {
                b.state([1, 2, 3, 4, 5, 6, 7, 8]);
                b.guard(83)
            })
        })
        .err();
    assert_eq!(failure.as_ref().map(|e| e.kind), Some(FailureKind::Storage));
    let saved = storage
        .rejection()
        .ok_or_else(|| storage.fail(FailureKind::InvalidWitness, "missing storage snapshot"))?;
    assert_eq!(saved.retained_before, 18);
    assert_eq!(saved.attempted_scratch, 83);
    assert_eq!(saved.required_memberships, Some(101));
    assert_eq!(saved.phase, BudgetPhase::Packing);
    assert_eq!(saved.state, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(storage.retained_memberships(), 0);
    assert!(saved.file.ends_with("adapter_context_tests.rs"));
    assert!(saved.line > 0);
    let _ = storage.with_phase(BudgetPhase::Other, |b| b.guard(200));
    assert_eq!(storage.rejection(), Some(saved));

    let mut work = Budget::new(Limits {
        operations: 3,
        ..Limits::default()
    })?;
    work.step(1)?;
    let failure = work
        .with_phase(BudgetPhase::Occupancy, |b| {
            b.with_retained(23, |b| b.step(3))
        })
        .err();
    assert_eq!(
        failure.as_ref().map(|e| e.kind),
        Some(FailureKind::Operations)
    );
    let saved = work
        .rejection()
        .ok_or_else(|| work.fail(FailureKind::InvalidWitness, "missing work snapshot"))?;
    assert_eq!(saved.operations_before, 1);
    assert_eq!(saved.operations_after, 4);
    assert_eq!(saved.attempted_operations, 3);
    assert_eq!(saved.retained_before, 23);
    assert_eq!(saved.phase_operations[BudgetPhase::Other as usize], 1);
    assert_eq!(saved.phase_operations[BudgetPhase::Occupancy as usize], 3);
    assert_eq!(saved.phase_operations.iter().sum::<u64>(), 4);
    assert_eq!(work.retained_memberships(), 0);
    assert!(saved.file.ends_with("adapter_context_tests.rs"));
    let mut admission = Budget::new(Limits {
        storage_memberships: 10,
        ..Limits::default()
    })?;
    let _ = admission.with_retained(11, |_| Ok(()));
    let saved = admission
        .rejection()
        .ok_or_else(|| admission.fail(FailureKind::InvalidWitness, "missing admission snapshot"))?;
    assert!(saved.file.ends_with("adapter_context_tests.rs"));
    assert_eq!(saved.retained_before, 0);
    assert_eq!(saved.attempted_scratch, 11);
    Ok(())
}
