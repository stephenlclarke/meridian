# Original generated audio port

Author-stage implementation. Rust tests and native audio are UNRUN until the joined source is reviewed and the existing executor runs it. This module is not yet registered or wired into App. It does not change the current preview.

## Frozen source

The reference is Elite/L337 commit `a11fe16924d87d06021816f9037952afcbac08db`, tree `68fec574df12ddf74acb14747e0dd6839f478642`. Relevant files are `Sources/L337Mac/AudioPresenter.swift` and `Sources/L337Core/{Sound,DischargeAudio,SpeechFeed}.swift`, plus the real missile notification in Game.swift. Preserve `assets/legacy/Elite-L337-MIT.txt`. All tones are independently synthesized; no imported Elite, Oolite or recorded sound assets are used.

The six source waveforms preserve mono PCM16 at 22,050 Hz, truncating sample count/envelope, square phase, wrapping noise seed and 0.4 playback volume. Reference fixtures are an independent Python transcription of the frozen formulas, not output from an executed Swift oracle. Their manifest records this limit. Playback converts the PCM16 samples to float without changing the reference generator. Device resampling and audible equivalence still require actual platform checks.

## Root integration binding

Root owns App, main and serial Cargo integration. Add `pub mod audio;` to the desktop crate root, then own one `AudioPresenter<NativeOutput>` on the desktop owner. `NativeOutput::open()` is the only device-open operation. It requires no recording or decoder feature. Keep the presenter out of World and save data.

1. After successful New or Continue, allocate a strictly increasing, process-local presentation epoch and call `begin_session` with the actual restored/current facts. A failed load must not rotate the epoch. Prime before accepting live flight commands. Never save or reuse the epoch across live session replacement.
2. Feed immutable `AudioFrame` values with a monotonically increasing host presentation revision. Increase revision when simulation or host presentation facts change; repeat render frames may reuse it. Older and equal revisions cannot rewind docking edges or event cursors. Rotate to a new epoch before revision or event-ID exhaustion. Do not sort or reconstruct old snapshots as new revisions.
3. Set `docked` only from the actual `FlightMode::Docked`. Launching and Flight are both undocked. An actual launch transition gives Launch; successful physical docking gives Dock. Priming a flying Continue gives neither.
4. Record one monotonically increasing discharge ID only after each successful `request_fire_front_pulse` result. A held fire key, retained PulseShot, hit, pulse TTL or render frame is not another discharge. Use DischargeJournal to keep the last 32 accepted discharges, as in the source. Its record method refuses ID exhaustion without wrapping; rotate the host epoch and replace the journal before continuing. Do not generate sound from failed/refused fire. The frame borrows this ring; the presenter retains only its high-water cursor.
5. The source missile launch has no bespoke missile sound. Its actual `MISSILE AWAY` notification gives Ping. Bind the real accepted missile-launch message once and route actual current notification text/liveness through the same message edge. Do not play a second direct missile tone, narrate cold restored missiles, or fabricate kills or ECM activations.
6. OneShot is the actual front Pulse path. ContinuousTool preserves the source cutter's one short Laser voice at a time and only restarts on another fresh event after completion. It is a presentation behavior, not implemented salvage/mining hardware. Do not wire unsupported tool gameplay.
7. Expose `status()` as Ready, Muted, Unavailable or Shutdown. `set_muted` stops current voices; muted and unavailable updates still consume events. Default-output failure leaves gameplay running and does not silently reopen another device. No reconnect feature is claimed.
8. Call `shutdown()` before window/application teardown, including any suspended frame or replacement host path. It stops voices and releases the device. Later frames, New, Continue and unmute cannot resurrect this presenter. Construct a new presenter only for a new app lifetime, not to evade shutdown.

Only small event cursors, prior dock state and the latest notification text are retained. Audio does not own a World, mission state or another simulation clock. The device error callback can set only an atomic flag. It cannot call the host or retain the presenter.

## Source behavior deliberately not claimed

Rust has no genuine speech-enabled commander state, ECM announcement stream or platform voice adapter. The frozen SpeechFeed consumes disabled/busy events, maintains a bounded 16-epoch LRU, clears departed queued speech, and prevents replay. This packet does not invent those gameplay inputs, a TTS voice, kills or announcements. Speech/TTS and unsupported tool/ECM/kill triggers remain a later source-grounded integration task. Explosion and ECM are pure reference waveform variants only.

## Qualification criteria

The authored tests cover complete reference WAV bytes for all six tones, actual docking edges, retained/stale revisions, cold-session priming, fresh discharge identity, missile notification identity, expired notifications, mute/unavailable consumption, one continuous tool voice, terminal shutdown and drop. They are not passing evidence until run from the registered/joined source.

After G2 reviews the coherent joined source, the designated executor must run focused audio tests, formatting and strict Clippy, then the current required CI for the combined batch. Execute native launch/dock/front Pulse/actual missile notification, mute, output-unavailable and normal-close checks with exact source/process evidence on supported targets. Check audible/recorded wave behavior separately from synthetic fakes. Native device/race/leak/performance and full speech parity remain unverified here. Preserve the existing unresolved XPC leak finding; this authoring does not cure it.

## Dependencies and distribution

The exact new direct dependency is Rodio 0.23.0 with default features disabled and playback enabled. It uses CPAL 0.18.2. Linux builds require ALSA development headers and pkg-config from the actual host package manager. No platform build is claimed from metadata resolution.

The author packet contains the exact manifest/lock delta and official checksum-verified archives for all 15 new resolved packages on the base. Ten archives contain notice files; dasp_sample and four objc2 framework archives do not. The packet retains exact archive-pinned upstream notices for those five, including the objc2 Apple SDK caveat, plus separately identified canonical MIT terms. Packaging must carry those bindings and the original archive-absence facts. This is an explicit archive fact, not a waiver or blanket legal clearance. Root must resolve the final union lock once alongside the geometry owner's pinned num-* dependencies and regenerate target-specific dependency/notice manifests for the actual final lock. No old 80-dependency release notice set covers this new graph.
