use super::*;
fn fixture() -> (Capture, Arc<Mutex<Vec<String>>>) {
    let records = Arc::new(Mutex::new(Vec::new()));
    let output = records.clone();
    (
        Capture::with_sink(
            true,
            Arc::new(move |s| {
                output
                    .lock()
                    .unwrap_or_else(|e| panic!("{e}"))
                    .push(s.into())
            }),
        ),
        records,
    )
}
fn id(token: u64) -> ReviewId {
    ReviewId {
        token,
        item: 17,
        operation: "Fit",
        facing: Some(LaserFacing::Front),
    }
}
fn log(records: &Mutex<Vec<String>>) -> Vec<String> {
    records.lock().unwrap_or_else(|e| panic!("{e}")).clone()
}
#[test]
fn later_frames_keep_fixed_ring_without_per_frame_output_and_exit_is_bounded() {
    let (capture, records) = fixture();
    for _ in 0..100 {
        let trace = capture
            .begin_draw(Some(id(1)))
            .unwrap_or_else(|| panic!("missing draw"));
        trace.record("probe", "actual adapter seam only; no GPU claim");
        capture.finish_draw(&trace, false);
    }
    assert_eq!(log(&records).len(), 1); // unchanged v1 first attempt only
    let shared = capture.0.as_ref().unwrap_or_else(|| panic!("disabled"));
    {
        let state = shared.state.lock().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(state.audit.ring.len(), LATEST);
        assert_eq!(state.audit.overwritten, 96);
        assert_eq!(state.audit.ring.back().map(|e| e.frame), Some(100));
    }
    capture.exit();
    let before = log(&records).len();
    capture.exit();
    assert_eq!(log(&records).len(), before);
    assert!(log(&records).iter().any(|s| s.contains("renderer_drop")));
}
#[test]
fn three_explicit_observations_bind_next_frame_and_cannot_cross_review_or_grow_output() {
    let (capture, records) = fixture();
    let first = capture
        .begin_draw(Some(id(1)))
        .unwrap_or_else(|| panic!("no frame"));
    capture.finish_draw(&first, false);
    for request in 1..=3 {
        assert!(capture.request_observation(Some(id(1))));
        assert!(!capture.request_observation(Some(id(1))));
        let next = capture
            .begin_draw(Some(id(1)))
            .unwrap_or_else(|| panic!("no next"));
        let tag = next
            .pixel_request()
            .unwrap_or_else(|| panic!("missing request"));
        assert_eq!(tag.request, request);
        assert_eq!(tag.frame, u64::from(request) + 1);
        assert_eq!(tag.review, id(1));
        capture.finish_draw(&next, false);
    }
    assert!(!capture.request_observation(Some(id(1))));
    for token in 2..100 {
        if let Some(trace) = capture.begin_draw(Some(id(token))) {
            for _ in 0..100 {
                trace.record("bounded", "x".repeat(MAX_LINE * 2));
            }
            capture.finish_draw(&trace, false);
        }
    }
    let lines = log(&records);
    assert!(lines.iter().all(|s| s.len() <= MAX_LINE));
    assert!(
        lines.len()
            <= usize::from(OUTPUT_RECORDS)
                + usize::from(MAX_TRANSITIONS) * usize::from(MAX_RECORDS)
    );
}
#[test]
fn changed_review_invalidates_observation_instead_of_copying_another_frame() {
    let (capture, records) = fixture();
    let first = capture
        .begin_draw(Some(id(1)))
        .unwrap_or_else(|| panic!("no frame"));
    capture.finish_draw(&first, false);
    assert!(capture.request_observation(Some(id(1))));
    let next = capture
        .begin_draw(Some(id(2)))
        .unwrap_or_else(|| panic!("no new review"));
    assert!(next.pixel_request().is_none());
    assert!(
        log(&records)
            .iter()
            .any(|s| s.contains("review changed before requested draw"))
    );
}
// Genuine public-paid physical-berth regression supplies its real Frame and pending identity.
pub(crate) fn assert_actual_ack_record(frame: &crate::docked_screens::Frame, id: ReviewId) {
    let (capture, records) = fixture();
    let trace = capture
        .begin_draw(Some(id))
        .unwrap_or_else(|| panic!("no real trace"));
    trace.frame(Some(frame));
    capture.finish_draw(&trace, false);
    let identity = (frame.stamp, frame.ui_revision);
    capture.acknowledge(identity, None, false);
    capture.request_observation(Some(id));
    assert!(
        log(&records)
            .iter()
            .any(|s| s.contains("app_ack") && s.contains("accepted=false"))
    );
    capture.acknowledge(identity, Some(identity), true);
    capture.exit();
    assert!(log(&records).iter().any(|s| s.contains("app_ack")
        && s.contains("accepted=true")
        && s.contains("GPUcompletion/display=UNOBSERVABLE")));
}

#[test]
fn disabled_v2_has_no_frame_identity_observation_ack_or_exit_output() {
    let records = Arc::new(Mutex::new(Vec::new()));
    let output = records.clone();
    let capture = Capture::with_sink(
        false,
        Arc::new(move |s| {
            output
                .lock()
                .unwrap_or_else(|e| panic!("{e}"))
                .push(s.to_owned())
        }),
    );
    assert!(capture.begin_draw(Some(id(1))).is_none());
    assert!(!capture.request_observation(Some(id(1))));
    capture.event("guard", "must not emit");
    capture.exit();
    assert!(log(&records).is_empty());
}
#[test]
fn audit_output_budget_has_one_explicit_exhaustion_marker_and_no_later_growth() {
    let (capture, records) = fixture();
    for _ in 0..usize::from(OUTPUT_RECORDS) + 100 {
        capture.event("budget", "bounded");
    }
    let lines = log(&records);
    assert_eq!(lines.len(), usize::from(OUTPUT_RECORDS));
    assert!(
        lines
            .last()
            .is_some_and(|l| l.contains("OUTPUT_BUDGET_EXHAUSTED"))
    );
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.contains("OUTPUT_BUDGET_EXHAUSTED"))
            .count(),
        1
    );
}

#[test]
fn presented_reports_real_rejection_even_if_previous_painted_identity_matches() {
    let world = meridian_core::World::new_commander(54023).unwrap_or_else(|e| panic!("{e:?}"));
    let mut menu = crate::docked::Docked::new(&world).unwrap_or_else(|e| panic!("{e}"));
    menu.open(&world, crate::docked_screens::Screen::Equipment)
        .unwrap_or_else(|e| panic!("{e}"));
    let viewport =
        crate::docked_screens::Viewport::new(640.0, 360.0).unwrap_or_else(|| panic!("viewport"));
    let first = menu
        .build_frame(&world, viewport, 0.0)
        .unwrap_or_else(|e| panic!("{e}"));
    let expected = (first.stamp, first.ui_revision);
    assert!(menu.presented(first.clone()));
    let _newer_unpresented = menu
        .build_frame(&world, viewport, 0.0)
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(!menu.presented(first));
    assert_eq!(menu.captured_painted_identity(), Some(expected));
}
