use super::DischargeJournal;
use std::cell::RefCell;
use std::rc::Rc;

use super::{
    AudioFrame, AudioOutput, AudioPresenter, Discharge, DischargeKind, OutputStatus, SoundEffect,
};

#[derive(Default)]
struct Playback {
    played: Vec<SoundEffect>,
    stops: usize,
    tool: bool,
    tool_starts: usize,
    failed: bool,
}
struct Fake(Rc<RefCell<Playback>>);
impl AudioOutput for Fake {
    fn available(&self) -> bool {
        !self.0.borrow().failed
    }
    fn maintain(&mut self) { /* Fake completion is set explicitly by each test. */
    }
    fn play(&mut self, effect: SoundEffect) {
        self.0.borrow_mut().played.push(effect);
    }
    fn continuous_tool_active(&self) -> bool {
        self.0.borrow().tool
    }
    fn play_continuous_tool(&mut self) {
        let mut state = self.0.borrow_mut();
        state.tool = true;
        state.tool_starts += 1;
    }
    fn stop_all(&mut self) {
        let mut state = self.0.borrow_mut();
        state.stops += 1;
        state.tool = false;
    }
}
fn frame(revision: u64, docked: bool) -> AudioFrame<'static> {
    AudioFrame {
        epoch: 1,
        revision,
        docked,
        discharges: &[],
        message: "",
        message_live: false,
    }
}
fn presenter() -> (AudioPresenter<Fake>, Rc<RefCell<Playback>>) {
    let state = Rc::new(RefCell::new(Playback::default()));
    let mut presenter = AudioPresenter::new(Fake(Rc::clone(&state)));
    presenter.begin_session(&frame(0, true));
    (presenter, state)
}

#[test]
fn generated_waves_match_independent_source_transcription() {
    for (effect, expected) in [
        (
            SoundEffect::Laser,
            include_bytes!("fixtures/laser.wav").as_slice(),
        ),
        (
            SoundEffect::Launch,
            include_bytes!("fixtures/launch.wav").as_slice(),
        ),
        (
            SoundEffect::Dock,
            include_bytes!("fixtures/dock.wav").as_slice(),
        ),
        (
            SoundEffect::Explosion,
            include_bytes!("fixtures/explosion.wav").as_slice(),
        ),
        (
            SoundEffect::Ping,
            include_bytes!("fixtures/ping.wav").as_slice(),
        ),
        (
            SoundEffect::Ecm,
            include_bytes!("fixtures/ecm.wav").as_slice(),
        ),
    ] {
        assert_eq!(effect.wav(), expected, "{effect:?}");
    }
}

#[test]
fn docking_edges_play_once_and_older_frames_cannot_rewind() {
    let (mut audio, state) = presenter();
    audio.update(&frame(1, false));
    audio.update(&frame(1, false));
    audio.update(&frame(0, true));
    audio.update(&frame(2, false));
    audio.update(&frame(3, true));
    assert_eq!(
        state.borrow().played,
        [SoundEffect::Launch, SoundEffect::Dock]
    );
}

#[test]
fn cold_continue_consumes_retained_events_and_message() {
    let (mut audio, state) = presenter();
    let events = [Discharge {
        id: 7,
        kind: DischargeKind::OneShot,
    }];
    let restored = AudioFrame {
        epoch: 2,
        revision: 0,
        docked: false,
        discharges: &events,
        message: "MISSILE AWAY",
        message_live: true,
    };
    audio.begin_session(&restored);
    let retained = AudioFrame {
        revision: 1,
        ..restored
    };
    audio.update(&retained);
    assert!(state.borrow().played.is_empty());
    audio.begin_session(&frame(2, true));
    audio.update(&frame(3, true));
    assert!(state.borrow().played.is_empty());
}

#[test]
fn discharge_identity_survives_cleared_and_retained_frames() {
    let (mut audio, state) = presenter();
    let events = [Discharge {
        id: 1,
        kind: DischargeKind::OneShot,
    }; 2];
    audio.update(&AudioFrame {
        discharges: &events,
        ..frame(1, true)
    });
    audio.update(&frame(2, true));
    audio.update(&AudioFrame {
        discharges: &events,
        ..frame(3, true)
    });
    let new_event = [Discharge {
        id: 2,
        kind: DischargeKind::OneShot,
    }];
    audio.update(&AudioFrame {
        discharges: &new_event,
        ..frame(4, true)
    });
    assert_eq!(
        state.borrow().played,
        [SoundEffect::Laser, SoundEffect::Laser]
    );
}

#[test]
fn missile_notification_has_no_invented_missile_waveform() {
    let (mut audio, state) = presenter();
    audio.update(&AudioFrame {
        message: "MISSILE AWAY",
        message_live: true,
        ..frame(1, true)
    });
    audio.update(&AudioFrame {
        message: "MISSILE AWAY",
        message_live: true,
        ..frame(2, true)
    });
    assert_eq!(state.borrow().played, [SoundEffect::Ping]);
}

#[test]
fn expired_message_is_consumed_and_exact_ecm_text_selects_original_tone() {
    let (mut audio, state) = presenter();
    audio.update(&AudioFrame {
        message: "OLD",
        ..frame(1, true)
    });
    audio.update(&AudioFrame {
        message: "OLD",
        message_live: true,
        ..frame(2, true)
    });
    audio.update(&AudioFrame {
        message: "E.C.M.",
        message_live: true,
        ..frame(3, true)
    });
    assert_eq!(state.borrow().played, [SoundEffect::Ecm]);
}

#[test]
fn mute_consumes_events_without_unmute_catchup() {
    let (mut audio, state) = presenter();
    audio.set_muted(true);
    assert_eq!(audio.status(), OutputStatus::Muted);
    let events = [Discharge {
        id: 1,
        kind: DischargeKind::OneShot,
    }];
    audio.update(&AudioFrame {
        discharges: &events,
        ..frame(1, false)
    });
    audio.set_muted(false);
    audio.update(&AudioFrame {
        discharges: &events,
        ..frame(2, false)
    });
    assert!(state.borrow().played.is_empty());
    audio.update(&frame(3, true));
    assert_eq!(state.borrow().played, [SoundEffect::Dock]);
}

#[test]
fn unavailable_output_does_not_replay_events_if_output_recovers() {
    let (mut audio, state) = presenter();
    state.borrow_mut().failed = true;
    assert_eq!(audio.status(), OutputStatus::Unavailable);
    audio.update(&frame(1, false));
    state.borrow_mut().failed = false;
    audio.update(&frame(2, false));
    assert!(state.borrow().played.is_empty());
}

#[test]
fn continuous_tool_is_one_short_voice_and_needs_a_fresh_event_to_restart() {
    let (mut audio, state) = presenter();
    let events = [
        Discharge {
            id: 1,
            kind: DischargeKind::ContinuousTool,
        },
        Discharge {
            id: 2,
            kind: DischargeKind::ContinuousTool,
        },
    ];
    audio.update(&AudioFrame {
        discharges: &events,
        ..frame(1, true)
    });
    assert_eq!(state.borrow().tool_starts, 1);
    state.borrow_mut().tool = false;
    audio.update(&AudioFrame {
        discharges: &events,
        ..frame(2, true)
    });
    assert_eq!(state.borrow().tool_starts, 1);
    let next = [Discharge {
        id: 3,
        kind: DischargeKind::ContinuousTool,
    }];
    audio.update(&AudioFrame {
        discharges: &next,
        ..frame(3, true)
    });
    assert_eq!(state.borrow().tool_starts, 2);
    audio.begin_session(&AudioFrame {
        epoch: 2,
        ..frame(0, true)
    });
    assert!(!state.borrow().tool);
}

#[test]
fn shutdown_is_terminal_even_for_resumed_frames_and_new_sessions() {
    let (mut audio, state) = presenter();
    audio.update(&frame(1, false));
    audio.shutdown();
    let stops = state.borrow().stops;
    audio.shutdown();
    audio.set_muted(false);
    audio.begin_session(&AudioFrame {
        epoch: 2,
        ..frame(0, true)
    });
    audio.update(&frame(2, true));
    assert_eq!(audio.status(), OutputStatus::Shutdown);
    assert_eq!(state.borrow().stops, stops);
    assert_eq!(state.borrow().played, [SoundEffect::Launch]);
    drop(audio);
    assert_eq!(state.borrow().stops, stops);
}

#[test]
fn dropping_presenter_stops_active_playback() {
    let (mut audio, state) = presenter();
    audio.update(&frame(1, false));
    let stops = state.borrow().stops;
    drop(audio);
    assert_eq!(state.borrow().stops, stops + 1);
}

#[test]
fn journal_keeps_only_the_last_32_accepted_discharge_identities() {
    let mut journal = DischargeJournal::default();
    for _ in 0..40 {
        assert!(journal.record(DischargeKind::OneShot));
    }
    assert_eq!(journal.events().len(), 32);
    assert_eq!(journal.events().first().map(|event| event.id), Some(9));
    assert_eq!(journal.events().last().map(|event| event.id), Some(40));
    assert!(
        journal
            .events()
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
    );
}
