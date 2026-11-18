use super::*;
use std::sync::Mutex;
fn fixture(enabled: bool) -> (Capture, Arc<Mutex<Vec<String>>>) {
    let records = Arc::new(Mutex::new(Vec::new()));
    let sink = records.clone();
    (
        Capture::with_sink(
            enabled,
            Arc::new(move |s| sink.lock().unwrap_or_else(|e| panic!("{e}")).push(s.into())),
        ),
        records,
    )
}
fn id(token: u64) -> ReviewId {
    ReviewId::from_action(token, ModuleAction::Store { item: 17 })
        .unwrap_or_else(|| panic!("missing Store identity"))
}
fn records(log: &Mutex<Vec<String>>) -> Vec<String> {
    log.lock().unwrap_or_else(|e| panic!("{e}")).clone()
}
#[test]
fn disabled_capture_and_nonreview_actions_emit_nothing() {
    let (capture, log) = fixture(false);
    assert!(capture.begin(Some(id(1))).is_none());
    capture.asynchronous_error("device callback");
    assert!(records(&log).is_empty());
    assert!(
        ReviewId::from_action(
            1,
            ModuleAction::Purchase {
                kind: meridian_core::ModuleKind::BeamLaser
            }
        )
        .is_none()
    );
    assert_eq!(
        ReviewId::from_action(
            9,
            ModuleAction::InstallLaser {
                item: 17,
                facing: LaserFacing::Front,
                replacing: None
            }
        ),
        Some(ReviewId {
            token: 9,
            item: 17,
            operation: "Fit",
            facing: Some(LaserFacing::Front)
        })
    );
}
#[test]
fn repeated_frames_and_excess_transitions_cannot_grow_the_capture() {
    let (capture, log) = fixture(true);
    for token in 1..=u64::from(MAX_TRANSITIONS) + 10 {
        if let Some(attempt) = capture.begin(Some(id(token))) {
            for _ in 0..MAX_RECORDS + 10 {
                attempt.record("guard", "bounded");
            }
        }
        for _ in 0..100 {
            assert!(capture.begin(Some(id(token))).is_none());
        }
    }
    assert_eq!(
        records(&log).len(),
        usize::from(MAX_TRANSITIONS) * usize::from(MAX_RECORDS)
    );
    assert!(capture.begin(None).is_none());
    assert!(capture.begin(Some(id(1))).is_none());
}
#[test]
fn error_and_missing_frame_records_never_claim_completion_and_lines_are_bounded() {
    let (capture, log) = fixture(true);
    let attempt = capture
        .begin(Some(id(3)))
        .unwrap_or_else(|| panic!("missing capture"));
    attempt.frame(None);
    attempt.record("compose_result", Err::<(), _>("bad clip\nnot a new record"));
    attempt.record("long_utf8", "é".repeat(MAX_LINE * 2));
    for _ in 0..10 {
        capture.asynchronous_error("validation error");
    }
    let lines = records(&log);
    assert_eq!(lines.len(), 3 + usize::from(MAX_ASYNC_ERRORS));
    assert!(lines[0].contains("MISSING"));
    assert!(lines[1].contains("Err") && lines[1].contains("\\n"));
    assert!(lines[2].ends_with(" [TRUNCATED]"));
    assert!(
        lines
            .iter()
            .all(|s| s.len() <= MAX_LINE && !s.contains('\n'))
    );
    assert!(
        lines[3..]
            .iter()
            .all(|s| s.contains("originating_frame=UNOBSERVABLE")
                && s.contains("gpu_completion=UNOBSERVABLE"))
    );
}
#[test]
fn geometry_digest_binds_exact_bytes_order_and_buffer_boundaries() {
    let empty = digest(&[]);
    assert_eq!(empty, 0xcbf29ce484222325);
    assert_ne!(digest(&[b"ab", b"c"]), digest(&[b"a", b"bc"]));
    assert_ne!(digest(&[b"ab", b"c"]), digest(&[b"c", b"ab"]));
    assert_ne!(digest(&[b"ab", b"c"]), digest(&[b"ab", b"d"]));
    // Independently calculated length-prefixed byte vector.
    assert_eq!(digest(&[b"ab", b"c"]), 9106356563233852118);
}
// Used by the genuine paid/physical-berth test, not a fabricated Review or Frame fixture.
pub(crate) fn assert_actual_frame_record(frame: &Frame, identity: ReviewId) {
    let (capture, log) = fixture(true);
    let attempt = capture
        .begin(Some(identity))
        .unwrap_or_else(|| panic!("missing real transition"));
    attempt.frame(Some(frame));
    let lines = records(&log);
    assert_eq!(lines.len(), 4);
    assert!(
        lines
            .iter()
            .all(|s| s.contains(&format!("token: {}", identity.token))
                && s.contains(&format!("item: {}", identity.item)))
    );
    assert!(
        lines[1].contains("Equipment review")
            && lines[1].contains("Beam Laser")
            && lines[1].contains("clip:")
    );
    assert!(
        lines[2].contains("ReviewConfirm")
            && lines[2].contains("ReviewCancel")
            && lines[2].contains("enabled: true")
    );
}
