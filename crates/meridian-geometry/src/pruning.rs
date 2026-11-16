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
