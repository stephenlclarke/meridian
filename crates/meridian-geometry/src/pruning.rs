//! Frozen full-construction comparison. Only synthetic analytic solids execute here.
mod before {
    include!("../fixtures/exact-pruning-reference/before-certificate.rs");
}
use crate::{
    AffineTransform, Budget, Facet, Failure, FailureKind, Limits, Point2, SourceId, Vertex, integer,
};

fn cube(bounds: [i64; 6], owner: u32) -> Result<Vec<Facet>, Failure> {
    let [x, y, z, a, c, d] = bounds;
    let points = [
        [x, y, z],
        [a, y, z],
        [a, c, z],
        [x, c, z],
        [x, y, d],
        [a, y, d],
        [a, c, d],
        [x, c, d],
    ]
    .map(|p| p.map(integer));
    let mut b = Budget::new(Limits::default())?;
    [
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
    ]
    .into_iter()
    .enumerate()
    .map(|(i, indices)| {
        Facet::new(
            SourceId {
                component: owner,
                facet: i as u32,
            },
            indices.map(|j| Vertex {
                point: points[j].clone(),
                uv: [
                    &points[j][0] + integer(2) * &points[j][1],
                    &points[j][2] - &points[j][1],
                ],
            }),
            "material0/texture0".into(),
            &mut b,
        )
    })
    .collect()
}
fn reference_stars(stars: &[before::ContactStar]) -> Vec<(Point2, Vec<[Point2; 2]>, bool)> {
    stars
        .iter()
        .map(|s| (s.point.clone(), s.sectors.clone(), s.boundary_path))
        .collect()
}
fn current_stars(stars: &[crate::ContactStar]) -> Vec<(Point2, Vec<[Point2; 2]>, bool)> {
    stars
        .iter()
        .map(|s| (s.point.clone(), s.sectors.clone(), s.boundary_path))
        .collect()
}

#[test]
fn complete_frozen_solids_preserve_all_exposed_fields_witnesses_and_charged_work()
-> Result<(), Failure> {
    let mut cases = vec![
        ("identity", vec![[0, 0, 0, 1, 1, 1]], 1, false, false),
        (
            "overlap",
            vec![[0, 0, 0, 2, 2, 2], [1, 1, 1, 3, 3, 3]],
            15,
            false,
            false,
        ),
        (
            "buried",
            vec![[0, 0, 0, 3, 3, 3], [1, 1, 1, 2, 2, 2]],
            27,
            false,
            false,
        ),
        (
            "shared_face",
            vec![[0, 0, 0, 1, 1, 1], [1, 0, 0, 2, 1, 1]],
            2,
            false,
            false,
        ),
    ];
    cases.push((
        "four_disconnected",
        (0..4).map(|i| [3 * i, 0, 0, 3 * i + 1, 1, 1]).collect(),
        4,
        false,
        true,
    ));
    cases.push((
        "eight_oblique_disconnected",
        (0..8).map(|i| [3 * i, 0, 0, 3 * i + 1, 1, 1]).collect(),
        8,
        true,
        true,
    ));
    for (id, boxes, volume, shear, require_saving) in cases {
        let mut originals = Vec::new();
        for (i, bounds) in boxes.into_iter().enumerate() {
            originals.extend(cube(bounds, i as u32)?);
        }
        if shear {
            let transform = AffineTransform {
                linear: [
                    [integer(1), integer(0), integer(1)],
                    [integer(0), integer(1), integer(0)],
                    [integer(0), integer(0), integer(1)],
                ],
                translation: [integer(0), integer(0), integer(0)],
            };
            let mut setup = Budget::new(Limits::default())?;
            originals = originals
                .iter()
                .map(|f| transform.apply(f, &mut setup))
                .collect::<Result<_, _>>()?;
        }
        let mut old = Budget::new(Limits::default())?;
        let old_hull = before::reconstruct(&originals, &mut old)?;
        let mut new = Budget::new(Limits::default())?;
        let new_hull = crate::reconstruct(&originals, &mut new)?;
        assert_eq!(old_hull.exact_volume(), &integer(volume));
        assert_eq!(new_hull.exact_volume(), old_hull.exact_volume());
        assert_eq!(new_hull.triangles(), old_hull.triangles(), "{id}");
        let visible = old_hull
            .witnesses()
            .iter()
            .filter(|w| !w.classified_cells.is_empty())
            .collect::<Vec<_>>();
        assert_eq!(new_hull.witnesses().len(), visible.len());
        for (actual, expected) in new_hull.witnesses().iter().zip(visible) {
            assert_eq!(actual.source, expected.source);
            assert_eq!(actual.classified_cells, expected.classified_cells);
            assert_eq!(actual.physical_segments, expected.physical_segments);
            assert_eq!(actual.protected_points, expected.protected_points);
            assert_eq!(actual.output_triangles, expected.output_triangles);
            assert_eq!(
                current_stars(&actual.contact_stars),
                reference_stars(&expected.contact_stars)
            );
            assert_eq!(actual.twice_area, expected.twice_area);
        }
        // Recheck both public witness views against the same exact originals.
        // These charges are included in the complete comparison below.
        for witness in old_hull
            .witnesses()
            .iter()
            .filter(|w| !w.classified_cells.is_empty())
        {
            let source = originals
                .iter()
                .find(|f| f.id() == witness.source)
                .ok_or_else(|| Failure {
                    kind: FailureKind::InvalidWitness,
                    source: Some(witness.source),
                    operations: old.operations(),
                    detail: "missing old control source",
                })?;
            witness.verify(source, &mut old)?;
        }
        for witness in new_hull.witnesses() {
            let source = originals
                .iter()
                .find(|f| f.id() == witness.source)
                .ok_or_else(|| Failure {
                    kind: FailureKind::InvalidWitness,
                    source: Some(witness.source),
                    operations: new.operations(),
                    detail: "missing new control source",
                })?;
            witness.verify(source, &mut new)?;
        }
        if require_saving {
            assert!(new.operations() < old.operations(), "{id}");
        }
        if id == "buried" {
            assert!(
                old_hull
                    .witnesses()
                    .iter()
                    .any(|w| w.classified_cells.is_empty())
            );
            assert!(new.retained_memberships() < old.retained_memberships());
        }
        eprintln!(
            "EXACT_PRUNING_COMPLETE {{\"fixture\":\"{id}\",\"oldOperations\":{},\"newOperations\":{},\"oldAttemptedPeak\":{},\"newAttemptedPeak\":{},\"oldRetained\":{},\"newRetained\":{},\"triangles\":{},\"exactOutputAndExposedWitnessMatch\":true}}",
            old.operations(),
            new.operations(),
            old.peak_memberships(),
            new.peak_memberships(),
            old.retained_memberships(),
            new.retained_memberships(),
            new_hull.triangles().len()
        );
    }
    let mut touching = cube([0, 0, 0, 1, 1, 1], 0)?;
    touching.extend(cube([1, 1, 1, 2, 2, 2], 1)?);
    let mut old = Budget::new(Limits::default())?;
    let mut new = Budget::new(Limits::default())?;
    assert_eq!(
        before::reconstruct(&touching, &mut old)
            .err()
            .map(|f| f.kind),
        Some(FailureKind::UncertifiedContact)
    );
    assert_eq!(
        crate::reconstruct(&touching, &mut new)
            .err()
            .map(|f| f.kind),
        Some(FailureKind::UncertifiedContact)
    );
    Ok(())
}

mod before_style_storage {
    include!("../fixtures/style-storage-reference/before-certificate.rs");
}

#[test]
fn complete_style_storage_controls_preserve_all_fields_witnesses_and_refusals()
-> Result<(), Failure> {
    let cases = vec![
        ("identity", vec![[0, 0, 0, 1, 1, 1]], false),
        (
            "overlap",
            vec![[0, 0, 0, 2, 2, 2], [1, 1, 1, 3, 3, 3]],
            false,
        ),
        (
            "buried",
            vec![[0, 0, 0, 3, 3, 3], [1, 1, 1, 2, 2, 2]],
            false,
        ),
        (
            "shared_face",
            vec![[0, 0, 0, 1, 1, 1], [1, 0, 0, 2, 1, 1]],
            false,
        ),
        (
            "four_disconnected",
            (0..4).map(|i| [3 * i, 0, 0, 3 * i + 1, 1, 1]).collect(),
            false,
        ),
        (
            "eight_oblique_disconnected",
            (0..8).map(|i| [3 * i, 0, 0, 3 * i + 1, 1, 1]).collect(),
            true,
        ),
    ];
    for (id, boxes, shear) in cases {
        let mut originals = Vec::new();
        for (i, bounds) in boxes.into_iter().enumerate() {
            originals.extend(cube(bounds, i as u32)?);
        }
        let mut setup = Budget::new(Limits::default())?;
        if shear {
            let transform = AffineTransform {
                linear: [
                    [integer(1), integer(0), integer(1)],
                    [integer(0), integer(1), integer(0)],
                    [integer(0), integer(0), integer(1)],
                ],
                translation: [integer(0), integer(0), integer(0)],
            };
            originals = originals
                .iter()
                .map(|f| transform.apply(f, &mut setup))
                .collect::<Result<_, _>>()?;
        }
        let style: std::sync::Arc<str> = std::sync::Arc::from("material0/texture0");
        originals = originals
            .iter()
            .map(|f| {
                Facet::with_shared_style(f.id(), f.vertices().clone(), style.clone(), &mut setup)
            })
            .collect::<Result<_, _>>()?;
        let mut old = Budget::new(Limits::default())?;
        let expected = before_style_storage::reconstruct(&originals, &mut old)?;
        let mut new = Budget::new(Limits::default())?;
        let actual = crate::reconstruct(&originals, &mut new)?;
        assert_eq!(actual.triangles(), expected.triangles(), "{id}");
        assert_eq!(actual.exact_volume(), expected.exact_volume());
        assert_eq!(actual.witnesses().len(), expected.witnesses().len());
        for (a, e) in actual.witnesses().iter().zip(expected.witnesses()) {
            assert_eq!(a.source, e.source);
            assert_eq!(a.classified_cells, e.classified_cells);
            assert_eq!(a.physical_segments, e.physical_segments);
            assert_eq!(a.protected_points, e.protected_points);
            assert_eq!(a.output_triangles, e.output_triangles);
            assert_eq!(a.twice_area, e.twice_area);
            let stars = e
                .contact_stars
                .iter()
                .map(|s| (s.point.clone(), s.sectors.clone(), s.boundary_path))
                .collect::<Vec<_>>();
            assert_eq!(current_stars(&a.contact_stars), stars);
            let source = originals
                .iter()
                .find(|f| f.id() == a.source)
                .ok_or_else(|| {
                    new.fail(
                        FailureKind::InvalidWitness,
                        "missing storage control source",
                    )
                })?;
            e.verify(source, &mut old)?;
            a.verify(source, &mut new)?;
        }
        assert!(new.peak_memberships() < old.peak_memberships(), "{id}");
        assert_eq!(new.retained_memberships(), old.retained_memberships());
        eprintln!(
            "STYLE_STORAGE_COMPLETE {{\"fixture\":\"{id}\",\"oldOperations\":{},\"newOperations\":{},\"oldAttemptedPeak\":{},\"newAttemptedPeak\":{},\"oldRetained\":{},\"newRetained\":{},\"triangles\":{}}}",
            old.operations(),
            new.operations(),
            old.peak_memberships(),
            new.peak_memberships(),
            old.retained_memberships(),
            new.retained_memberships(),
            actual.triangles().len()
        );
    }
    let mut singular = cube([0, 0, 0, 1, 1, 1], 0)?;
    singular.extend(cube([1, 1, 1, 2, 2, 2], 1)?);
    let mut old = Budget::new(Limits::default())?;
    let mut new = Budget::new(Limits::default())?;
    assert_eq!(
        before_style_storage::reconstruct(&singular, &mut old)
            .err()
            .map(|error| error.kind),
        Some(FailureKind::UncertifiedContact)
    );
    assert_eq!(
        crate::reconstruct(&singular, &mut new)
            .err()
            .map(|error| error.kind),
        Some(FailureKind::UncertifiedContact)
    );
    Ok(())
}

mod before_bounds_cache {
    include!("../fixtures/bounds-cache-reference/before-certificate.rs");
}

#[test]
fn complete_bounds_cache_controls_preserve_fields_witnesses_and_reduce_work() -> Result<(), Failure>
{
    let cases = vec![
        ("identity", vec![[0, 0, 0, 1, 1, 1]], false),
        (
            "overlap",
            vec![[0, 0, 0, 2, 2, 2], [1, 1, 1, 3, 3, 3]],
            false,
        ),
        (
            "buried",
            vec![[0, 0, 0, 3, 3, 3], [1, 1, 1, 2, 2, 2]],
            false,
        ),
        (
            "shared_face",
            vec![[0, 0, 0, 1, 1, 1], [1, 0, 0, 2, 1, 1]],
            false,
        ),
        (
            "four_disconnected",
            (0..4).map(|i| [3 * i, 0, 0, 3 * i + 1, 1, 1]).collect(),
            false,
        ),
        (
            "eight_oblique_disconnected",
            (0..8).map(|i| [3 * i, 0, 0, 3 * i + 1, 1, 1]).collect(),
            true,
        ),
    ];
    for (id, boxes, shear) in cases {
        let mut originals = Vec::new();
        for (i, bounds) in boxes.into_iter().enumerate() {
            originals.extend(cube(bounds, i as u32)?);
        }
        let mut setup = Budget::new(Limits::default())?;
        if shear {
            let transform = AffineTransform {
                linear: [
                    [integer(1), integer(0), integer(1)],
                    [integer(0), integer(1), integer(0)],
                    [integer(0), integer(0), integer(1)],
                ],
                translation: [integer(0), integer(0), integer(0)],
            };
            originals = originals
                .iter()
                .map(|f| transform.apply(f, &mut setup))
                .collect::<Result<_, _>>()?;
        }
        let style: std::sync::Arc<str> = std::sync::Arc::from("material0/texture0");
        originals = originals
            .iter()
            .map(|f| {
                Facet::with_shared_style(f.id(), f.vertices().clone(), style.clone(), &mut setup)
            })
            .collect::<Result<_, _>>()?;
        let mut old = Budget::new(Limits::default())?;
        let expected = before_bounds_cache::reconstruct(&originals, &mut old)?;
        let mut new = Budget::new(Limits::default())?;
        let actual = crate::reconstruct(&originals, &mut new)?;
        assert_eq!(actual.triangles(), expected.triangles(), "{id}");
        assert_eq!(actual.exact_volume(), expected.exact_volume());
        assert_eq!(actual.witnesses().len(), expected.witnesses().len());
        for (a, e) in actual.witnesses().iter().zip(expected.witnesses()) {
            assert_eq!(a.source, e.source);
            assert_eq!(a.classified_cells, e.classified_cells);
            assert_eq!(a.physical_segments, e.physical_segments);
            assert_eq!(a.protected_points, e.protected_points);
            assert_eq!(a.output_triangles, e.output_triangles);
            assert_eq!(a.twice_area, e.twice_area);
            let stars = e
                .contact_stars
                .iter()
                .map(|s| (s.point.clone(), s.sectors.clone(), s.boundary_path))
                .collect::<Vec<_>>();
            assert_eq!(current_stars(&a.contact_stars), stars);
            let source = originals
                .iter()
                .find(|f| f.id() == a.source)
                .ok_or_else(|| {
                    new.fail(
                        FailureKind::InvalidWitness,
                        "missing bounds-cache control source",
                    )
                })?;
            e.verify(source, &mut old)?;
            a.verify(source, &mut new)?;
        }
        assert!(new.operations() < old.operations(), "{id}");
        assert_eq!(new.retained_memberships(), old.retained_memberships());
        eprintln!(
            "BOUNDS_CACHE_COMPLETE {{\"fixture\":\"{id}\",\"oldOperations\":{},\"newOperations\":{},\"oldAttemptedPeak\":{},\"newAttemptedPeak\":{},\"oldRetained\":{},\"newRetained\":{},\"triangles\":{}}}",
            old.operations(),
            new.operations(),
            old.peak_memberships(),
            new.peak_memberships(),
            old.retained_memberships(),
            new.retained_memberships(),
            actual.triangles().len()
        );
    }
    let mut singular = cube([0, 0, 0, 1, 1, 1], 0)?;
    singular.extend(cube([1, 1, 1, 2, 2, 2], 1)?);
    let mut old = Budget::new(Limits::default())?;
    let mut new = Budget::new(Limits::default())?;
    assert_eq!(
        before_bounds_cache::reconstruct(&singular, &mut old)
            .err()
            .map(|error| error.kind),
        Some(FailureKind::UncertifiedContact)
    );
    assert_eq!(
        crate::reconstruct(&singular, &mut new)
            .err()
            .map(|error| error.kind),
        Some(FailureKind::UncertifiedContact)
    );
    Ok(())
}
