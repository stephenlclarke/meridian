//! Opt-in, bounded review-transition evidence. No gameplay state or GPU success inference.
use crate::docked_screens::Frame;
use meridian_core::{LaserFacing, ModuleAction};
use std::{
    fmt::{self, Write},
    sync::{Arc, Mutex},
};

const MAX_TRANSITIONS: u8 = 8;
const MAX_RECORDS: u8 = 20;
const MAX_LINE: usize = 4096;
const MAX_ASYNC_ERRORS: u8 = 2;
type Sink = Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReviewId {
    pub token: u64,
    pub item: u64,
    pub operation: &'static str,
    pub facing: Option<LaserFacing>,
}
impl ReviewId {
    pub fn from_action(token: u64, action: ModuleAction) -> Option<Self> {
        let (item, operation, facing) = match action {
            ModuleAction::Store { item } => (item, "Store", None),
            ModuleAction::InstallLaser { item, facing, .. } => (item, "Fit", Some(facing)),
            ModuleAction::Install { item, .. } => (item, "Fit", None),
            _ => return None,
        };
        Some(Self {
            token,
            item,
            operation,
            facing,
        })
    }
}
struct State {
    last: Option<ReviewId>,
    transitions: u8,
    async_errors: u8,
}
struct Shared {
    state: Mutex<State>,
    sink: Sink,
}
#[derive(Clone, Default)]
pub(crate) struct Capture(Option<Arc<Shared>>);
pub(crate) struct Attempt {
    shared: Arc<Shared>,
    id: ReviewId,
    sequence: u8,
    records: std::cell::Cell<u8>,
}

// Limit UTF-8 by bytes, preserving character boundaries and escaping newlines through Debug.
struct Line {
    text: String,
    truncated: bool,
}
impl Write for Line {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let remaining = MAX_LINE.saturating_sub(self.text.len());
        let mut end = s.len().min(remaining);
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        self.truncated |= end < s.len();
        self.text.push_str(&s[..end]);
        Ok(())
    }
}
fn line(arguments: fmt::Arguments<'_>) -> String {
    let mut output = Line {
        text: String::with_capacity(MAX_LINE),
        truncated: false,
    };
    let _ = output.write_fmt(arguments);
    if output.truncated {
        let marker = " [TRUNCATED]";
        let mut end = output.text.len().min(MAX_LINE - marker.len());
        while !output.text.is_char_boundary(end) {
            end -= 1;
        }
        output.text.truncate(end);
        output.text.push_str(marker);
    }
    output.text
}
impl Capture {
    pub fn from_environment() -> Self {
        Self::with_sink(
            std::env::var_os("MERIDIAN_REVIEW_CAPTURE").is_some_and(|v| v == "1"),
            Arc::new(|s| {
                use std::io::Write as _;
                let _ = writeln!(std::io::stderr().lock(), "{s}");
            }),
        )
    }
    fn with_sink(enabled: bool, sink: Sink) -> Self {
        Self(enabled.then(|| {
            Arc::new(Shared {
                state: Mutex::new(State {
                    last: None,
                    transitions: 0,
                    async_errors: 0,
                }),
                sink,
            })
        }))
    }
    pub fn begin(&self, id: Option<ReviewId>) -> Option<Attempt> {
        let shared = self.0.as_ref()?;
        let mut state = shared.state.lock().ok()?;
        if id.is_none() {
            state.last = None;
            return None;
        }
        if state.last == id || state.transitions >= MAX_TRANSITIONS {
            return None;
        }
        state.last = id;
        state.transitions += 1;
        Some(Attempt {
            shared: shared.clone(),
            id: id?,
            sequence: state.transitions,
            records: std::cell::Cell::new(0),
        })
    }
    // Callback timing cannot identify the originating GPU frame. Correlation is explicitly latest-only.
    pub fn asynchronous_error(&self, error: impl fmt::Debug) {
        let Some(shared) = &self.0 else {
            return;
        };
        let Ok(mut state) = shared.state.lock() else {
            return;
        };
        if state.last.is_none() || state.async_errors >= MAX_ASYNC_ERRORS {
            return;
        }
        state.async_errors += 1;
        let message = line(format_args!(
            "MERIDIAN_REVIEW_CAPTURE v1 latest_captured_review={:?} asynchronous_error={error:?} originating_frame=UNOBSERVABLE gpu_completion=UNOBSERVABLE",
            state.last
        ));
        drop(state);
        (shared.sink)(&message);
    }
}
impl Attempt {
    pub fn record(&self, stage: &'static str, data: impl fmt::Debug) {
        let count = self.records.get();
        if count >= MAX_RECORDS {
            return;
        }
        self.records.set(count + 1);
        (self.shared.sink)(&line(format_args!(
            "MERIDIAN_REVIEW_CAPTURE v1 transition={} review={:?} stage={stage} data={data:?}",
            self.sequence, self.id
        )));
    }
    pub fn frame(&self, frame: Option<&Frame>) {
        let Some(f) = frame else {
            self.record("cpu_frame", "MISSING");
            return;
        };
        self.record(
            "cpu_frame",
            (
                f.stamp,
                f.ui_revision,
                f.screen,
                f.viewport,
                f.labels.len(),
                f.labels
                    .iter()
                    .map(|l| l.text.chars().count())
                    .sum::<usize>(),
                f.lines.len(),
                f.hits.len(),
                f.hits.iter().filter(|h| h.enabled).count(),
                f.models.len(),
                f.planets.len(),
            ),
        );
        // Actual post-App labels (including title prefix), clips and hit boxes; bounded samples.
        self.record(
            "cpu_labels_first8",
            f.labels.iter().take(8).collect::<Vec<_>>(),
        );
        self.record("cpu_hits_first8", f.hits.iter().take(8).collect::<Vec<_>>());
        self.record(
            "cpu_clip_counts",
            (
                f.labels
                    .iter()
                    .filter(|l| {
                        [l.clip.x, l.clip.y, l.clip.width, l.clip.height]
                            .into_iter()
                            .all(f64::is_finite)
                            && l.clip.width > 0.0
                            && l.clip.height > 0.0
                    })
                    .count(),
                f.labels
                    .iter()
                    .filter(|l| l.clip.width <= 0.0 || l.clip.height <= 0.0)
                    .count(),
            ),
        );
    }
}
/// FNV-1a over each length-delimited exact upload byte slice; diagnostic, not a security hash.
pub(crate) fn digest(parts: &[&[u8]]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for part in parts {
        for byte in (part.len() as u64).to_le_bytes().iter().chain(part.iter()) {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
        }
    }
    hash
}
#[cfg(test)]
pub(crate) mod tests;
