use super::*;
use crate::Limits;
use crate::certificate::*;
use std::time::Instant;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/exact-leaf-reference/compiled-publisher-control.rs"
));

#[path = "../fixtures/exact-leaf-reference/before-vertex_distances.rs"]
mod parent_distance_cache;

fn checked_failure<T>(result: Result<T, Failure>) -> Failure {
    match result {
        Err(failure) => failure,
        Ok(_) => panic!("unexpected success in vertex-distance refusal test"),
    }
}
fn labelled<T>(
    fixture: &str,
    lane: &str,
    run: impl FnOnce() -> Result<T, Failure>,
) -> Result<T, Failure> {
    match run() {
        Err(failure) => {
            eprintln!(
                "VERTEX_DISTANCE_FAILURE fixture={fixture} lane={lane} kind={:?} source={:?} operations={} detail={}",
                failure.kind, failure.source, failure.operations, failure.detail
            );
            Err(failure)
        }
        Ok(value) => Ok(value),
    }
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

fn query(i: usize) -> Result<Facet, Failure> {
    let z = (i % 5) as i64 - 2;
    let mut vertices = [[0, 0, z], [2 + (i % 3) as i64, 0, z], [0, 2, z]].map(|p| Vertex {
        point: p.map(integer),
        uv: [integer(0), integer(0)],
    });
    if i % 2 == 1 {
        vertices.swap(1, 2);
    }
    let mut b = Budget::new(Limits::default())?;
    Facet::new(
        SourceId {
            component: 99,
            facet: i as u32,
        },
        vertices,
        "query".into(),
        &mut b,
    )
}
fn drop_cache(cache: VertexDistances<'_>, b: &mut Budget) {
    let members = cache.memberships();
    drop(cache);
    b.release(members);
}

fn oblique_query(reverse: bool) -> Result<Facet, Failure> {
    let mut b = Budget::new(Limits::default())?;
    let mut vertices = [[0, 0, 0], [0, 0, 2], [2, -2, 0]].map(|p| Vertex {
        point: p.map(integer),
        uv: [integer(0), integer(0)],
    });
    if reverse {
        vertices.swap(1, 2);
    }
    Facet::new(
        SourceId {
            component: 99,
            facet: u32::from(reverse),
        },
        vertices,
        "query".into(),
        &mut b,
    )
}

#[test]
fn exact_leaf_eligibility_preserves_joint_results_and_reduces_complete_batch_work()
-> Result<(), Failure> {
    labelled("oblique-triangle-support", "case-return", || {
        let mut setup = Budget::new(Limits::default())?;
        let mut originals = Vec::new();
        for i in 0..256u32 {
            originals.push(Facet::new(
                SourceId {
                    component: 0,
                    facet: i,
                },
                [[-4, 5, 0], [5, -4, 0], [6, 6, 1]].map(|p| Vertex {
                    point: p.map(integer),
                    uv: [integer(i64::from(i)), integer(0)],
                }),
                "oblique".into(),
                &mut setup,
            )?);
        }
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let index_members = FacetBoundsIndex::memberships(originals.len());
        let start = Instant::now();
        let baseline = Facet::input_memberships(&originals, &mut old)? + index_members + 512;
        old.retain(baseline)?;
        let old_index = FacetBoundsIndex::build(&originals, &mut old)?;
        let mut parent_cache = parent_distance_cache::VertexDistances::build(&originals, &mut old)?
            .ok_or_else(|| old.fail(FailureKind::InvalidWitness, "missing parent cache"))?;
        let mut old_elapsed = start.elapsed();
        assert_eq!(parent_cache.originals().len(), originals.len());
        let start = Instant::now();
        let new_baseline = Facet::input_memberships(&originals, &mut new)? + index_members + 512;
        new.retain(new_baseline)?;
        let new_index = FacetBoundsIndex::build(&originals, &mut new)?;
        let mut cache = VertexDistances::build(&originals, &mut new)?
            .ok_or_else(|| new.fail(FailureKind::InvalidWitness, "missing cache"))?;
        let mut new_elapsed = start.elapsed();
        assert_eq!(new_baseline, baseline);
        let owners = BTreeMap::from([(0, Vec::new())]);
        for i in 0..32 {
            let face = oblique_query(i % 2 == 1)?;
            let start = Instant::now();
            let old_finite = Bounds::for_facet(&face, &mut old)?;
            let mut parent_plane = parent_cache.plane(&face, &mut old)?;
            old.retain(3 * originals.len())?;
            let expected = old_index.collection_candidates(
                &face,
                &old_finite,
                &owners,
                &originals,
                &mut old,
            )?;
            old_elapsed += start.elapsed();
            let start = Instant::now();
            let finite = Bounds::for_facet(&face, &mut new)?;
            let mut plane = cache.plane(&face, &mut new)?;
            new.retain(3 * originals.len())?;
            let actual = new_index
                .collection_cached(&face, &finite, &owners, &originals, &mut plane, &mut new)?;
            new_elapsed += start.elapsed();
            assert_eq!(expected.len(), 256);
            assert_eq!(actual.len(), 256);
            for ((ordinal, physical, outward), (id, kept_physical, exact_outward)) in
                expected.into_iter().zip(actual)
            {
                assert_eq!((id, kept_physical), (ordinal, physical));
                assert!(outward);
                assert!(!exact_outward);
                let start = Instant::now();
                old.step(1)?;
                let expected =
                    old.with_retained(96, |b| parent_plane.section(ordinal, physical, outward, b))?;
                old_elapsed += start.elapsed();
                let start = Instant::now();
                new.step(1)?;
                let actual =
                    new.with_retained(96, |b| plane.section(id, kept_physical, exact_outward, b))?;
                new_elapsed += start.elapsed();
                assert_eq!(actual, expected);
                // Separate uncached semantic oracle; excluded from both work/timing lanes.
                let mut oracle = Budget::new(Limits::default())?;
                let uncached = oracle.with_retained(96, |b| {
                    joint_section_source(&face, &originals[ordinal], physical, outward, b)
                })?;
                assert_eq!(expected, uncached);
            }
            let start = Instant::now();
            old.release(3 * originals.len());
            old_elapsed += start.elapsed();
            let start = Instant::now();
            new.release(3 * originals.len());
            new_elapsed += start.elapsed();
        }
        let start = Instant::now();
        let parent_members = parent_cache.memberships();
        drop(parent_cache);
        drop(old_index);
        old.release(parent_members + index_members);
        old_elapsed += start.elapsed();
        let start = Instant::now();
        drop_cache(cache, &mut new);
        drop(new_index);
        new.release(index_members);
        new_elapsed += start.elapsed();
        assert!(new.operations() < old.operations());
        eprintln!(
            "EXACT_LEAF_COMPLETE_BATCH fixture=oblique-triangle-support reference=independent-frozen-parent-cache scope=input-index-cache-generations-ordering-queries-sections-cleanup oldOperations={} newOperations={} oldPeak={} newPeak={} oldWallNanoseconds={} newWallNanoseconds={}",
            old.operations(),
            new.operations(),
            old.peak_memberships(),
            new.peak_memberships(),
            old_elapsed.as_nanos(),
            new_elapsed.as_nanos()
        );
        assert!(new_elapsed < old_elapsed.saturating_mul(10));
        assert_eq!(old.retained_memberships(), baseline - index_members);
        assert_eq!(new.retained_memberships(), baseline - index_members);
        Ok(())
    })
}

#[test]
fn exact_leaf_sign_filter_keeps_zero_contact_multiplicity_and_failure_guards() -> Result<(), Failure>
{
    labelled("all-signed-triangle-support", "case-return", || {
        let face = query(2)?;
        let mut setup = Budget::new(Limits::default())?;
        let mut originals = Vec::new();
        for a in [-1, 0, 1] {
            for c in [-1, 0, 1] {
                for d in [-1, 0, 1] {
                    for _ in 0..10 {
                        let id = originals.len() as u32;
                        originals.push(Facet::new(
                            SourceId {
                                component: 0,
                                facet: id,
                            },
                            [[0, 0, a], [2, 0, c], [0, 2, d]].map(|p| Vertex {
                                point: p.map(integer),
                                uv: [integer(0), integer(0)],
                            }),
                            "sign".into(),
                            &mut setup,
                        )?);
                    }
                }
            }
        }
        let mut cache = VertexDistances::build(&originals, &mut setup)?
            .ok_or_else(|| setup.fail(FailureKind::InvalidWitness, "missing sign cache"))?;
        let mut b = Budget::new(Limits::default())?;
        b.retain(setup.retained_memberships() + 512)?;
        let mut plane = cache.plane(&face, &mut b)?;
        for (ordinal, other) in originals.iter().enumerate() {
            let signs = other.vertices().each_ref().map(|v| face.distance(&v.point));
            let expected =
                signs.iter().any(|v| v > &integer(0)) && signs.iter().any(|v| v <= &integer(0));
            b.step(1)?;
            let actual = b.with_retained(96, |b| plane.outward(ordinal, b))?;
            assert_eq!(actual, expected);
            let mut old = Budget::new(Limits::default())?;
            let reference =
                old.with_retained(96, |b| joint_section_source(&face, other, false, true, b))?;
            let candidate = b.with_retained(96, |b| plane.section(ordinal, false, actual, b))?;
            assert_eq!(candidate, reference);
            if signs.iter().filter(|v| v.is_zero()).count() == 1
                && signs.iter().filter(|v| v.is_positive()).count() == 2
            {
                let segment = candidate
                    .2
                    .ok_or_else(|| b.fail(FailureKind::InvalidWitness, "zero contact lost"))?;
                assert_eq!(segment[0], segment[1]);
            }
        }
        assert_eq!(
            checked_failure(plane.outward(originals.len(), &mut b)).kind,
            FailureKind::InvalidWitness
        );
        // A fresh generation must pay the original word-reset charge before mutation.
        let mut short = Budget::new(Limits {
            operations: 1,
            ..Limits::default()
        })?;
        short.retain(setup.retained_memberships() + 512)?;
        let retained = short.retained_memberships();
        let mut plane = cache.plane(&face, &mut short)?;
        assert_eq!(
            checked_failure(short.with_retained(96, |b| plane.outward(0, b))).kind,
            FailureKind::Operations
        );
        assert_eq!(short.retained_memberships(), retained);
        let index_members = FacetBoundsIndex::memberships(originals.len());
        setup.retain(index_members)?;
        let index = FacetBoundsIndex::build(&originals, &mut setup)?;
        let finite = Bounds::for_facet(&face, &mut setup)?;
        let owners = BTreeMap::from([(0, Vec::new())]);
        let baseline = setup.retained_memberships() + 512;
        let mut tight = Budget::new(Limits {
            storage_memberships: baseline
                + 3 * originals.len()
                + FacetBoundsIndex::stack_members(originals.len())
                + 16
                + 95,
            ..Limits::default()
        })?;
        tight.retain(baseline)?;
        let mut plane = cache.plane(&face, &mut tight)?;
        let failure = checked_failure(tight.with_retained(3 * originals.len(), |b| {
            index.collection_cached(&face, &finite, &owners, &originals, &mut plane, b)
        }));
        assert_eq!(failure.kind, FailureKind::Storage);
        assert_eq!(tight.retained_memberships(), baseline);
        drop(index);
        setup.release(index_members);
        drop_cache(cache, &mut setup);
        assert_eq!(setup.retained_memberships(), 0);
        Ok(())
    })
}

#[test]
fn exact_leaf_complete_publisher_matches_parent_fields_witnesses_and_volume() -> Result<(), Failure>
{
    labelled("four-original-tetrahedra", "case-return", || {
        let mut originals = Vec::new();
        let mut build = Budget::new(Limits::default())?;
        for owner in 0..4u32 {
            for face in mesh(
                &[[0, 0, 0], [2, 0, 0], [0, 2, 0], [0, 0, 2]],
                &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
                owner,
                2,
            )? {
                let mut vertices = face.vertices().clone();
                for v in &mut vertices {
                    v.point[0] += integer(i64::from(4 * owner));
                    v.uv[0] += integer(i64::from(4 * owner));
                }
                originals.push(Facet::new(
                    face.id(),
                    vertices,
                    format!("owner-{owner}"),
                    &mut build,
                )?);
            }
        }
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let start = Instant::now();
        let expected = labelled("four-original-tetrahedra", "parent-publisher", || {
            reference_exact_leaf_publisher(&originals, &mut old)
        })?;
        let old_elapsed = start.elapsed();
        let start = Instant::now();
        let actual = labelled("four-original-tetrahedra", "exact-leaf-publisher", || {
            reconstruct(&originals, &mut new)
        })?;
        let new_elapsed = start.elapsed();
        assert_eq!(actual.triangles(), expected.triangles());
        assert_eq!(actual.witnesses(), expected.witnesses());
        assert_eq!(actual.exact_volume(), expected.exact_volume());
        assert_eq!(actual.exact_volume(), &(integer(16) / integer(3)));
        assert_eq!(new.retained_memberships(), old.retained_memberships());
        eprintln!(
            "EXACT_LEAF_COMPLETE_PUBLISHER fixture=four-original-tetrahedra oldOperations={} newOperations={} oldPeak={} newPeak={} oldWallNanoseconds={} newWallNanoseconds={}",
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

fn shell_cube(lo: i64, hi: i64) -> Result<Vec<Facet>, Failure> {
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
        0,
        0,
    )
}

#[test]
fn exact_leaf_uncertified_cavity_publisher_exercises_joint_dispatch_and_preserves_all_outputs()
-> Result<(), Failure> {
    labelled(
        "closed-cavity-plus-four-convex-owners",
        "case-return",
        || {
            let mut originals = shell_cube(0, 8)?;
            let mut build = Budget::new(Limits::default())?;
            for face in shell_cube(2, 6)? {
                let mut vertices = face.vertices().clone();
                vertices.swap(1, 2);
                originals.push(Facet::new(
                    SourceId {
                        component: 0,
                        facet: 12 + face.id().facet,
                    },
                    vertices,
                    "test".into(),
                    &mut build,
                )?);
            }
            for owner in 1..=4u32 {
                for face in mesh(
                    &[[0, 0, 0], [2, 0, 0], [0, 2, 0], [0, 0, 2]],
                    &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
                    owner,
                    2,
                )? {
                    let mut vertices = face.vertices().clone();
                    let shift = integer(i64::from(8 + 4 * owner));
                    for vertex in &mut vertices {
                        vertex.point[0] += &shift;
                        vertex.uv[0] += &shift;
                    }
                    originals.push(Facet::new(
                        face.id(),
                        vertices,
                        format!("owner-{owner}"),
                        &mut build,
                    )?);
                }
            }
            assert_eq!(originals.len(), 280);
            assert!(originals.iter().any(|f| {
                f.vertices()
                    .iter()
                    .any(|v| v.uv.iter().any(|q| !q.is_zero()))
            }));
            // Establish closed source validity and the exact production branch precondition.
            let mut precondition = Budget::new(Limits::default())?;
            let owner_ids = originals
                .iter()
                .map(|f| f.id().component)
                .collect::<BTreeSet<_>>();
            audit_original_owner_ranges(&originals, &owner_ids, &mut precondition)?;
            audit_original_sheets(&originals, &mut precondition)?;
            let index = FacetBoundsIndex::build(&originals, &mut precondition)?;
            let owners = component_bounds(&originals, &index.facets, &mut precondition)?;
            let certificates = strict_interior::InteriorCertificates::build_sections(
                &originals,
                &owners,
                &mut precondition,
            )?;
            let certificates_ref = certificates.as_ref().ok_or_else(|| {
                precondition.fail(
                    FailureKind::InvalidWitness,
                    "convex controls did not certify",
                )
            })?;
            assert_eq!(
                certificates_ref
                    .section_owners()
                    .map(|(owner, _, _)| owner)
                    .collect::<Vec<_>>(),
                vec![1, 2, 3, 4]
            );
            let face = &originals[0];
            let finite = &index.facets[0];
            let rows = convex_sections::Rows::build(
                face,
                finite,
                Some(certificates_ref),
                &mut precondition,
            )?;
            let cavity = owners.get(&0).ok_or_else(|| {
                precondition.fail(FailureKind::InvalidWitness, "missing cavity owner")
            })?;
            assert!(finite.meets(&cavity.bounds, &mut precondition)?);
            assert!(!rows.contains(0, &mut precondition)?);
            // collect_cached_admitted inserts owner 0 under exactly these conditions;
            // its owner map is nonempty and the production collection_cached branch is required.
            let row_members = rows.memberships();
            drop(rows);
            precondition.release(row_members);
            let cert_members = certificates_ref.memberships();
            drop(certificates);
            precondition.release(cert_members);
            assert_eq!(precondition.retained_memberships(), 0);
            let mut old = Budget::new(Limits::default())?;
            let mut new = Budget::new(Limits::default())?;
            let start = Instant::now();
            let expected = labelled(
                "closed-cavity-plus-four-convex-owners",
                "parent-publisher",
                || reference_exact_leaf_publisher(&originals, &mut old),
            )?;
            let old_elapsed = start.elapsed();
            let start = Instant::now();
            let actual = labelled(
                "closed-cavity-plus-four-convex-owners",
                "exact-leaf-publisher",
                || reconstruct(&originals, &mut new),
            )?;
            let new_elapsed = start.elapsed();
            assert_eq!(actual.triangles(), expected.triangles());
            assert_eq!(actual.witnesses(), expected.witnesses());
            assert_eq!(actual.exact_volume(), expected.exact_volume());
            assert_eq!(actual.exact_volume(), &(integer(1360) / integer(3)));
            assert_eq!(new.retained_memberships(), old.retained_memberships());
            for triangle in actual.triangles() {
                for vertex in triangle.vertices() {
                    assert_eq!(
                        vertex.uv,
                        [vertex.point[0].clone(), vertex.point[1].clone()]
                    );
                }
            }
            eprintln!(
                "EXACT_LEAF_COMPLETE_PUBLISHER fixture=closed-cavity-plus-four-convex-owners branch=uncertified-owner-joint-collection oldOperations={} newOperations={} oldPeak={} newPeak={} oldWallNanoseconds={} newWallNanoseconds={}",
                old.operations(),
                new.operations(),
                old.peak_memberships(),
                new.peak_memberships(),
                old_elapsed.as_nanos(),
                new_elapsed.as_nanos()
            );
            assert!(new_elapsed < old_elapsed.saturating_mul(10));
            Ok(())
        },
    )
}
