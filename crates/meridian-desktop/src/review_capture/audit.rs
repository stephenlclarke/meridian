//! Fixed latest-attempt history; v1 first-attempt output remains separate and unchanged.
use super::*;
use std::{cell::RefCell, collections::VecDeque};
pub(crate) const LATEST: usize = 4;
const OUTPUT_RECORDS: u16 = 1536;
const OBSERVATIONS: u8 = 3;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tag {
    pub request: u8,
    pub frame: u64,
    pub review: ReviewId,
}
pub(super) struct Frame {
    pub id: u64,
    pub review: ReviewId,
    pub request: Option<u8>,
    pub lines: RefCell<Vec<String>>,
}
struct Entry {
    frame: u64,
    review: ReviewId,
    stamp: Option<(crate::docked_screens::Stamp, u64)>,
    lines: Vec<String>,
    observation: bool,
}
pub(super) struct State {
    current: Option<ReviewId>,
    next_frame: u64,
    ring: VecDeque<Entry>,
    overwritten: u64,
    remaining: u16,
    requests: u8,
    pending: Option<(u8, ReviewId)>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            current: None,
            next_frame: 0,
            ring: VecDeque::with_capacity(LATEST),
            overwritten: 0,
            remaining: OUTPUT_RECORDS,
            requests: 0,
            pending: None,
        }
    }
}
impl State {
    pub fn emit(&mut self, sink: &Sink, stage: &'static str, data: impl fmt::Debug) {
        if self.remaining == 0 {
            return;
        }
        self.remaining -= 1;
        if self.remaining == 0 {
            sink(
                "MERIDIAN_REVIEW_CAPTURE v2 OUTPUT_BUDGET_EXHAUSTED; subsequent boundaries UNOBSERVABLE",
            );
            return;
        }
        sink(&line(format_args!(
            "MERIDIAN_REVIEW_CAPTURE v2 stage={stage} data={data:?}"
        )));
    }
    pub fn flush(&mut self, sink: &Sink, reason: &'static str) {
        let summaries: Vec<_> = self
            .ring
            .iter()
            .map(|e| (e.frame, e.review, e.stamp, e.lines.clone()))
            .collect();
        self.emit(
            sink,
            "latest_begin",
            (reason, self.current, self.overwritten, summaries.len()),
        );
        for (frame, review, stamp, records) in summaries {
            self.emit(sink, "latest_frame", (frame, review, stamp));
            for record in records {
                self.emit(sink, "latest_record", (frame, record));
            }
        }
        self.emit(sink, "latest_end", reason);
    }
    pub fn enter(&mut self, sink: &Sink, id: Option<ReviewId>, admitted: bool) -> Option<Frame> {
        let id = if admitted { id } else { None };
        if self.current != id {
            if self.current.is_some() {
                self.flush(sink, "review_transition_or_exit");
            }
            if let Some(pending) = self.pending.take() {
                self.emit(
                    sink,
                    "observation_unobservable",
                    (pending, "review changed before requested draw"),
                );
            }
            self.ring.clear();
            self.overwritten = 0;
            self.current = id;
        }
        if !admitted {
            return None;
        }
        let review = id?;
        self.next_frame = self.next_frame.checked_add(1)?;
        let request = self
            .pending
            .take()
            .filter(|(_, expected)| *expected == review)
            .map(|(n, _)| n);
        if let Some(n) = request {
            self.emit(
                sink,
                "observation_frame_assigned",
                Tag {
                    request: n,
                    frame: self.next_frame,
                    review,
                },
            );
        }
        Some(Frame {
            id: self.next_frame,
            review,
            request,
            lines: RefCell::new(Vec::with_capacity(MAX_RECORDS as usize)),
        })
    }
    pub fn finish(&mut self, frame: &Frame, stamp: Option<(crate::docked_screens::Stamp, u64)>) {
        if self.ring.len() == LATEST {
            self.ring.pop_front();
            self.overwritten = self.overwritten.saturating_add(1);
        }
        self.ring.push_back(Entry {
            frame: frame.id,
            review: frame.review,
            stamp,
            lines: frame.lines.borrow().clone(),
            observation: frame.request.is_some(),
        });
    }
    pub fn emit_observation_frame(&mut self, sink: &Sink, frame: u64) {
        let Some(entry) = self.ring.iter().find(|e| e.frame == frame) else {
            return;
        };
        let (identity, stamp, lines) = (entry.review, entry.stamp, entry.lines.clone());
        self.emit(sink, "observation_actual_frame", (frame, identity, stamp));
        for record in lines {
            self.emit(sink, "observation_actual_record", (frame, record));
        }
    }
    pub fn acknowledge(
        &mut self,
        sink: &Sink,
        expected: (crate::docked_screens::Stamp, u64),
        actual: Option<(crate::docked_screens::Stamp, u64)>,
        accepted: bool,
    ) {
        let mut observation = None;
        if let Some(entry) = self
            .ring
            .iter_mut()
            .rev()
            .find(|e| e.stamp == Some(expected))
        {
            if entry.lines.len() < MAX_RECORDS as usize {
                entry.lines.push(line(format_args!("app_ack expected={expected:?} actual={actual:?} accepted={accepted} identity_matches={} GPUcompletion/display=UNOBSERVABLE",actual==Some(expected))));
            }
            if entry.observation {
                observation = Some((
                    entry.frame,
                    expected,
                    actual,
                    accepted,
                    actual == Some(expected),
                ));
            }
        }
        if let Some(value) = observation {
            self.emit(sink, "observation_app_ack", value);
        }
    }
    pub fn request(&mut self, sink: &Sink, id: ReviewId) -> bool {
        if self.current != Some(id) || self.pending.is_some() || self.requests >= OBSERVATIONS {
            return false;
        }
        self.requests += 1;
        self.flush(sink, "F8_observation_input");
        self.pending = Some((self.requests, id));
        self.emit(sink,"observation_requested",(self.requests,id,"latest history is before this input; pixel request is NEXT matching draw, not the previous screenshot"));
        true
    }
    pub fn exit(&mut self, sink: &Sink) {
        if self.current.is_some() {
            self.flush(sink, "renderer_drop");
            self.current = None;
        }
        if let Some(p) = self.pending.take() {
            self.emit(
                sink,
                "observation_unobservable",
                (p, "renderer exited before requested draw"),
            );
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
