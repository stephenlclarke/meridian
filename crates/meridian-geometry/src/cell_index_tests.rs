use super::*;
use crate::Limits;
use std::time::Instant;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/cell-index-reference/compiled-publisher-control.rs"
));

fn checked_failure<T>(result: Result<T, Failure>) -> Failure {
    match result {
        Err(failure) => failure,
        Ok(_) => panic!("unexpected success in cell index refusal test"),
    }
}
fn case_result<T>(
    fixture: &str,
    lane: &str,
    run: impl FnOnce() -> Result<T, Failure>,
) -> Result<T, Failure> {
    match run() {
        Err(failure) => {
            eprintln!(
                "CELL_INDEX_FAILURE fixture={fixture} lane={lane} kind={:?} source={:?} detail={}",
                failure.kind, failure.source, failure.detail
            );
            Err(failure)
        }
        Ok(value) => Ok(value),
    }
}
fn scattered() -> Result<Vec<Facet>, Failure> {
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
                    uv: [point[0].clone(), point[1].clone()],
                    point,
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

#[test]
fn cell_grouping_preserves_all_bounds_owner_tags_source_order_and_complete_index_query_work()
-> Result<(), Failure> {
    let faces = scattered()?;
    for count in [255, 256, 257, 512] {
        let faces = &faces[..count];
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let start = Instant::now();
        let inputs = Facet::input_memberships(faces, &mut old)?;
        old.retain(inputs + FacetBoundsIndex::memberships(count))?;
        let before = FacetBoundsIndex::build(faces, &mut old)?;
        let old_owners = component_bounds(faces, &before.facets, &mut old)?;
        let mut old_elapsed = start.elapsed();
        let start = Instant::now();
        let inputs = Facet::input_memberships(faces, &mut new)?;
        new.retain(inputs + FacetBoundsIndex::cell_memberships(count))?;
        let after = FacetBoundsIndex::build_cells(faces, &mut new)?;
        let new_owners = component_bounds(faces, &after.facets, &mut new)?;
        let mut new_elapsed = start.elapsed();
        assert_eq!(
            after.order.as_ref().map(Vec::len),
            if count < 256 { None } else { Some(count + 1) }
        );
        let mut permutation: Vec<_> = (0..count).map(|i| after.source_ordinal(i)).collect();
        permutation.sort();
        assert_eq!(permutation, (0..count).collect::<Vec<_>>());
        for node in &after.nodes {
            let owner = faces[after.source_ordinal(node.start)].id().component;
            let mut uniform = true;
            for position in node.start..node.end {
                let face = &faces[after.source_ordinal(position)];
                uniform &= face.id().component == owner;
                for vertex in face.vertices() {
                    assert!(
                        (0..3).all(|axis| node.bounds.lower[axis] <= vertex.point[axis]
                            && vertex.point[axis] <= node.bounds.upper[axis])
                    );
                }
            }
            assert_eq!(node.owner, uniform.then_some(owner));
        }
        for bounds in &after.facets {
            let start = Instant::now();
            let expected = old.with_retained(count, |b| before.candidates(bounds, b))?;
            old_elapsed += start.elapsed();
            let start = Instant::now();
            let actual = new.with_retained(count, |b| after.candidates(bounds, b))?;
            new_elapsed += start.elapsed();
            assert_eq!(actual, expected);
        }
        for direction in [[1, 0, 0], [1, 2, 3], [-1, -2, -3]] {
            let direction = direction.map(integer);
            for face in faces.iter().step_by(64) {
                let owner = face.id().component;
                let bounds = &old_owners[&owner].bounds;
                let origin = &face.vertices()[0].point;
                let start = Instant::now();
                let expected = old.with_retained(count, |b| {
                    before.ray_intersections(origin, &direction, faces, owner, bounds, b)
                })?;
                old_elapsed += start.elapsed();
                let start = Instant::now();
                let actual = new.with_retained(count, |b| {
                    after.ray_intersections(origin, &direction, faces, owner, bounds, b)
                })?;
                new_elapsed += start.elapsed();
                assert_eq!(actual, expected);
            }
        }
        for face in faces.iter().step_by(64) {
            let ordinal = face.id().facet as usize;
            let start = Instant::now();
            let (a, ac, af) = sections::Frame::collect(
                face,
                &before.facets[ordinal],
                faces,
                &old_owners,
                &before,
                &mut old,
            )?;
            old_elapsed += start.elapsed();
            let start = Instant::now();
            let (c, cc, cf) = sections::Frame::collect(
                face,
                &after.facets[ordinal],
                faces,
                &new_owners,
                &after,
                &mut new,
            )?;
            new_elapsed += start.elapsed();
            assert_eq!(a, c);
            assert_eq!(
                ac.iter().map(|f| f.id()).collect::<Vec<_>>(),
                cc.iter().map(|f| f.id()).collect::<Vec<_>>()
            );
            assert_eq!(af.memberships(), cf.memberships());
            for sample in [[integer(1), integer(1)], [integer(20), integer(30)]] {
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
        if count >= 256 {
            assert!(new.operations() < old.operations());
        } else {
            assert_eq!(new.operations(), old.operations());
        }
        eprintln!(
            "CELL_INDEX_COMPLETE_QUERY fixture={count} scope=complete-index-build-and-queries oldOperations={} newOperations={} oldWallNanoseconds={} newWallNanoseconds={}",
            old.operations(),
            new.operations(),
            old_elapsed.as_nanos(),
            new_elapsed.as_nanos()
        );
        assert!(new_elapsed < old_elapsed.saturating_mul(10));
        drop(before);
        old.release(inputs + FacetBoundsIndex::memberships(count));
        drop(after);
        new.release(inputs + FacetBoundsIndex::cell_memberships(count));
        assert_eq!(old.retained_memberships(), 0);
        assert_eq!(new.retained_memberships(), 0);
    }
    Ok(())
}

#[test]
fn cell_index_complete_publisher_keeps_every_original_field_witness_and_volume()
-> Result<(), Failure> {
    case_result("separated-22-boxes", "case-return", || {
        let mut originals = Vec::new();
        let mut build = Budget::new(Limits::default())?;
        for owner in 0..22u32 {
            let place = (owner * 7) % 22;
            let offset = [8 * (place % 4), 8 * ((place / 4) % 3), 8 * (place / 12)];
            for face in cube(0, 2, owner, 0)? {
                let mut vertices = face.vertices().clone();
                for vertex in &mut vertices {
                    for (axis, value) in offset.iter().enumerate() {
                        vertex.point[axis] += integer(i64::from(*value));
                    }
                    vertex.uv[0] += integer(i64::from(offset[0]));
                    vertex.uv[1] += integer(i64::from(offset[1]));
                }
                originals.push(Facet::new(
                    face.id(),
                    vertices,
                    face.style().to_owned(),
                    &mut build,
                )?);
            }
        }
        assert_eq!(originals.len(), 264);
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let start = Instant::now();
        let expected = case_result("separated-22-boxes", "reference-publisher", || {
            reference_cell_index_publisher(&originals, &mut old)
        })?;
        let old_elapsed = start.elapsed();
        let start = Instant::now();
        let actual = case_result("separated-22-boxes", "candidate-publisher", || {
            reconstruct(&originals, &mut new)
        })?;
        let new_elapsed = start.elapsed();
        assert_eq!(actual.triangles(), expected.triangles());
        assert_eq!(actual.witnesses(), expected.witnesses());
        assert_eq!(actual.exact_volume(), expected.exact_volume());
        assert_eq!(actual.exact_volume(), &integer(176));
        assert_eq!(new.retained_memberships(), old.retained_memberships());
        eprintln!(
            "CELL_INDEX_COMPLETE_PUBLISHER fixture=separated-22-boxes oldOperations={} newOperations={} oldPeak={} newPeak={} oldWallNanoseconds={} newWallNanoseconds={}",
            old.operations(),
            new.operations(),
            old.peak_memberships(),
            new.peak_memberships(),
            old_elapsed.as_nanos(),
            new_elapsed.as_nanos()
        );
        assert!(new_elapsed < old_elapsed.saturating_mul(10));
        Ok(())
    })
}

#[test]
fn cell_rank_cycles_preserve_complete_payload_and_admit_exact_temporary_storage()
-> Result<(), Failure> {
    let faces = scattered()?;
    let mut setup = Budget::new(Limits::default())?;
    let index = FacetBoundsIndex::build_cells(&faces, &mut setup)?;
    let records = vec![
        (511, true, false),
        (0, false, true),
        (64, true, true),
        (63, false, false),
        (127, false, true),
    ];
    let expected = vec![
        (0, false, true),
        (63, false, false),
        (64, true, true),
        (127, false, true),
        (511, true, false),
    ];
    let baseline = Facet::input_memberships(&faces, &mut setup)?
        + FacetBoundsIndex::cell_memberships(faces.len())
        + 3 * records.len()
        + 64;
    let mut measured = Budget::new(Limits::default())?;
    measured.retain(baseline)?;
    assert_eq!(
        index.source_order(records.clone(), 3, |r| r.0, &mut measured)?,
        expected
    );
    assert_eq!(measured.retained_memberships(), baseline);
    let cap = measured.peak_memberships();
    let mut exact = Budget::new(Limits {
        storage_memberships: cap,
        ..Limits::default()
    })?;
    exact.retain(baseline)?;
    assert_eq!(
        index.source_order(records.clone(), 3, |r| r.0, &mut exact)?,
        expected
    );
    assert_eq!(exact.retained_memberships(), baseline);
    let mut tight = Budget::new(Limits {
        storage_memberships: cap - 1,
        ..Limits::default()
    })?;
    tight.retain(baseline)?;
    assert_eq!(
        checked_failure(index.source_order(records.clone(), 3, |r| r.0, &mut tight)).kind,
        FailureKind::Storage
    );
    assert_eq!(tight.retained_memberships(), baseline);
    let mut short = Budget::new(Limits {
        operations: 1,
        ..Limits::default()
    })?;
    short.retain(baseline)?;
    assert_eq!(
        checked_failure(index.source_order(records, 3, |r| r.0, &mut short)).kind,
        FailureKind::Operations
    );
    assert_eq!(short.retained_memberships(), baseline);
    for invalid in [vec![0usize, 0], vec![0, 512], vec![512]] {
        let mut b = Budget::new(Limits::default())?;
        b.retain(baseline)?;
        assert_eq!(
            checked_failure(index.source_order(invalid, 1, |r| *r, &mut b)).kind,
            FailureKind::InvalidWitness
        );
        assert_eq!(b.retained_memberships(), baseline);
    }
    Ok(())
}

#[test]
fn cell_builder_keeps_exact_storage_operation_and_degenerate_extent_guards() -> Result<(), Failure>
{
    let faces = scattered()?;
    let faces = &faces[..257];
    let mut setup = Budget::new(Limits::default())?;
    let baseline = Facet::input_memberships(faces, &mut setup)?
        + FacetBoundsIndex::cell_memberships(faces.len());
    let mut measured = Budget::new(Limits::default())?;
    measured.retain(baseline)?;
    let index = FacetBoundsIndex::build_cells(faces, &mut measured)?;
    let cap = measured.peak_memberships();
    drop(index);
    assert_eq!(measured.retained_memberships(), baseline);
    let mut exact = Budget::new(Limits {
        storage_memberships: cap,
        ..Limits::default()
    })?;
    exact.retain(baseline)?;
    let index = FacetBoundsIndex::build_cells(faces, &mut exact)?;
    drop(index);
    assert_eq!(exact.retained_memberships(), baseline);
    let mut tight = Budget::new(Limits {
        storage_memberships: cap - 1,
        ..Limits::default()
    })?;
    tight.retain(baseline)?;
    assert_eq!(
        checked_failure(FacetBoundsIndex::build_cells(faces, &mut tight)).kind,
        FailureKind::Storage
    );
    assert_eq!(tight.retained_memberships(), baseline);
    let mut short = Budget::new(Limits {
        operations: 1,
        ..Limits::default()
    })?;
    short.retain(baseline)?;
    assert_eq!(
        checked_failure(FacetBoundsIndex::build_cells(faces, &mut short)).kind,
        FailureKind::Operations
    );
    assert_eq!(short.retained_memberships(), baseline);
    let flat = mesh(&[[0, 0, 0], [2, 0, 0], [0, 2, 0]], &[[0, 1, 2]; 256], 0, 0)?;
    let mut b = Budget::new(Limits::default())?;
    let reserve =
        Facet::input_memberships(&flat, &mut b)? + FacetBoundsIndex::cell_memberships(flat.len());
    b.retain(reserve)?;
    let index = FacetBoundsIndex::build_cells(&flat, &mut b)?;
    let query = &index.facets[0];
    let all = b.with_retained(flat.len(), |b| index.candidates(query, b))?;
    assert_eq!(all, (0..flat.len()).collect::<Vec<_>>());
    drop(index);
    b.release(reserve);
    assert_eq!(b.retained_memberships(), 0);
    Ok(())
}
