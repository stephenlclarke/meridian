use super::*;
use crate::{Limits, SourceId, Vertex, integer};

fn source() -> Result<Vec<Facet>, Failure> {
    let mut b = Budget::new(Limits::default())?;
    let mut result = Vec::new();
    for i in 0..512 {
        let place = (i * 73) % 512;
        let x = 10 * (place % 8);
        let y = 10 * ((place / 8) % 8);
        let z = 10 * (place / 64);
        result.push(Facet::new(
            SourceId {
                component: if place % 8 < 4 { 0 } else { 1 },
                facet: i as u32,
            },
            [[x, y, z], [x + 2, y, z], [x, y + 2, z + 1]].map(|p| Vertex {
                point: p.map(integer),
                uv: [integer(0), integer(0)],
            }),
            "test".into(),
            &mut b,
        )?);
    }
    Ok(result)
}

#[test]
fn spatial_bounds_preserve_source_order_owner_proofs_and_charged_total_query_work()
-> Result<(), Failure> {
    let faces = source()?;
    for count in [257, 512] {
        let faces = &faces[..count];
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        old.retain(FacetBoundsIndex::memberships(faces.len()))?;
        new.retain(FacetBoundsIndex::spatial_memberships(faces.len()))?;
        let contiguous = FacetBoundsIndex::build(faces, &mut old)?;
        let spatial = FacetBoundsIndex::build_spatial(faces, &mut new)?;
        assert!(spatial.order.is_some());
        let mut permutation: Vec<_> = (0..faces.len())
            .map(|p| spatial.source_ordinal(p))
            .collect();
        permutation.sort();
        assert_eq!(permutation, (0..faces.len()).collect::<Vec<_>>());
        for node in &spatial.nodes {
            let mut owner = None;
            let mut uniform = true;
            for position in node.start..node.end {
                let face = &faces[spatial.source_ordinal(position)];
                if owner.is_none() {
                    owner = Some(face.id().component);
                } else if owner != Some(face.id().component) {
                    uniform = false;
                }
                for vertex in face.vertices() {
                    assert!(
                        (0..3).all(|axis| node.bounds.lower[axis] <= vertex.point[axis]
                            && vertex.point[axis] <= node.bounds.upper[axis])
                    );
                }
            }
            assert_eq!(node.owner, if uniform { owner } else { None });
        }
        for bounds in &spatial.facets {
            let expected = old.with_retained(faces.len(), |b| contiguous.candidates(bounds, b))?;
            let actual = new.with_retained(faces.len(), |b| spatial.candidates(bounds, b))?;
            assert_eq!(actual, expected);
        }
        for direction in [[1, 0, 0], [1, 2, 3], [-1, -2, -3]] {
            let direction = direction.map(integer);
            for face in faces.iter().step_by(31) {
                let origin = &face.vertices()[0].point;
                let owner = face.id().component;
                let bounds = Bounds {
                    lower: [integer(-1), integer(-1), integer(-1)],
                    upper: [integer(100), integer(100), integer(100)],
                };
                let expected = old.with_retained(faces.len(), |b| {
                    contiguous.ray_intersections(origin, &direction, faces, owner, &bounds, b)
                })?;
                let actual = new.with_retained(faces.len(), |b| {
                    spatial.ray_intersections(origin, &direction, faces, owner, &bounds, b)
                })?;
                assert_eq!(actual, expected);
            }
        }
        // Include both complete builders and every physical/ray query above.
        assert!(new.operations() < old.operations());
        drop(contiguous);
        old.release(FacetBoundsIndex::memberships(faces.len()));
        drop(spatial);
        new.release(FacetBoundsIndex::spatial_memberships(faces.len()));
        new.with_retained(FacetBoundsIndex::spatial_memberships(64), |b| {
            let small = FacetBoundsIndex::build_spatial(&faces[..64], b)?;
            assert!(small.order.is_none());
            Ok(())
        })?;
        assert_eq!(old.retained_memberships(), 0);
        assert_eq!(new.retained_memberships(), 0);
    }
    Ok(())
}

#[test]
fn spatial_joint_sections_keep_all_ordered_constraints_fields_and_parity() -> Result<(), Failure> {
    let faces = source()?;
    let mut setup = Budget::new(Limits::default())?;
    let contiguous = FacetBoundsIndex::build(&faces, &mut setup)?;
    let spatial = FacetBoundsIndex::build_spatial(&faces, &mut setup)?;
    let owners = crate::certificate::component_bounds(&faces, &contiguous.facets, &mut setup)?;
    for ordinal in (0..faces.len()).step_by(47) {
        let face = &faces[ordinal];
        let bounds = &contiguous.facets[ordinal];
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let (segments, fields, frame) = crate::certificate::sections::Frame::collect(
            face,
            bounds,
            &faces,
            &owners,
            &contiguous,
            &mut old,
        )?;
        let (actual, actual_fields, actual_frame) = crate::certificate::sections::Frame::collect(
            face, bounds, &faces, &owners, &spatial, &mut new,
        )?;
        assert_eq!(actual, segments);
        assert_eq!(
            actual_fields.iter().map(|f| f.id()).collect::<Vec<_>>(),
            fields.iter().map(|f| f.id()).collect::<Vec<_>>()
        );
        assert_eq!(actual_frame.memberships(), frame.memberships());
        for p in [[0, 0], [1, 1], [5, 5], [70, 70]] {
            let p = p.map(integer);
            assert_eq!(
                actual_frame.occupied(&p, &mut new)?,
                frame.occupied(&p, &mut old)?
            );
        }
    }
    Ok(())
}

fn admitted(faces: &[Facet], b: &mut Budget) -> Result<(), Failure> {
    b.with_retained(FacetBoundsIndex::spatial_memberships(faces.len()), |b| {
        let index = FacetBoundsIndex::build_spatial(faces, b)?;
        let bounds = Bounds {
            lower: [integer(-1), integer(-1), integer(-1)],
            upper: [integer(100), integer(100), integer(100)],
        };
        let candidates = b.with_retained(faces.len(), |b| index.candidates(&bounds, b))?;
        assert_eq!(candidates, (0..faces.len()).collect::<Vec<_>>());
        Ok(())
    })
}
#[test]
fn spatial_sort_nodes_and_source_order_restore_admit_then_rollback_exactly() -> Result<(), Failure>
{
    let faces = source()?;
    let mut probe = Budget::new(Limits::default())?;
    probe.retain(1000)?;
    admitted(&faces, &mut probe)?;
    let cap = probe.peak_memberships();
    assert_eq!(probe.retained_memberships(), 1000);
    let mut exact = Budget::new(Limits {
        storage_memberships: cap,
        ..Limits::default()
    })?;
    exact.retain(1000)?;
    admitted(&faces, &mut exact)?;
    assert_eq!(exact.peak_admitted_memberships(), cap);
    assert_eq!(exact.retained_memberships(), 1000);
    let mut tight = Budget::new(Limits {
        storage_memberships: cap - 1,
        ..Limits::default()
    })?;
    tight.retain(1000)?;
    assert_eq!(
        admitted(&faces, &mut tight).err().map(|f| f.kind),
        Some(crate::FailureKind::Storage)
    );
    assert_eq!(tight.retained_memberships(), 1000);
    let mut work = Budget::new(Limits {
        operations: 3,
        ..Limits::default()
    })?;
    work.retain(1000)?;
    assert_eq!(
        admitted(&faces, &mut work).err().map(|f| f.kind),
        Some(crate::FailureKind::Operations)
    );
    assert_eq!(work.retained_memberships(), 1000);
    Ok(())
}
