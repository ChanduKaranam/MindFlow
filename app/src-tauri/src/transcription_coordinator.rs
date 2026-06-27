use crate::actions::ACTION_MAP;
use crate::managers::audio::AudioRecordingManager;
use crate::settings::RecordingMode;
use log::{debug, error, warn};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const DEBOUNCE: Duration = Duration::from_millis(30);

/// Commands processed sequentially by the coordinator thread.
enum Command {
    Input {
        binding_id: String,
        hotkey_string: String,
        is_pressed: bool,
        mode: RecordingMode,
    },
    Cancel {
        recording_was_active: bool,
    },
    ProcessingFinished,
}

/// Pipeline lifecycle, owned exclusively by the coordinator thread.
enum Stage {
    Idle,
    Recording(String), // binding_id
    Processing,
}

/// Serialises all transcription lifecycle events through a single thread
/// to eliminate race conditions between keyboard shortcuts, signals, and
/// the async transcribe-paste pipeline.
pub struct TranscriptionCoordinator {
    tx: Sender<Command>,
}

pub fn is_transcribe_binding(id: &str) -> bool {
    id == "transcribe" || id == "transcribe_with_post_process"
}

impl TranscriptionCoordinator {
    pub fn new(app: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut stage = Stage::Idle;
                let mut last_press: Option<Instant> = None;

                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        Command::Input {
                            binding_id,
                            hotkey_string,
                            is_pressed,
                            mode,
                        } => {
                            // Debounce rapid-fire press events (key repeat / double-tap).
                            // Releases always pass through.
                            if is_pressed {
                                let now = Instant::now();
                                if last_press.map_or(false, |t| now.duration_since(t) < DEBOUNCE) {
                                    debug!("Debounced press for '{binding_id}'");
                                    continue;
                                }
                                last_press = Some(now);
                            }

                            let event = if binding_id == "hands_free_stop" {
                                InputEvent::StopKeyPress
                            } else if is_pressed {
                                InputEvent::ActivationPress
                            } else {
                                InputEvent::ActivationRelease
                            };
                            let stage_kind = match &stage {
                                Stage::Idle => StageKind::Idle,
                                Stage::Processing => StageKind::Processing,
                                Stage::Recording(id) if id == &binding_id => StageKind::RecordingThis,
                                Stage::Recording(_) => StageKind::RecordingOther,
                            };
                            match decide(mode, stage_kind, event) {
                                Decision::Start => {
                                    start(&app, &mut stage, &binding_id, &hotkey_string);
                                    if mode == RecordingMode::HandsFree && matches!(stage, Stage::Recording(_)) {
                                        crate::shortcut::register_handsfree_stop_shortcut(&app);
                                    }
                                }
                                Decision::Stop => {
                                    // For a StopKeyPress the active binding lives in `stage`, not `binding_id`.
                                    let active = match &stage { Stage::Recording(id) => id.clone(), _ => binding_id.clone() };
                                    // Unconditionally unregister the hands-free Enter shortcut on every stop.
                                    // It is idempotent (a no-op when not registered), and gating on the
                                    // current mode leaked the Enter capture if the user changed recording mode
                                    // mid-recording and then stopped via the activation hotkey.
                                    crate::shortcut::unregister_handsfree_stop_shortcut(&app);
                                    stop(&app, &mut stage, &active, &hotkey_string);
                                }
                                Decision::Ignore => {}
                            }
                        }
                        Command::Cancel {
                            recording_was_active,
                        } => {
                            // Don't reset during processing — wait for the pipeline to finish.
                            if !matches!(stage, Stage::Processing)
                                && (recording_was_active || matches!(stage, Stage::Recording(_)))
                            {
                                crate::shortcut::unregister_handsfree_stop_shortcut(&app);
                                stage = Stage::Idle;
                            }
                        }
                        Command::ProcessingFinished => {
                            stage = Stage::Idle;
                        }
                    }
                }
                // On normal app shutdown the OS releases all global shortcuts as part of
                // process teardown, so the hands-free Enter shortcut is implicitly freed
                // here. A future refactor adding in-process graceful shutdown must call
                // unregister_handsfree_stop_shortcut() explicitly to preserve the
                // "Enter never lingers after recording stops" invariant.
                debug!("Transcription coordinator exited");
            }));
            if let Err(e) = result {
                error!("Transcription coordinator panicked: {e:?}");
            }
        });

        Self { tx }
    }

    /// Send a keyboard/signal input event for a transcribe binding.
    /// Pass the current `RecordingMode` so the coordinator can route via `decide()`.
    /// For signal-based events use `is_pressed: true` and `mode: RecordingMode::Toggle`.
    pub fn send_input(
        &self,
        binding_id: &str,
        hotkey_string: &str,
        is_pressed: bool,
        mode: RecordingMode,
    ) {
        if self
            .tx
            .send(Command::Input {
                binding_id: binding_id.to_string(),
                hotkey_string: hotkey_string.to_string(),
                is_pressed,
                mode,
            })
            .is_err()
        {
            warn!("Transcription coordinator channel closed");
        }
    }

    pub fn notify_cancel(&self, recording_was_active: bool) {
        if self
            .tx
            .send(Command::Cancel {
                recording_was_active,
            })
            .is_err()
        {
            warn!("Transcription coordinator channel closed");
        }
    }

    pub fn notify_processing_finished(&self) {
        if self.tx.send(Command::ProcessingFinished).is_err() {
            warn!("Transcription coordinator channel closed");
        }
    }
}

fn start(app: &AppHandle, stage: &mut Stage, binding_id: &str, hotkey_string: &str) {
    let Some(action) = ACTION_MAP.get(binding_id) else {
        warn!("No action in ACTION_MAP for '{binding_id}'");
        return;
    };
    action.start(app, binding_id, hotkey_string);
    if app
        .try_state::<Arc<AudioRecordingManager>>()
        .map_or(false, |a| a.is_recording())
    {
        *stage = Stage::Recording(binding_id.to_string());
    } else {
        debug!("Start for '{binding_id}' did not begin recording; staying idle");
    }
}

fn stop(app: &AppHandle, stage: &mut Stage, binding_id: &str, hotkey_string: &str) {
    let Some(action) = ACTION_MAP.get(binding_id) else {
        warn!("No action in ACTION_MAP for '{binding_id}'");
        return;
    };
    action.stop(app, binding_id, hotkey_string);
    *stage = Stage::Processing;
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InputEvent {
    ActivationPress,
    ActivationRelease,
    StopKeyPress,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StageKind {
    Idle,
    RecordingThis,
    RecordingOther,
    Processing,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Decision {
    Start,
    Stop,
    Ignore,
}

/// Pure recording-lifecycle decision. No side effects — fully unit-tested.
pub fn decide(mode: RecordingMode, stage: StageKind, event: InputEvent) -> Decision {
    use Decision::*;
    use InputEvent::*;
    use RecordingMode::*;
    use StageKind::*;

    if matches!(stage, Processing) {
        return Ignore;
    }
    match (mode, stage, event) {
        // Start: an activation press while idle, in any mode.
        (_, Idle, ActivationPress) => Start,

        // Hold: stop on release of the active binding.
        (Hold, RecordingThis, ActivationRelease) => Stop,

        // Toggle: stop on a second press of the active binding.
        (Toggle, RecordingThis, ActivationPress) => Stop,

        // Hands-free: Enter stops whatever is recording; activation press is the safety stop.
        // Note: in practice a StopKeyPress always has binding_id == "hands_free_stop", which
        // never matches the active recording's id, so the coordinator classifies it as
        // RecordingOther rather than RecordingThis. This arm is defensive symmetry — it
        // ensures Stop is returned even in any future path where the ids could match.
        (HandsFree, RecordingThis, StopKeyPress) => Stop,
        (HandsFree, RecordingOther, StopKeyPress) => Stop,
        (HandsFree, RecordingThis, ActivationPress) => Stop,

        _ => Ignore,
    }
}

#[cfg(test)]
mod decide_tests {
    use super::*;
    use crate::settings::RecordingMode::*;

    #[test]
    fn hold_mode() {
        assert_eq!(decide(Hold, StageKind::Idle, InputEvent::ActivationPress), Decision::Start);
        assert_eq!(decide(Hold, StageKind::RecordingThis, InputEvent::ActivationRelease), Decision::Stop);
        // press while recording does nothing in hold; release in idle does nothing
        assert_eq!(decide(Hold, StageKind::RecordingThis, InputEvent::ActivationPress), Decision::Ignore);
        assert_eq!(decide(Hold, StageKind::Idle, InputEvent::ActivationRelease), Decision::Ignore);
    }

    #[test]
    fn toggle_mode() {
        assert_eq!(decide(Toggle, StageKind::Idle, InputEvent::ActivationPress), Decision::Start);
        assert_eq!(decide(Toggle, StageKind::RecordingThis, InputEvent::ActivationPress), Decision::Stop);
        // releases are ignored in toggle; a different binding doesn't stop this one
        assert_eq!(decide(Toggle, StageKind::RecordingThis, InputEvent::ActivationRelease), Decision::Ignore);
        assert_eq!(decide(Toggle, StageKind::RecordingOther, InputEvent::ActivationPress), Decision::Ignore);
    }

    #[test]
    fn hands_free_mode() {
        assert_eq!(decide(HandsFree, StageKind::Idle, InputEvent::ActivationPress), Decision::Start);
        // Enter stops whatever is recording
        assert_eq!(decide(HandsFree, StageKind::RecordingThis, InputEvent::StopKeyPress), Decision::Stop);
        assert_eq!(decide(HandsFree, StageKind::RecordingOther, InputEvent::StopKeyPress), Decision::Stop);
        // activation press again is the safety stop
        assert_eq!(decide(HandsFree, StageKind::RecordingThis, InputEvent::ActivationPress), Decision::Stop);
        // releases ignored
        assert_eq!(decide(HandsFree, StageKind::RecordingThis, InputEvent::ActivationRelease), Decision::Ignore);
    }

    #[test]
    fn processing_always_ignores() {
        for m in [Hold, Toggle, HandsFree] {
            for e in [InputEvent::ActivationPress, InputEvent::ActivationRelease, InputEvent::StopKeyPress] {
                assert_eq!(decide(m, StageKind::Processing, e), Decision::Ignore);
            }
        }
    }

    #[test]
    fn stop_key_only_acts_in_hands_free() {
        assert_eq!(decide(Hold, StageKind::RecordingThis, InputEvent::StopKeyPress), Decision::Ignore);
        assert_eq!(decide(Toggle, StageKind::RecordingThis, InputEvent::StopKeyPress), Decision::Ignore);
    }
}
