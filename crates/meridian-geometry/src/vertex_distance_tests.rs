use super::*;
use crate::Limits;
use std::time::Instant;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/vertex-distance-reference/compiled-publisher-control.rs"
));

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

fn repeated(count: usize, large_zero: bool) -> Result<Vec<Facet>, Failure> {
    let mut b = Budget::new(Limits::default())?;
    (0..count)
        .map(|i| {
            Facet::new(
                SourceId {
                    component: (i % 2) as u32,
                    facet: i as u32,
                },
                if large_zero {
                    [[32, 0, 0], [2, 0, 1], [0, 2, 1]]
                } else {
                    [[0, 0, -1], [2, 0, 0], [0, 2, 1]]
                }
                .map(|p| Vertex {
                    point: p.map(integer),
                    uv: [integer(i as i64), integer(2 * i as i64)],
                }),
                "cache-test".into(),
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
fn drop_cache(cache: sections::VertexDistances<'_>, b: &mut Budget) {
    let members = cache.memberships();
    drop(cache);
    b.release(members);
}

#[test]
fn vertex_distance_cache_matches_complete_section_queries_and_charges_build_and_generations()
-> Result<(), Failure> {
    labelled("repeated-original-points", "case-return", || {
        let originals = repeated(512, false)?;
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let start = Instant::now();
        let inputs = Facet::input_memberships(&originals, &mut old)?;
        old.retain(inputs + 256)?;
        let mut old_elapsed = start.elapsed();
        let start = Instant::now();
        let inputs = Facet::input_memberships(&originals, &mut new)?;
        new.retain(inputs + 256)?;
        let mut cache =
            sections::VertexDistances::build(&originals, &mut new)?.ok_or_else(|| {
                new.fail(
                    FailureKind::InvalidWitness,
                    "missing large-input distance cache",
                )
            })?;
        let mut new_elapsed = start.elapsed();
        for i in 0..32 {
            let face = query(i)?;
            let start = Instant::now();
            let mut plane = cache.plane(&face, &mut new)?;
            new_elapsed += start.elapsed();
            for (ordinal, other) in originals.iter().enumerate() {
                let start = Instant::now();
                old.step(1)?;
                let expected = labelled("repeated-original-points", "reference-joint", || {
                    old.with_retained(96, |b| {
                        sections::joint_section_source(&face, other, true, true, b)
                    })
                })?;
                old_elapsed += start.elapsed();
                let start = Instant::now();
                new.step(1)?;
                let actual = labelled("repeated-original-points", "cached-joint", || {
                    new.with_retained(96, |b| plane.section(ordinal, true, true, b))
                })?;
                new_elapsed += start.elapsed();
                assert_eq!(actual, expected, "query={i} ordinal={ordinal}");
            }
        }
        assert!(new.operations() < old.operations());
        eprintln!(
            "VERTEX_DISTANCE_COMPLETE_QUERY fixture=repeated-original-points scope=input-admission-cache-build-and-32-plane-batches oldOperations={} newOperations={} oldPeak={} newPeak={} oldWallNanoseconds={} newWallNanoseconds={}",
            old.operations(),
            new.operations(),
            old.peak_memberships(),
            new.peak_memberships(),
            old_elapsed.as_nanos(),
            new_elapsed.as_nanos()
        );
        assert!(new_elapsed < old_elapsed.saturating_mul(10));
        drop_cache(cache, &mut new);
        assert_eq!(old.retained_memberships(), inputs + 256);
        assert_eq!(new.retained_memberships(), inputs + 256);
        Ok(())
    })
}

#[test]
fn vertex_distance_frames_never_reuse_other_plane_answers_and_keep_coordinate_refusals()
-> Result<(), Failure> {
    labelled("plane-generations", "case-return", || {
        let originals = repeated(256, false)?;
        let mut new = Budget::new(Limits::default())?;
        let inputs = Facet::input_memberships(&originals, &mut new)?;
        new.retain(inputs + 256)?;
        let mut cache = sections::VertexDistances::build(&originals, &mut new)?
            .ok_or_else(|| new.fail(FailureKind::InvalidWitness, "missing cache"))?;
        for i in [0, 1, 2, 5, 6, 9, 10] {
            let face = query(i)?;
            let mut old = Budget::new(Limits::default())?;
            let mut plane = cache.plane(&face, &mut new)?;
            let mut previous = None;
            for ordinal in [0, 1] {
                let start = new.operations();
                new.step(1)?;
                let actual = new.with_retained(96, |b| plane.section(ordinal, true, true, b))?;
                let used = new.operations() - start;
                old.step(1)?;
                let expected = old.with_retained(96, |b| {
                    sections::joint_section_source(&face, &originals[ordinal], true, true, b)
                })?;
                assert_eq!(actual, expected);
                if let Some(first) = previous {
                    assert_eq!(first - used, 5);
                } else {
                    previous = Some(used);
                }
            }
            assert_eq!(
                checked_failure(plane.section(256, true, true, &mut new)).kind,
                FailureKind::InvalidWitness
            );
        }
        drop_cache(cache, &mut new);
        assert_eq!(new.retained_memberships(), inputs + 256);
        let originals = repeated(256, true)?;
        let face = query(2)?;
        let mut setup = Budget::new(Limits::default())?;
        let mut cache = sections::VertexDistances::build(&originals, &mut setup)?
            .ok_or_else(|| setup.fail(FailureKind::InvalidWitness, "missing endpoint cache"))?;
        for lane in ["reference", "cached"] {
            let mut b = Budget::new(Limits {
                rational_bits: 4,
                ..Limits::default()
            })?;
            let failure = if lane == "reference" {
                checked_failure(b.with_retained(96, |b| {
                    sections::joint_section_source(&face, &originals[0], false, true, b)
                }))
            } else {
                let mut plane = cache.plane(&face, &mut b)?;
                checked_failure(b.with_retained(96, |b| plane.section(0, false, true, b)))
            };
            assert_eq!(failure.kind, FailureKind::Rational, "lane={lane}");
            assert_eq!(b.retained_memberships(), 0);
        }
        drop_cache(cache, &mut setup);
        assert_eq!(setup.retained_memberships(), 0);
        Ok(())
    })
}

#[test]
fn vertex_distance_constructor_and_plane_queries_keep_exact_storage_and_operation_rollback()
-> Result<(), Failure> {
    labelled("distance-resource-guards", "case-return", || {
        let originals = repeated(257, false)?;
        let mut setup = Budget::new(Limits::default())?;
        assert!(sections::VertexDistances::build(&originals[..255], &mut setup)?.is_none());
        assert_eq!(setup.operations(), 0);
        assert_eq!(setup.retained_memberships(), 0);
        let baseline = Facet::input_memberships(&originals, &mut setup)? + 256;
        let mut measured = Budget::new(Limits::default())?;
        measured.retain(baseline)?;
        let cache = sections::VertexDistances::build(&originals, &mut measured)?
            .ok_or_else(|| measured.fail(FailureKind::InvalidWitness, "missing cache"))?;
        let cap = measured.peak_memberships();
        drop_cache(cache, &mut measured);
        assert_eq!(measured.retained_memberships(), baseline);
        let mut exact = Budget::new(Limits {
            storage_memberships: cap,
            ..Limits::default()
        })?;
        exact.retain(baseline)?;
        let cache = sections::VertexDistances::build(&originals, &mut exact)?
            .ok_or_else(|| exact.fail(FailureKind::InvalidWitness, "missing exact cache"))?;
        drop_cache(cache, &mut exact);
        assert_eq!(exact.retained_memberships(), baseline);
        let mut tight = Budget::new(Limits {
            storage_memberships: cap - 1,
            ..Limits::default()
        })?;
        tight.retain(baseline)?;
        assert_eq!(
            checked_failure(sections::VertexDistances::build(&originals, &mut tight)).kind,
            FailureKind::Storage
        );
        assert_eq!(tight.retained_memberships(), baseline);
        let mut short = Budget::new(Limits {
            operations: 1,
            ..Limits::default()
        })?;
        short.retain(baseline)?;
        assert_eq!(
            checked_failure(sections::VertexDistances::build(&originals, &mut short)).kind,
            FailureKind::Operations
        );
        assert_eq!(short.retained_memberships(), baseline);
        let mut cache = sections::VertexDistances::build(&originals, &mut measured)?
            .ok_or_else(|| measured.fail(FailureKind::InvalidWitness, "missing query cache"))?;
        let retained = measured.retained_memberships();
        let face = query(4)?;
        let mut b = Budget::new(Limits {
            operations: 1,
            ..Limits::default()
        })?;
        b.retain(retained)?;
        let mut plane = cache.plane(&face, &mut b)?;
        assert_eq!(
            checked_failure(b.with_retained(96, |b| plane.section(0, true, true, b))).kind,
            FailureKind::Operations
        );
        assert_eq!(b.retained_memberships(), retained);
        let expected = {
            let mut plane = cache.plane(&face, &mut measured)?;
            measured.with_retained(96, |b| plane.section(0, true, true, b))?
        };
        let mut old = Budget::new(Limits::default())?;
        assert_eq!(
            expected,
            old.with_retained(96, |b| sections::joint_section_source(
                &face,
                &originals[0],
                true,
                true,
                b
            ))?
        );
        drop_cache(cache, &mut measured);
        assert_eq!(measured.retained_memberships(), baseline);
        Ok(())
    })
}

#[test]
fn cached_original_vertex_distances_preserve_full_publisher_fields_witnesses_and_volume()
-> Result<(), Failure> {
    labelled("four-closed-original-tetrahedra", "case-return", || {
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
                for vertex in &mut vertices {
                    vertex.point[0] += integer(i64::from(4 * owner));
                    vertex.uv[0] += integer(i64::from(4 * owner));
                }
                originals.push(Facet::new(
                    face.id(),
                    vertices,
                    format!("owner-{owner}"),
                    &mut build,
                )?);
            }
        }
        assert_eq!(originals.len(), 256);
        let mut old = Budget::new(Limits::default())?;
        let mut new = Budget::new(Limits::default())?;
        let start = Instant::now();
        let expected = labelled(
            "four-closed-original-tetrahedra",
            "reference-publisher",
            || reference_vertex_distance_publisher(&originals, &mut old),
        )?;
        let old_elapsed = start.elapsed();
        let start = Instant::now();
        let actual = labelled(
            "four-closed-original-tetrahedra",
            "candidate-publisher",
            || reconstruct(&originals, &mut new),
        )?;
        let new_elapsed = start.elapsed();
        assert_eq!(actual.triangles(), expected.triangles());
        assert_eq!(actual.witnesses(), expected.witnesses());
        assert_eq!(actual.exact_volume(), expected.exact_volume());
        assert_eq!(actual.exact_volume(), &(integer(16) / integer(3)));
        assert_eq!(new.retained_memberships(), old.retained_memberships());
        eprintln!(
            "VERTEX_DISTANCE_COMPLETE_PUBLISHER fixture=four-closed-original-tetrahedra oldOperations={} newOperations={} oldPeak={} newPeak={} oldWallNanoseconds={} newWallNanoseconds={}",
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
