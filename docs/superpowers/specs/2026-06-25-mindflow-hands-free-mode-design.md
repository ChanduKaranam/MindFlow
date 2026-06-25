# MindFlow Hands-Free Recording Mode — Design Spec

*Brainstormed 2026-06-25. Adds a third recording mode: press the activation key once to start dictating, keep talking hands-free, press Enter to finish.*

## Problem / Goal

Hold-to-talk is tiring for long-form dictation, and the existing Toggle mode requires re-pressing the same activation combo to stop. The user wants: **press Ctrl+Space once → keep listening → press Enter to stop** — a hands-free long-form mode with a dedicated, easy finish key.

## Existing modes (context)

Handy's recording behavior is governed by `push_to_talk: bool` and the single-threaded `TranscriptionCoordinator` state machine (`src-tauri/src/transcription_coordinator.rs`):
- **Hold-to-talk** (`push_to_talk = true`, default): activation press starts (from `Idle`), activation release stops.
- **Toggle** (`push_to_talk = false`): activation press starts (from `Idle`), activation press again stops.

Both record the full buffer, then transcribe + insert once stopped. The new mode reuses that lifecycle; only the start/stop triggers differ.

## Decisions (from brainstorming)

1. **New `HandsFree` mode** alongside Hold and Toggle (user picks one in Settings).
2. **Start:** single press of the activation key (Ctrl+Space) — no holding; recording stays on.
3. **Stop:** pressing **Enter** stops recording; Enter is **consumed** (no newline reaches the app); the speech is transcribed + inserted. **Stop-only** — no submit/newline is sent.
4. **Safety stop:** pressing the activation key again while recording *also* stops it (so the user can never get stuck).
5. **Stop key = Enter, hard-coded** for v1. Making the stop key user-configurable is an explicit follow-up, not in scope.
6. `push_to_talk: bool` is **replaced** by a `recording_mode` enum (pre-release app, single user; default `Hold` matches the old default — no migration shim).

## Behavior

| Mode | Activation press (Idle) | Activation press (Recording) | Activation release (Recording) | Enter press (Recording) |
|---|---|---|---|---|
| Hold | Start | — | Stop | — |
| Toggle | Start | Stop | — | — |
| **HandsFree** | **Start** | **Stop (safety)** | — | **Stop** |

In all modes, events during `Processing` are ignored. After any Stop, the existing transcribe → insert pipeline runs unchanged.

## Mechanism

The change is contained to the coordinator + the stop-key registration:

- **Pure decision core (new, testable):** extract the coordinator's branching into a pure function
  `decide(mode: RecordingMode, stage: &Stage, event: InputEvent) -> Decision`
  where `InputEvent ∈ { ActivationPress, ActivationRelease, StopKeyPress }` and `Decision ∈ { Start, Stop, Ignore }`. The coordinator maps each `Command::Input` to an `InputEvent` (by `is_pressed` and whether the binding id is the stop key) and applies the `Decision`. This isolates all mode logic from the Tauri/`ACTION_MAP` side effects so it can be unit-tested.
- **Enter as a dynamic stop-shortcut:** the stop key is *not* always captured. When a HandsFree recording **starts**, the coordinator registers Enter as a global shortcut (binding id e.g. `hands_free_stop`) via `tauri-plugin-global-shortcut`; its handler calls `send_input("hands_free_stop", …, is_pressed: true)`. When recording **stops** (by Enter or activation), the coordinator unregisters Enter. So Enter behaves 100% normally except during an active hands-free dictation, where it stops and is swallowed.
- The coordinator already serialises all lifecycle events through one thread, so register/unregister and start/stop stay race-free.

**Cross-platform caveat (honest):** *consuming* Enter (swallowing the newline) works via global-shortcut registration on **Windows** (the user's platform) and Linux. On macOS the OS restricts capturing a bare modifier-less key like Enter; if registration fails there, the fallback is "Enter still stops recording, but a newline may pass through to the app." The implementation must register defensively and fall back gracefully (never panic / never break Enter) rather than assume capture succeeds.

## Settings & UI

- **Setting:** replace `AppSettings.push_to_talk: bool` with `AppSettings.recording_mode: RecordingMode` where `RecordingMode ∈ { Hold, Toggle, HandsFree }`, default `Hold`. Derive `specta::Type`/serde like the existing enums; serde-default to `Hold`. Update every reader of `push_to_talk` (coordinator call sites in the shortcut/signal handlers) to pass/derive from `recording_mode`. Regenerate `bindings.ts`.
- **UI:** the existing "Push to talk" toggle in Settings becomes a small **"Recording mode"** selector (Hold to talk / Toggle / Hands-free) in the same place, using the existing select/segmented-control idiom. i18n keys for the three labels + a short help line.
- The stop key (Enter) is fixed in v1 — no stop-key UI.

## Testing & success criteria

- **Pure-decision unit tests** (CI): `decide()` returns the correct `Start`/`Stop`/`Ignore` for every (mode × stage × event) combination — covering hold (start-on-press / stop-on-release), toggle (start / stop-on-same-key / ignore-release), and hands-free (start-on-activation-press / stop-on-Enter / stop-on-activation-again / ignore-during-Processing / ignore-release). This is the new mode's real coverage and the bug-prone surface.
- **Settings unit test:** `recording_mode` defaults to `Hold` and round-trips through serde.
- **Manual (real hardware, documented in the PR):** in HandsFree mode — Ctrl+Space → speak → Enter → text inserted with no stray newline; Ctrl+Space-again also stops; Enter behaves normally when not dictating; switching modes in Settings takes effect.

## Risks & mitigations

- **Enter capture fails / OS quirks** → register defensively, fall back to non-consuming stop, never break normal Enter. (macOS only; user is on Windows.)
- **Stuck recording** if the stop shortcut is missed → activation-key-again always stops as a safety net; unregister Enter on every stop path (including cancel) so a stale Enter capture can't linger.
- **Dangling Enter registration** across cancel/app-exit → ensure `Cancel` and shutdown paths also unregister the stop shortcut.

## Global constraints

- CPU-only, fully local. No new heavy deps (reuse `tauri-plugin-global-shortcut`, already present).
- Follow the existing coordinator / binding / settings patterns; don't restructure beyond extracting the testable `decide()` core.
- Builds on current `main` (post noise-suppression merge).

## Out of scope (YAGNI)

- User-configurable stop key / stop-key UI (follow-up).
- Streaming/partial transcription while recording (the pipeline still transcribes on stop).
- Enter "stop & submit" passthrough (the user chose stop-only).
- Auto-stop on silence timeout.
