use super::*;
use crate::{Limits, Segment};
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/proved-visits-reference/compiled-clip-control.rs"
));

#[test]
fn strict_box_pruning_matches_frozen_clip_preserves_contacts_and_admits_before_mutation()
-> Result<(), Failure> {
    let p = |x, y| [integer(x), integer(y)];
    for axis in 0..3 {
        let mut setup = Budget::new(Limits::default())?;
        let face = Facet::new(
            SourceId {
                component: 0,
                facet: 0,
            },
            [[0, 0], [4, 0], [0, 4]].map(|q| {
                let mut next = 0;
                Vertex {
                    point: std::array::from_fn(|k| {
                        if k == axis {
                            integer(0)
                        } else {
                            let n = integer(q[next]);
                            next += 1;
                            n
                        }
                    }),
                    uv: [integer(0), integer(0)],
                }
            }),
            "test".into(),
            &mut setup,
        )?;
        let bounds = Bounds::for_facet(&face, &mut setup)?;
        let domain = face.domain();
        let mut input = Vec::new();
        for i in 0..100 {
            input.push([p(-5, i), p(-1, i + 1)]);
        }
        input.extend([
            [p(5, 1), p(8, 1)],
            [p(0, 5), p(4, 5)],
            [p(0, -5), p(4, -1)],
            [p(0, 0), p(0, 2)],
            [p(0, 0), p(0, 0)],
            [p(4, 0), p(4, 0)],
            [p(-1, 2), p(5, 2)],
            [p(3, 3), p(4, 4)],
        ]);
        let mut old = Budget::new(Limits::default())?;
        let expected = Arrangement::build_accounted(&domain, &input, &mut old, |_, _| Ok(true))?;
        let mut new = Budget::new(Limits::default())?;
        new.retain(8 * input.len())?;
        let mut filtered = input.clone();
        prune_impossible_barriers(&face, &bounds, &mut filtered, &mut new)?;
        assert_eq!(new.retained_memberships(), 8 * filtered.len());
        assert!(filtered.contains(&[p(0, 0), p(0, 0)]));
        assert!(filtered.contains(&[p(4, 0), p(4, 0)]));
        assert!(filtered.contains(&[p(0, 0), p(0, 2)]));
        assert!(filtered.contains(&[p(3, 3), p(4, 4)]));
        for segment in &input {
            if !filtered.contains(segment) {
                assert_eq!(frozen_clip(&domain, segment, &mut setup)?, None);
            }
        }
        new.release(8 * filtered.len());
        let actual = Arrangement::build_accounted(&domain, &filtered, &mut new, |_, _| Ok(true))?;
        assert_eq!(actual, expected);
        assert!(new.operations() < old.operations());
        assert_eq!(new.retained_memberships(), 0);
        let reserve = 8 * input.len();
        let mut exact = Budget::new(Limits {
            storage_memberships: reserve + 16,
            ..Limits::default()
        })?;
        exact.retain(reserve)?;
        let mut copy = input.clone();
        prune_impossible_barriers(&face, &bounds, &mut copy, &mut exact)?;
        assert_eq!(copy, filtered);
        assert_eq!(exact.peak_memberships(), reserve + 16);
        let mut tight = Budget::new(Limits {
            storage_memberships: reserve + 15,
            ..Limits::default()
        })?;
        tight.retain(reserve)?;
        let mut copy = input.clone();
        assert_eq!(
            prune_impossible_barriers(&face, &bounds, &mut copy, &mut tight)
                .err()
                .map(|e| e.kind),
            Some(FailureKind::Storage)
        );
        assert_eq!(copy, input);
        assert_eq!(tight.retained_memberships(), reserve);
        let mut work = Budget::new(Limits {
            operations: (2 * input.len() - 1) as u64,
            ..Limits::default()
        })?;
        work.retain(reserve)?;
        assert_eq!(
            prune_impossible_barriers(&face, &bounds, &mut copy, &mut work)
                .err()
                .map(|e| e.kind),
            Some(FailureKind::Operations)
        );
        assert_eq!(copy, input);
        assert_eq!(work.retained_memberships(), reserve);
    }
    Ok(())
}
