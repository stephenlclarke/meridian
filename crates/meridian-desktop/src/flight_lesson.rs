//! Presentation of World-issued T01 reviews and retained measurements; no flight authority.
use meridian_core::{FlightReceptionAction, FlightReceptionOutcome, FlightView, World};

pub fn view_name(view: FlightView) -> &'static str {
    match view {
        FlightView::Front => "FRONT",
        FlightView::Rear => "REAR",
        FlightView::Left => "LEFT",
        FlightView::Right => "RIGHT",
    }
}

pub fn review_title(action: FlightReceptionAction) -> &'static str {
    match action {
        FlightReceptionAction::Accept => "FLIGHT LESSON CONSENT",
        FlightReceptionAction::Receive => "FLIGHT LESSON RECEPTION",
        FlightReceptionAction::Cancel => "CANCEL FLIGHT LESSON",
    }
}

pub fn terms(action: FlightReceptionAction) -> &'static str {
    match action {
        FlightReceptionAction::Accept => {
            "Review current-fit flight practice at this issuing berth. Launch with the same owned hull and fit. In front view, face the station centre within 2 degrees at 1000-1500 flight units and move closer with clear sight. Return through its actual doorway, then Shift+1 reviews reception. No fee, cargo or cash reward."
        }
        FlightReceptionAction::Receive => {
            "Receive the actual measured flight and later return at the issuing berth. This settles zero cash and retains the first Flight Reception Family Trace once. Enter confirms and saves; B cancels this review."
        }
        FlightReceptionAction::Cancel => {
            "Cancel this flight lesson at its issuing berth. Retain its history without payment, refund or clue. A new attempt requires fresh consent and an available record slot. Enter confirms and saves; B cancels this review."
        }
    }
}

pub fn line(world: &World) -> String {
    let Some(state) = world.flight_reception_snapshot() else {
        return "FLIGHT LESSON UNAVAILABLE".into();
    };
    let progress = if state.active.is_none() {
        "Shift+1 REVIEW FLIGHT LESSON"
    } else if state.ready {
        "Shift+1 REVIEW FLIGHT RECEPTION"
    } else if state
        .attempts
        .last()
        .is_some_and(|a| a.measured_ticks.is_some())
    {
        "READING RETAINED | RETURN TO ISSUING BERTH"
    } else {
        "FRONT VIEW | ALIGN AND CLOSE AT 1000-1500"
    };
    format!(
        "VIEW {} | {progress} | C LESSON DETAILS",
        view_name(state.view)
    )
}

pub fn append_details(world: &World, lines: &mut Vec<String>) {
    crate::session::append_paragraph(lines, &line(world));
    crate::session::append_paragraph(
        lines,
        "0 FRONT | 1 REAR | 2 LEFT | 3 RIGHT | Shift+1 REVIEW CONSENT OR RECEPTION | Shift+2 REVIEW CANCELLATION | ENTER CONFIRM AND SAVE | B BACK",
    );
    let Some(state) = world.flight_reception_snapshot() else {
        return;
    };
    let count = |outcome| {
        state
            .attempts
            .iter()
            .filter(|a| a.outcome == Some(outcome))
            .count()
    };
    lines.push(format!(
        "FLIGHT HISTORY | COMPLETE {} | FAILED {} | ABANDONED {}",
        count(FlightReceptionOutcome::Complete),
        count(FlightReceptionOutcome::Failed),
        count(FlightReceptionOutcome::Abandoned)
    ));
    lines.push(format!(
        "SHARED ORIGINAL RECORDS {} | ACTIVE SERVICES {}",
        state.retained_mission_records, state.active_services
    ));
    if let Some(attempt) = state.attempts.last() {
        lines.push(format!(
            "ISSUING SYSTEM {} | ATTEMPT {}",
            attempt.issuing_dock.system().system,
            attempt.id.ordinal()
        ));
        if let (Some(ticks), Some(ranges)) = (attempt.measured_ticks, attempt.measured_ranges) {
            lines.push(format!(
                "MEASURED TICKS {} TO {} | RANGE {:.2} TO {:.2}",
                ticks[0], ticks[1], ranges[0], ranges[1]
            ));
        }
    }
    lines.push(
        if state.trace.is_some() {
            "FLIGHT RECEPTION FAMILY TRACE RECORDED ONCE"
        } else {
            "FLIGHT RECEPTION FAMILY TRACE NOT YET RECORDED"
        }
        .into(),
    );
    crate::session::append_paragraph(
        lines,
        "No cash reward. Keep the accepted hull and fit until measured. Use normal flight and actual swept entry; labels alone do not qualify. Shift+2 reviews cancellation at the issuing berth. Earlier receipts and spent ammunition remain unchanged.",
    );
}
