//! Closed y-interval pruning inside one source facet's section frame.
use super::{Frame, crosses};
use crate::{Budget, Failure, Point2, Rational, Segment};

const LEAF: usize = 8;
const MIN_EDGES: usize = 4 * LEAF;

struct Node<'a> {
    lower: &'a Rational,
    upper: &'a Rational,
    start: usize,
    end: usize,
    children: Option<[usize; 2]>,
}
struct Edges<'a> {
    source: &'a [Segment],
    intervals: Vec<[&'a Rational; 2]>,
    order: Vec<usize>,
    nodes: Vec<Node<'a>>,
    root: usize,
}
enum Owner<'a> {
    Linear(&'a [Segment]),
    Indexed(Edges<'a>),
}
pub(in crate::certificate) struct ParityIndex<'a> {
    owners: Vec<Owner<'a>>,
    members: usize,
}

impl<'a> ParityIndex<'a> {
    pub(in crate::certificate) fn build(
        frame: &'a Frame,
        b: &mut Budget,
    ) -> Result<Option<Self>, Failure> {
        // Four leaf blocks amortize the fixed tree structure; smaller owners
        // keep the exact original scan and need no indexed allocation.
        let mut useful = false;
        for edges in frame.owners.values() {
            b.step(1)?;
            if edges.len() >= MIN_EDGES {
                useful = true;
                break;
            }
        }
        if !useful {
            return Ok(None);
        }
        let entry = b.retained_memberships();
        let result = Self::build_admitted(frame, b);
        if result.is_err() {
            b.release(b.retained_memberships() - entry);
        }
        result.map(Some)
    }
    fn build_admitted(frame: &'a Frame, b: &mut Budget) -> Result<Self, Failure> {
        b.retain(16)?;
        let mut result = Self {
            owners: Vec::new(),
            members: 16,
        };
        for edges in frame.owners.values() {
            b.step(1)?;
            b.state([
                result.members,
                result.owners.len(),
                edges.len(),
                0,
                0,
                0,
                0,
                0,
            ]);
            // Enum, borrowed source slice, vector headers, root and record.
            b.retain(16)?;
            result.members += 16;
            let owner = if edges.len() < MIN_EDGES {
                Owner::Linear(edges)
            } else {
                let members = 4 * edges.len() + 8 * Edges::node_limit(edges.len());
                b.retain(members)?;
                result.members += members;
                Owner::Indexed(Edges::build(edges, b)?)
            };
            result.owners.push(owner);
        }
        Ok(result)
    }
    pub(in crate::certificate) fn memberships(&self) -> usize {
        self.members
    }
    pub(in crate::certificate) fn occupied(
        &self,
        p: &Point2,
        b: &mut Budget,
    ) -> Result<Option<bool>, Failure> {
        b.guard(32)?;
        for owner in &self.owners {
            b.step(1)?;
            let inside = match owner {
                Owner::Linear(edges) => {
                    let mut inside = false;
                    for edge in *edges {
                        match crosses(edge, p, b)? {
                            Some(value) => inside ^= value,
                            None => return Ok(None),
                        }
                    }
                    Some(inside)
                }
                Owner::Indexed(edges) => edges.inside(p, b)?,
            };
            match inside {
                Some(true) => return Ok(Some(true)),
                Some(false) => {}
                None => return Ok(None),
            }
        }
        Ok(Some(false))
    }
}
impl<'a> Edges<'a> {
    fn node_limit(count: usize) -> usize {
        2 * count.div_ceil(LEAF).next_power_of_two() - 1
    }
    fn stack_members(count: usize) -> usize {
        8 * (count.ilog2() as usize + 2) + 32
    }
    fn build(source: &'a [Segment], b: &mut Budget) -> Result<Self, Failure> {
        let mut result = Self {
            source,
            intervals: Vec::new(),
            order: Vec::new(),
            nodes: Vec::new(),
            root: 0,
        };
        for (i, [a, c]) in source.iter().enumerate() {
            // One interval comparison and one ordinal initialization.
            b.step(2)?;
            result.intervals.push(if a[1] <= c[1] {
                [&a[1], &c[1]]
            } else {
                [&c[1], &a[1]]
            });
            result.order.push(i);
        }
        result.sort(b)?;
        result.root = b.with_retained(Self::stack_members(source.len()), |b| {
            result.build_range(0, source.len(), b)
        })?;
        Ok(result)
    }
    fn sort(&mut self, b: &mut Budget) -> Result<(), Failure> {
        let count = self.order.len();
        b.with_retained(count + 16, |b| {
            b.step(count as u64)?;
            let mut scratch = vec![0; count];
            let mut width = 1;
            while width < count {
                for start in (0..count).step_by(2 * width) {
                    let middle = (start + width).min(count);
                    let end = (start + 2 * width).min(count);
                    let (mut left, mut right) = (start, middle);
                    for output in &mut scratch[start..end] {
                        b.state([count, width, start, left, right, end, 0, 0]);
                        let from_left = if right == end {
                            true
                        } else if left == middle {
                            false
                        } else {
                            b.step(1)?;
                            self.intervals[self.order[left]][0]
                                <= self.intervals[self.order[right]][0]
                        };
                        b.step(1)?;
                        *output = if from_left {
                            let value = self.order[left];
                            left += 1;
                            value
                        } else {
                            let value = self.order[right];
                            right += 1;
                            value
                        };
                    }
                }
                std::mem::swap(&mut self.order, &mut scratch);
                width *= 2;
            }
            Ok(())
        })
    }
    fn build_range(&mut self, start: usize, end: usize, b: &mut Budget) -> Result<usize, Failure> {
        let lower = self.intervals[self.order[start]][0];
        let (upper, children) = if end - start <= LEAF {
            let mut upper = self.intervals[self.order[start]][1];
            for ordinal in &self.order[start + 1..end] {
                b.step(1)?;
                upper = upper.max(self.intervals[*ordinal][1]);
            }
            (upper, None)
        } else {
            let middle = start + (end - start) / 2;
            let left = self.build_range(start, middle, b)?;
            let right = self.build_range(middle, end, b)?;
            b.step(1)?;
            (
                self.nodes[left].upper.max(self.nodes[right].upper),
                Some([left, right]),
            )
        };
        b.step(1)?;
        let ordinal = self.nodes.len();
        self.nodes.push(Node {
            lower,
            upper,
            start,
            end,
            children,
        });
        Ok(ordinal)
    }
    fn inside(&self, p: &Point2, b: &mut Budget) -> Result<Option<bool>, Failure> {
        // Original crossing scratch and the actual DFS stack coexist.
        b.with_retained(Self::stack_members(self.source.len()) + 32, |b| {
            let mut pending = vec![self.root];
            let mut inside = false;
            while let Some(ordinal) = pending.pop() {
                b.state([
                    self.source.len(),
                    self.nodes.len(),
                    pending.len(),
                    ordinal,
                    0,
                    0,
                    0,
                    0,
                ]);
                b.step(1)?;
                let node = &self.nodes[ordinal];
                if &p[1] < node.lower || &p[1] > node.upper {
                    continue;
                }
                if let Some([left, right]) = node.children {
                    pending.push(right);
                    pending.push(left);
                } else {
                    for ordinal in &self.order[node.start..node.end] {
                        b.step(1)?;
                        let [lower, upper] = self.intervals[*ordinal];
                        if &p[1] < lower || &p[1] > upper {
                            continue;
                        }
                        match crosses(&self.source[*ordinal], p, b)? {
                            Some(value) => inside ^= value,
                            None => return Ok(None),
                        }
                    }
                }
            }
            Ok(Some(inside))
        })
    }
}

#[cfg(test)]
#[path = "parity_index_tests.rs"]
mod tests;
