use super::{audit_original_components, audit_original_owner_ranges};
use crate::{Budget, Facet, Failure, FailureKind, Limits, SourceId, Vertex, integer};
use std::collections::BTreeSet;

fn tetrahedron(owner: u32, first: u32, reflected: bool) -> Result<Vec<Facet>, Failure> {
    let mut b = Budget::new(Limits::default())?;
    let s = if reflected { -1 } else { 1 };
    let points = [[0, 0, 0], [s, 0, 0], [0, s, 0], [0, 0, s]].map(|p| p.map(integer));
    [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]
        .into_iter()
        .enumerate()
        .map(|(i, mut indices)| {
            if reflected {
                indices.swap(1, 2);
            }
            Facet::new(
                SourceId {
                    component: owner,
                    facet: first + i as u32,
                },
                indices.map(|j| Vertex {
                    point: points[j].clone(),
                    uv: [integer(0), integer(0)],
                }),
                "test".into(),
                &mut b,
            )
        })
        .collect()
}

#[test]
fn sorted_owner_ranges_preserve_closed_audits_and_eliminate_full_input_scans() -> Result<(), Failure>
{
    for (ids, saving) in [(&[2, 5, 9][..], 57), (&[2][..], 7)] {
        let owners = ids.iter().copied().collect::<BTreeSet<_>>();
        let mut faces = Vec::new();
        for owner in ids {
            faces.extend(tetrahedron(*owner, 0, false)?);
        }
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        old.retain(700)?;
        new.retain(700)?;
        audit_original_components(&faces, &owners, &mut old)?;
        audit_original_owner_ranges(&faces, &owners, &mut new)?;
        assert_eq!(old.operations() - new.operations(), saving);
        assert_eq!(new.retained_memberships(), 700);
        let peak = new.peak_memberships();
        let mut exact = Budget::new(Limits {
            storage_memberships: peak,
            ..Limits::default()
        })?;
        exact.retain(700)?;
        audit_original_owner_ranges(&faces, &owners, &mut exact)?;
        assert_eq!(exact.retained_memberships(), 700);
        let mut short = Budget::new(Limits {
            storage_memberships: peak - 1,
            ..Limits::default()
        })?;
        short.retain(700)?;
        assert_eq!(
            audit_original_owner_ranges(&faces, &owners, &mut short)
                .err()
                .map(|e| e.kind),
            Some(FailureKind::Storage)
        );
        assert_eq!(short.retained_memberships(), 700);
    }
    Ok(())
}

#[test]
fn owner_order_fallback_keeps_original_semantic_refusal_and_source() -> Result<(), Failure> {
    let owners = BTreeSet::from([2, 5, 9]);
    let mut faces = tetrahedron(2, 0, false)?;
    let mut open = tetrahedron(5, 0, false)?;
    open.pop();
    faces.extend(open);
    faces.extend(tetrahedron(9, 0, false)?);
    for unordered in [false, true] {
        let mut input = faces.clone();
        if unordered {
            input.rotate_right(1);
        }
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let before = audit_original_components(&input, &owners, &mut old).err();
        let after = audit_original_owner_ranges(&input, &owners, &mut new).err();
        let semantic = |e: Failure| (e.kind, e.source, e.detail);
        assert_eq!(
            after.as_ref().map(|e| e.kind),
            Some(FailureKind::NonManifold)
        );
        assert_eq!(before.map(semantic), after.map(semantic));
        if unordered {
            assert_eq!(new.operations(), old.operations() + 2);
        }
        assert_eq!(new.retained_memberships(), 0);
    }
    // One owner can have several sheets and still must receive one complete
    // link audit. No owner range is split at contacts or disconnected sheets.
    let mut contact = tetrahedron(2, 0, false)?;
    contact.extend(tetrahedron(2, 4, true)?);
    assert_eq!(
        after_kind(&contact, &BTreeSet::from([2]))?,
        Some(FailureKind::UncertifiedContact)
    );
    Ok(())
}

fn after_kind(faces: &[Facet], owners: &BTreeSet<u32>) -> Result<Option<FailureKind>, Failure> {
    let mut b = Budget::new(Limits::default())?;
    Ok(audit_original_owner_ranges(faces, owners, &mut b)
        .err()
        .map(|e| e.kind))
}
