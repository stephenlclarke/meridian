//! Accounted callback entry point for the unchanged planar decomposition.
//! This mechanically matched copy keeps the historical public `build` oracle
//! byte exact. Only callback lifetime admission and the entry name differ;
//! sealing checks the normalized source against the immutable parent.
use crate::exact::{add, area, cross2, orient, point, scale, sub};
use crate::{
    Arrangement, ArrangementWork, Budget, Cell, Failure, FailureKind, Point2, Rational, Segment,
    integer,
};
use num_traits::{Signed, Zero};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
struct Run {
    lower: Cell,
    upper: Cell,
}

fn key(mut segment: Segment) -> Segment {
    if segment[1] < segment[0] {
        segment.swap(0, 1);
    }
    segment
}
fn tree_work(len: usize, b: &mut Budget) -> Result<(), Failure> {
    b.step((len.saturating_add(1).ilog2() + 1).into())
}
fn inside(domain: &[Point2], p: &Point2, strict: bool, b: &mut Budget) -> Result<bool, Failure> {
    for i in 0..domain.len() {
        let side = orient(&domain[i], &domain[(i + 1) % domain.len()], p, b)?;
        if side.is_negative() || strict && side.is_zero() {
            return Ok(false);
        }
    }
    Ok(true)
}
fn clip(domain: &[Point2], segment: &Segment, b: &mut Budget) -> Result<Option<Segment>, Failure> {
    let [first, second] = segment;
    point(first, b)?;
    point(second, b)?;
    let (mut low, mut high) = (integer(0), integer(1));
    for i in 0..domain.len() {
        let a = &domain[i];
        let c = &domain[(i + 1) % domain.len()];
        let x = orient(a, c, first, b)?;
        let y = orient(a, c, second, b)?;
        if x.is_negative() && y.is_negative() {
            return Ok(None);
        }
        if x.is_negative() {
            low = low.max(&x / (&x - &y));
        } else if y.is_negative() {
            high = high.min(&x / (&x - &y));
        }
    }
    if low > high {
        return Ok(None);
    }
    let delta = sub(second, first);
    let result = [
        add(first, &scale(&delta, &low)),
        add(first, &scale(&delta, &high)),
    ];
    for p in &result {
        point(p, b)?;
    }
    Ok(Some(key(result)))
}

impl Arrangement {
    /// Finite clipped constraints and exact sampling, ported from PlanarArrangement.
    /// `visible` receives a strict interior sample of one fully noded atomic region.
    pub(crate) fn build_accounted<F>(
        domain: &[Point2],
        barriers: &[Segment],
        b: &mut Budget,
        mut visible: F,
    ) -> Result<Self, Failure>
    where
        F: FnMut(&Point2, &mut Budget) -> Result<bool, Failure>,
    {
        b.step(1)?;
        b.guard(domain.len())?;
        b.guard(barriers.len())?;
        b.guard(domain.len() * 4 + barriers.len() * 6 + 64)?;
        if domain.len() < 3 {
            return Err(b.fail(
                FailureKind::InvalidInput,
                "facet needs a positive convex domain",
            ));
        }
        for (i, a) in domain.iter().enumerate() {
            point(a, b)?;
            if a == &domain[(i + 1) % domain.len()] {
                return Err(b.fail(FailureKind::Degenerate, "duplicate domain corner"));
            }
            for p in domain {
                if orient(a, &domain[(i + 1) % domain.len()], p, b)?.is_negative() {
                    return Err(b.fail(
                        FailureKind::InvalidInput,
                        "nonconvex or reversed facet domain",
                    ));
                }
            }
        }
        if area(domain, b)? <= integer(0) {
            return Err(b.fail(FailureKind::Degenerate, "zero-area domain"));
        }
        let mut segments = BTreeSet::new();
        let mut protected = BTreeSet::new();
        let mut events = BTreeSet::new();
        let mut work = ArrangementWork::default();
        let mut base = domain.len() * 4 + barriers.len() * 6 + 64;
        for segment in (0..domain.len())
            .map(|i| [domain[i].clone(), domain[(i + 1) % domain.len()].clone()])
            .chain(barriers.iter().cloned())
        {
            if let Some(value) = clip(domain, &segment, b)? {
                b.guard(base + 32 * (segments.len() + protected.len() + events.len() + 1))?;
                tree_work(segments.len(), b)?;
                for p in &value {
                    tree_work(protected.len(), b)?;
                    protected.insert(p.clone());
                    tree_work(events.len(), b)?;
                    events.insert(p[0].clone());
                }
                segments.insert(value);
            }
        }
        let physical = segments.clone();
        base += physical.len() * 6;
        let mut vertical: BTreeMap<Rational, Vec<[Rational; 2]>> = BTreeMap::new();
        for [a, c] in &physical {
            b.step(1)?;
            if a[0] != c[0] {
                continue;
            }
            b.guard(
                base + 32 * (segments.len() + protected.len() + events.len() + physical.len() + 1),
            )?;
            tree_work(vertical.len(), b)?;
            vertical
                .entry(a[0].clone())
                .or_default()
                .push([a[1].clone(), c[1].clone()]);
            let left = domain
                .iter()
                .map(|p| &p[0])
                .min()
                .ok_or_else(|| b.fail(FailureKind::InvalidInput, "missing domain"))?;
            let right = domain
                .iter()
                .map(|p| &p[0])
                .max()
                .ok_or_else(|| b.fail(FailureKind::InvalidInput, "missing domain"))?;
            for p in [a, c] {
                if let Some(chord) = clip(
                    domain,
                    &[[left.clone(), p[1].clone()], [right.clone(), p[1].clone()]],
                    b,
                )? {
                    tree_work(segments.len(), b)?;
                    segments.insert(chord);
                }
                if a == c {
                    work.point_contact_chords += 1;
                } else {
                    work.vertical_contact_chords += 1;
                }
            }
        }
        b.guard(
            base + 32 * (segments.len() + protected.len() + events.len() + physical.len() + 1),
        )?;
        let ordered: Vec<_> = segments.into_iter().collect();
        for (i, [a, c]) in ordered.iter().enumerate() {
            for [d, e] in &ordered[i + 1..] {
                b.step(1)?;
                work.segment_pairs += 1;
                let first = sub(c, a);
                let second = sub(e, d);
                let den = cross2(&first, &second);
                if den.is_zero() {
                    continue;
                }
                let t = cross2(&sub(d, a), &second) / &den;
                let u = cross2(&sub(d, a), &first) / &den;
                if t < integer(0) || t > integer(1) || u < integer(0) || u > integer(1) {
                    continue;
                }
                let hit = add(a, &scale(&first, &t));
                point(&hit, b)?;
                b.guard(
                    base + 32
                        * (ordered.len() + protected.len() + events.len() + physical.len() + 1),
                )?;
                tree_work(events.len(), b)?;
                events.insert(hit[0].clone());
                tree_work(physical.len() * 2, b)?;
                if physical.contains(&key([a.clone(), c.clone()]))
                    && physical.contains(&key([d.clone(), e.clone()]))
                {
                    tree_work(protected.len(), b)?;
                    protected.insert(hit);
                }
            }
        }
        let events: Vec<_> = events.into_iter().collect();
        base += 32 * (ordered.len() + protected.len() + events.len() + physical.len() + 1);
        let mut cells = Vec::new();
        let mut active: BTreeMap<[Rational; 3], Run> = BTreeMap::new();
        let mut members = 0;
        for event in events.windows(2) {
            b.step(1)?;
            work.slabs += 1;
            let (left, right) = (&event[0], &event[1]);
            let middle = (left + right) / integer(2);
            let mut levels = BTreeMap::new();
            for [a, c] in &ordered {
                b.step(1)?;
                if !(a[0] < middle && middle < c[0]) {
                    continue;
                }
                let slope = (&c[1] - &a[1]) / (&c[0] - &a[0]);
                let intercept = &a[1] - &slope * &a[0];
                b.guard(base + 16 * (members + levels.len() + 1))?;
                tree_work(levels.len(), b)?;
                levels.insert(&slope * &middle + &intercept, [slope, intercept]);
            }
            let levels: Vec<_> = levels.into_iter().collect();
            let mut following = BTreeMap::new();
            for bands in levels.windows(2) {
                let [(bottom, lower), (top, upper)] = [&bands[0], &bands[1]];
                let sample = [middle.clone(), (bottom + top) / integer(2)];
                point(&sample, b)?;
                if !inside(domain, &sample, true, b)? {
                    continue;
                }
                b.step(1)?;
                work.classified_cells += 1;
                let exposed = b.with_retained(
                    base + 16 * (members + levels.len() + following.len() + active.len() + 1),
                    |local| visible(&sample, local),
                )?;
                if !exposed {
                    continue;
                }
                let lo_left = [left.clone(), &lower[0] * left + &lower[1]];
                let lo_right = [right.clone(), &lower[0] * right + &lower[1]];
                let hi_left = [left.clone(), &upper[0] * left + &upper[1]];
                let hi_right = [right.clone(), &upper[0] * right + &upper[1]];
                for p in [&lo_left, &lo_right, &hi_left, &hi_right] {
                    point(p, b)?;
                }
                tree_work(active.len(), b)?;
                let mut run = if lo_left[1] < hi_left[1] {
                    active.remove(&[left.clone(), lo_left[1].clone(), hi_left[1].clone()])
                } else {
                    None
                };
                if let Some(previous) = run.as_ref() {
                    let mut blocked = false;
                    for [low, high] in vertical.get(left).into_iter().flatten() {
                        b.step(1)?;
                        let low = low.max(&lo_left[1]);
                        let high = high.min(&hi_left[1]);
                        if low < high || low == high && lo_left[1] < *low && *low < hi_left[1] {
                            blocked = true;
                            break;
                        }
                    }
                    let n = previous.lower.len();
                    let m = previous.upper.len();
                    let convex =
                        orient(&previous.lower[n - 2], &previous.lower[n - 1], &lo_right, b)?
                            >= integer(0)
                            && orient(
                                &previous.upper[m - 2],
                                &previous.upper[m - 1],
                                &hi_right,
                                b,
                            )? <= integer(0);
                    if (blocked || !convex)
                        && let Some(done) = run.take()
                    {
                        finish(done, &mut cells, &mut work, b)?;
                    }
                }
                let mut run = match run {
                    Some(mut old) => {
                        extend(&mut old.lower, lo_right.clone(), &protected, b)?;
                        extend(&mut old.upper, hi_right.clone(), &protected, b)?;
                        work.strips_extended += 1;
                        old
                    }
                    None => Run {
                        lower: vec![lo_left, lo_right.clone()],
                        upper: vec![hi_left, hi_right.clone()],
                    },
                };
                members += 4;
                b.guard(base + 16 * (members + levels.len() + following.len() + active.len() + 1))?;
                if lo_right[1] < hi_right[1] {
                    tree_work(following.len(), b)?;
                    if following
                        .insert(
                            [right.clone(), lo_right[1].clone(), hi_right[1].clone()],
                            run,
                        )
                        .is_some()
                    {
                        return Err(b.fail(
                            FailureKind::InvalidWitness,
                            "overlapping complete vertical sides",
                        ));
                    }
                } else {
                    finish(
                        std::mem::replace(
                            &mut run,
                            Run {
                                lower: Vec::new(),
                                upper: Vec::new(),
                            },
                        ),
                        &mut cells,
                        &mut work,
                        b,
                    )?;
                }
            }
            for run in active.into_values() {
                finish(run, &mut cells, &mut work, b)?;
            }
            active = following;
        }
        for run in active.into_values() {
            finish(run, &mut cells, &mut work, b)?;
        }
        Ok(Self {
            cells,
            physical,
            protected,
            work,
        })
    }
}
fn extend(
    chain: &mut Cell,
    p: Point2,
    protected: &BTreeSet<Point2>,
    b: &mut Budget,
) -> Result<(), Failure> {
    while chain.len() >= 2 {
        let n = chain.len();
        tree_work(protected.len(), b)?;
        if !orient(&chain[n - 2], &chain[n - 1], &p, b)?.is_zero()
            || protected.contains(&chain[n - 1])
        {
            break;
        }
        chain.pop();
    }
    b.step(1)?;
    chain.push(p);
    Ok(())
}
fn finish(
    run: Run,
    cells: &mut Vec<Cell>,
    work: &mut ArrangementWork,
    b: &mut Budget,
) -> Result<(), Failure> {
    b.step((run.lower.len() + run.upper.len()) as u64)?;
    let mut result = run.lower;
    result.extend(run.upper.into_iter().rev());
    result.dedup();
    if result.len() > 1 && result.first() == result.last() {
        result.pop();
    }
    if result.len() < 3 || area(&result, b)? <= integer(0) {
        return Err(b.fail(FailureKind::Degenerate, "strip lost positive area"));
    }
    cells.push(result);
    work.retained_cells += 1;
    Ok(())
}
