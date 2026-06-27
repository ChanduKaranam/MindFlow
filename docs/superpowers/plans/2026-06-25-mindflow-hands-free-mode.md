# MindFlow Hands-Free Recording Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a third recording mode — press the activation key once to start, talk hands-free, press Enter to stop (Enter consumed, stop-only).

**Architecture:** Replace the `push_to_talk: bool` setting with a `RecordingMode` enum (Hold/Toggle/HandsFree). Extract the coordinator's start/stop branching into a pure, unit-tested `decide()` function. The Enter stop-key reuses Handy's existing dynamic-shortcut machinery (an exact mirror of the `cancel`/`escape` binding): a `hands_free_stop` binding bound to `enter`, registered when a HandsFree recording starts and unregistered on stop, dispatched by id to stop the active recording.

**Tech Stack:** Rust, Tauri 2, `tauri-plugin-global-shortcut` (already present), `handy-keys` (already present), React + i18next.

## Global Constraints

- CPU-only, fully local; no new dependencies.
- **Mirror the existing `cancel` binding pattern exactly** for the `hands_free_stop` binding — same dynamic register/unregister, same static-registration exclusion, same `handler.rs` dispatch shape. Do not invent a new shortcut mechanism.
- Stop key is `enter`, **hard-coded** (no stop-key config UI — out of scope).
- `push_to_talk: bool` is **replaced** by `recording_mode: RecordingMode` (pre-release, single user; default `Hold` == old default). Update every reader of `push_to_talk`.
- The coordinator stays single-threaded; all register/unregister happens on its lifecycle transitions (or the handler), never concurrently.
- Enter is registered ONLY during an active HandsFree recording; unregister on every stop path including `Cancel` and shutdown, so no Enter capture lingers.
- All user-facing strings use i18next `t(...)`.
- Backend under `app/src-tauri/`; frontend under `app/src/`. Bash CWD `app/src-tauri` unless noted.

---

### Task 1: `RecordingMode` enum + `hands_free_stop` binding (settings)

**Files:**
- Modify: `app/src-tauri/src/settings.rs` (replace `push_to_talk`; add enum + default binding)
- Modify: `app/src/bindings.ts` (regenerate)
- Test: inline `#[cfg(test)]` in `settings.rs`

**Interfaces:**
- Produces: `pub enum RecordingMode { Hold, Toggle, HandsFree }` (derives `Serialize, Deserialize, Debug, Clone, Copy, PartialEq, specta::Type`; serde rename_all lowercase), `AppSettings.recording_mode: RecordingMode` (default `Hold`), and a default `bindings["hands_free_stop"]` `ShortcutBinding` bound to `enter`.

- [ ] **Step 1: Write the failing tests** (add a test module in `settings.rs`):

```rust
#[cfg(test)]
mod recording_mode_tests {
    use super::*;

    #[test]
    fn default_recording_mode_is_hold() {
        assert_eq!(default_recording_mode(), RecordingMode::Hold);
    }

    #[test]
    fn recording_mode_round_trips() {
        #[derive(serde::Deserialize)]
        struct Probe {
            #[serde(default = "default_recording_mode")]
            recording_mode: RecordingMode,
        }
        let p: Probe = serde_json::from_str(r#"{"recording_mode":"hands_free"}"#).unwrap();
        assert_eq!(p.recording_mode, RecordingMode::HandsFree);
        let p2: Probe = serde_json::from_str("{}").unwrap();
        assert_eq!(p2.recording_mode, RecordingMode::Hold);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd app/src-tauri && cargo test --lib recording_mode 2>&1 | tail -10`
Expected: FAIL — `RecordingMode` / `default_recording_mode` not found.

- [ ] **Step 3: Add the enum + default fn**

Near the other enums in `settings.rs`:

```rust
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Type)]
#[serde(rename_all = "snake_case")]
pub enum RecordingMode {
    Hold,
    Toggle,
    HandsFree,
}

fn default_recording_mode() -> RecordingMode {
    RecordingMode::Hold
}
```

(Confirm `"hands_free"` is the snake_case serialization the test expects; `HandsFree` → `hands_free` under `rename_all = "snake_case"`.)

- [ ] **Step 4: Replace the `push_to_talk` field**

In `AppSettings`, replace `pub push_to_talk: bool,` (settings.rs:340) with:

```rust
    #[serde(default = "default_recording_mode")]
    pub recording_mode: RecordingMode,
```

In `get_default_settings()` (the struct literal ~settings.rs:783), replace `push_to_talk: true,` with `recording_mode: RecordingMode::Hold,`.

- [ ] **Step 5: Add the `hands_free_stop` default binding**

In `get_default_settings()` where the `cancel` binding is inserted (settings.rs ~773), add an identical-shaped insert after it:

```rust
    bindings.insert(
        "hands_free_stop".to_string(),
        ShortcutBinding {
            id: "hands_free_stop".to_string(),
            name: "Hands-free stop".to_string(),
            description: "Stops a hands-free recording.".to_string(),
            default_binding: "enter".to_string(),
            current_binding: "enter".to_string(),
        },
    );
```

- [ ] **Step 6: Build, fix remaining `push_to_talk` references to compile**

Run: `cd app/src-tauri && cargo check 2>&1 | tail -25`. The compiler will flag every `push_to_talk` reader (`shortcut/handler.rs`, `signal_handle.rs`, `shortcut/mod.rs`). Those are migrated in Tasks 3-4; for THIS task, make the crate compile by the minimal change at each site that preserves current behavior — `settings.recording_mode == RecordingMode::Hold` wherever a `push_to_talk` bool was read, and the setter at `shortcut/mod.rs:478` set `recording_mode` from the bool (`if enabled { Hold } else { Toggle }`). (Tasks 3-4 replace these with mode-aware logic.) Run the tests:

Run: `cd app/src-tauri && cargo test --lib recording_mode 2>&1 | tail -10` → PASS.

- [ ] **Step 7: Regenerate bindings**

Regenerate `app/src/bindings.ts` (run the app via cargo as prior tasks did; tauri-specta exports on build). Confirm `RecordingMode` and `recording_mode?: RecordingMode` appear in `AppSettings`. If headless can't regenerate, hand-add: `export type RecordingMode = "hold" | "toggle" | "hands_free"` and `recording_mode?: RecordingMode` to `AppSettings`, matching existing style. Note which in the report.

- [ ] **Step 8: Commit**

```bash
git add app/src-tauri/src/settings.rs app/src/bindings.ts
git commit -m "feat(hands-free): add RecordingMode enum and hands_free_stop binding"
```

---

### Task 2: Pure `decide()` decision core

**Files:**
- Modify: `app/src-tauri/src/transcription_coordinator.rs` (add the pure function + types)
- Test: inline `#[cfg(test)]` in `transcription_coordinator.rs`

**Interfaces:**
- Consumes: `RecordingMode` (Task 1).
- Produces:
  - `pub enum InputEvent { ActivationPress, ActivationRelease, StopKeyPress }`
  - `pub enum StageKind { Idle, RecordingThis, RecordingOther, Processing }`
  - `pub enum Decision { Start, Stop, Ignore }`
  - `pub fn decide(mode: RecordingMode, stage: StageKind, event: InputEvent) -> Decision`

- [ ] **Step 1: Write the failing tests**

```rust
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
```

- [ ] **Step 2: Run to verify failure**

Run: `cd app/src-tauri && cargo test --lib decide_tests 2>&1 | tail -10` → FAIL (`decide` not found).

- [ ] **Step 3: Implement the types + `decide`**

```rust
use crate::settings::RecordingMode;

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
        (HandsFree, RecordingThis, StopKeyPress) => Stop,
        (HandsFree, RecordingOther, StopKeyPress) => Stop,
        (HandsFree, RecordingThis, ActivationPress) => Stop,

        _ => Ignore,
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd app/src-tauri && cargo test --lib decide_tests 2>&1 | tail -10` → PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/transcription_coordinator.rs
git commit -m "feat(hands-free): add pure decide() recording-mode decision core"
```

---

### Task 3: `hands_free_stop` dynamic shortcut (mirror of `cancel`)

**Files:**
- Modify: `app/src-tauri/src/shortcut/mod.rs` (dispatcher fns)
- Modify: `app/src-tauri/src/shortcut/tauri_impl.rs` (register/unregister + static-exclusion)
- Modify: `app/src-tauri/src/shortcut/handy_keys.rs` (register/unregister + static-exclusion)
- Test: `cargo check` (registration is integration-level; behavior covered by Task 2's `decide` + manual)

**Interfaces:**
- Produces: `shortcut::register_handsfree_stop_shortcut(app)` and `shortcut::unregister_handsfree_stop_shortcut(app)`, mirroring `register_cancel_shortcut`/`unregister_cancel_shortcut`. Consumed by Task 4. This task adds ONLY the registration infrastructure (the functions compile as `pub`, callable but not yet called); the `handler.rs` dispatch and all coordinator wiring live in Task 4, so this task builds standalone with no signature coupling.

- [ ] **Step 1: Mirror the cancel register/unregister in `mod.rs`**

Add next to `register_cancel_shortcut`/`unregister_cancel_shortcut` (mod.rs:60-76) the same shape for `hands_free_stop`:

```rust
pub fn register_handsfree_stop_shortcut(app: &AppHandle) {
    let settings = get_settings(app);
    match settings.keyboard_implementation {
        KeyboardImplementation::Tauri => tauri_impl::register_handsfree_stop_shortcut(app),
        KeyboardImplementation::HandyKeys => handy_keys::register_handsfree_stop_shortcut(app),
    }
}

pub fn unregister_handsfree_stop_shortcut(app: &AppHandle) {
    let settings = get_settings(app);
    match settings.keyboard_implementation {
        KeyboardImplementation::Tauri => tauri_impl::unregister_handsfree_stop_shortcut(app),
        KeyboardImplementation::HandyKeys => handy_keys::unregister_handsfree_stop_shortcut(app),
    }
}
```

- [ ] **Step 2: tauri_impl — exclude from static reg + add register/unregister**

In `tauri_impl.rs`, find the static-registration loop that skips `cancel` (the `if id == "cancel" { continue; }` at ~line 23) and also skip `hands_free_stop`:

```rust
if id == "cancel" || id == "hands_free_stop" {
    continue; // registered dynamically
}
```

Add `register_handsfree_stop_shortcut`/`unregister_handsfree_stop_shortcut` as exact copies of the cancel versions (tauri_impl.rs:158-196) but looking up `bindings.get("hands_free_stop")` instead of `"cancel"`.

- [ ] **Step 3: handy_keys — same exclusion + register/unregister**

In `handy_keys.rs`, mirror the same two changes: extend the static-skip (~line 433 `if id == "cancel"`) to also skip `hands_free_stop`, and add `register_handsfree_stop_shortcut`/`unregister_handsfree_stop_shortcut` copying the cancel versions (handy_keys.rs:461-500) with `"hands_free_stop"`.

- [ ] **Step 4: Verify it compiles**

Run: `cd app/src-tauri && cargo check 2>&1 | tail -15`
Expected: compiles clean, no new warnings. The two new `pub fn`s are not yet called (Task 4 calls them) — `pub` functions in this lib crate do not trigger dead-code warnings. (The `handler.rs` dispatch of the `hands_free_stop` binding is intentionally deferred to Task 4, where `send_input`'s signature changes, so this task has no cross-task signature coupling.)

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/shortcut/mod.rs app/src-tauri/src/shortcut/tauri_impl.rs \
  app/src-tauri/src/shortcut/handy_keys.rs
git commit -m "feat(hands-free): add Enter dynamic stop-shortcut registration (mirror of cancel)"
```

---

### Task 4: Wire the coordinator (use `decide`, register Enter on hands-free start)

**Files:**
- Modify: `app/src-tauri/src/transcription_coordinator.rs` (Command, send_input, the loop, start/stop)
- Modify: `app/src-tauri/src/shortcut/handler.rs` (pass `recording_mode`)
- Modify: `app/src-tauri/src/signal_handle.rs` (pass `RecordingMode::Toggle`)
- Test: `cargo test --lib` (the `decide` tests already cover the logic; this task is wiring) + `cargo check`

**Interfaces:**
- Consumes: `decide()`, `RecordingMode`, `register_handsfree_stop_shortcut`/`unregister_handsfree_stop_shortcut`.

- [ ] **Step 1: Change `Command::Input` and `send_input` to carry `RecordingMode`**

Replace the `push_to_talk: bool` field in `Command::Input` and the `send_input(..., push_to_talk: bool)` parameter with `mode: RecordingMode`. Update `signal_handle.rs:18` to pass `RecordingMode::Toggle` (signal toggles are toggle-style), and the existing transcribe-binding call in `handler.rs` (`coordinator.send_input(binding_id, hotkey_string, is_pressed, settings.push_to_talk)`) to pass `settings.recording_mode`.

- [ ] **Step 1b: Dispatch the `hands_free_stop` binding in handler.rs**

In `handle_shortcut_event` (handler.rs), add — before the generic `ACTION_MAP` lookup (there is no action for `hands_free_stop`), near the `cancel` handling — forwarding of the stop key to the coordinator (it maps to `StopKeyPress`):

```rust
// Hands-free stop key (e.g. Enter): forward to the coordinator on press.
if binding_id == "hands_free_stop" {
    if is_pressed {
        if let Some(coordinator) = app.try_state::<TranscriptionCoordinator>() {
            coordinator.send_input(binding_id, hotkey_string, true, settings.recording_mode);
        }
    }
    return;
}
```

- [ ] **Step 2: Replace the branching in the coordinator loop with `decide()`**

In the `Command::Input` arm, compute the `InputEvent` and `StageKind`, call `decide`, and apply:

```rust
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
        if mode == RecordingMode::HandsFree {
            crate::shortcut::unregister_handsfree_stop_shortcut(&app);
        }
        stop(&app, &mut stage, &active, &hotkey_string);
    }
    Decision::Ignore => {}
}
```

Keep the existing debounce on presses. (The `mode` is the value carried by this `Command::Input`.)

- [ ] **Step 3: Unregister Enter on the Cancel path too**

In the `Command::Cancel` arm, when a recording is being cancelled, also call `crate::shortcut::unregister_handsfree_stop_shortcut(&app)` so a hands-free Enter capture never lingers after a cancel. (Safe to call even if not registered — mirror how cancel-unregister is already idempotent.)

- [ ] **Step 4: Build + tests**

Run: `cd app/src-tauri && cargo check 2>&1 | tail -15 && cargo test --lib 2>&1 | tail -8`
Expected: clean build, all tests pass (incl. `decide_tests`, `recording_mode_tests`).

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/transcription_coordinator.rs app/src-tauri/src/shortcut/handler.rs app/src-tauri/src/signal_handle.rs
git commit -m "feat(hands-free): drive coordinator via decide() and register Enter on hands-free start"
```

- [ ] **Step 6: Manual verification (real hardware, documented in PR)**

In HandsFree mode: Ctrl+Space → speak → Enter → text inserted, no stray newline; Ctrl+Space-again also stops; Enter normal when not dictating; Escape still cancels.

---

### Task 5: Settings UI — recording-mode selector

**Files:**
- Modify: the settings component rendering the push-to-talk control (under `app/src/components/settings/`)
- Modify: `app/src/i18n/locales/en/translation.json`
- Test: `bun run lint` + manual

**Interfaces:**
- Consumes: `AppSettings.recording_mode` (Task 1) via the settings hook/store.

- [ ] **Step 1: Find the push-to-talk control** — search `app/src/components/settings/` for `push_to_talk`. Read that component and how it reads/writes the setting via `useSettings`.

- [ ] **Step 2: Replace the toggle with a 3-option selector** bound to `recording_mode`, reusing the existing select/segmented idiom in that settings area (match a sibling dropdown's component + props):

```tsx
const mode = getSetting("recording_mode") ?? "hold";
// Options: hold | toggle | hands_free — labels via t(...)
<SettingSelect
  label={t("settings.recordingMode.title")}
  description={t("settings.recordingMode.description")}
  value={mode}
  options={[
    { value: "hold", label: t("settings.recordingMode.hold") },
    { value: "toggle", label: t("settings.recordingMode.toggle") },
    { value: "hands_free", label: t("settings.recordingMode.handsFree") },
  ]}
  onChange={(v) => updateSetting("recording_mode", v)}
/>
```

Match the actual select component used by neighbors (e.g. the sound-theme or language dropdown); do not invent one.

- [ ] **Step 3: Add i18n strings** to `app/src/i18n/locales/en/translation.json`:

```json
"recordingMode": {
  "title": "Recording mode",
  "description": "How the activation shortcut controls recording.",
  "hold": "Hold to talk",
  "toggle": "Toggle (press to start and stop)",
  "handsFree": "Hands-free (press to start, Enter to stop)"
}
```

- [ ] **Step 4: Lint**

Run: `cd app && bun run lint 2>&1 | tail -15`
Expected: no NEW errors (pre-existing `DevInject.tsx` errors, if any, are in an untouched file).

- [ ] **Step 5: Commit**

```bash
git add app/src/components/settings app/src/i18n/locales/en/translation.json
git commit -m "feat(hands-free): add recording-mode selector to settings"
```

---

## Self-Review

**Spec coverage:**
- New HandsFree mode (start on press, Enter stop, activation-again safety) → Task 2 (`decide`) + Task 4 (wiring). ✓
- Enter consumed / stop-only → Task 3 (dynamic registration consumes the key; no submit sent). ✓
- `push_to_talk` → `RecordingMode` enum → Task 1 + Task 4 (call-site migration). ✓
- Enter via dynamic shortcut mirroring cancel → Task 3. ✓
- Unregister on every stop incl. cancel → Task 4 Steps 2-3. ✓
- Pure testable decision core → Task 2. ✓
- UI selector → Task 5. ✓
- Stop key hard-coded `enter`, no stop-key UI → Tasks 1/5 (no config surface). ✓

**Placeholder scan:** No TBD/TODO. Task 3 was scoped to registration infrastructure only (no `handler.rs`/`send_input` references) so it compiles standalone; all signature-coupled changes (handler dispatch + `send_input` mode param + coordinator) are consolidated in Task 4, which compiles as a unit. No cross-task compile coupling remains.

**Type consistency:** `RecordingMode { Hold, Toggle, HandsFree }` (Task 1) is used identically in `decide()` (Task 2), the coordinator (Task 4), and the UI values `hold|toggle|hands_free` (Task 5, matching snake_case serde). `decide(mode, stage, event)` signature is identical between Task 2's definition and Task 4's call. `register_handsfree_stop_shortcut`/`unregister_handsfree_stop_shortcut` (Task 3) are called with those exact names in Task 4. Binding id `"hands_free_stop"` and accelerator `"enter"` are consistent across Tasks 1, 3, 4.
