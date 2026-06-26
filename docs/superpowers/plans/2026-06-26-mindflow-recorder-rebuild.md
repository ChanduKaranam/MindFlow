# Recorder Rebuild + Audio-Settings Persistence — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Mic-Sensitivity (`vad_threshold`) and Noise-Suppression (`noise_suppression`) settings actually take effect — persist them to the backend (they currently don't at all) and rebuild the cached audio recorder so the change applies immediately, without an app restart.

**Architecture:** Two new Tauri commands persist the settings and then call a new `AudioRecordingManager::rebuild_recorder()` which drops the cached recorder and rebuilds it from current settings — guarded to run only when idle (never mid-recording, which would kill the live cpal stream), and restarting the mic stream if it was open (always-on mode). Frontend `settingsStore` updaters + hand-edited bindings route the slider/toggle to the new commands.

**Tech Stack:** Rust (Tauri 2 backend, `cargo test --lib`), tauri-specta bindings (hand-edited), React/TypeScript (`settingsStore`).

## Background (verified in code)

- The recorder is cached in `self.recorder: Arc<Mutex<Option<AudioRecorder>>>` and built once in `preload_vad()` (`managers/audio.rs:300`), only when `recorder_opt.is_none()`. It is **never reset to `None`** anywhere.
- `create_audio_recorder()` (`audio.rs:126`) bakes `vad_threshold` (into `SileroV6Vad::new`) and `noise_suppression` (whether the GTCRN denoiser is attached) into the recorder at construction. These are **constructor parameters**, so they only change on rebuild.
- `noise_suppression` and `vad_threshold` have **no backend command** and **no `settingsStore` updater** — the UI updates Zustand state only and the generic fallback `console.warn`s without persisting (`settingsStore.ts` ~line 288–299). The fields already exist in `AppSettings` (Rust + `bindings.ts`).
- `update_selected_device()` (`audio.rs:461`) is the proven safe-restart precedent: when `is_open`, it bumps `close_generation`, `stop_microphone_stream()`, then `start_microphone_stream()`. Device change works without restart because the device is an `.open()` arg, not a constructor arg.
- Recording state: `self.state: Arc<Mutex<RecordingState>>` (`Idle | Recording { binding_id }`), and `self.is_recording: Arc<Mutex<bool>>`. `self.is_open: Arc<Mutex<bool>>` tracks the cpal stream. `start_microphone_stream()` calls `preload_vad()` (so a `None` recorder rebuilds on next open) but returns early if `is_open` is already true.

## Global Constraints

- CPU-only, fully local, zero network. **No new dependencies.**
- The rebuild MUST be safe: **never drop/rebuild the recorder while recording** (dropping `AudioRecorder` stops the live cpal stream → lost audio). Rebuild only when `state == Idle`.
- No lock-ordering deadlocks: read `is_open`/`state`/`is_recording` into locals (releasing the temporary guard) before calling `stop_microphone_stream`/`start_microphone_stream` (which lock those fields themselves).
- New commands mirror existing `change_*`/`update_*` command style (`#[tauri::command] #[specta::specta]`, `get_settings → mutate → write_settings`, `Result<(), String>`), registered in `lib.rs` `collect_commands![`.
- Bindings `app/src/bindings.ts` hand-edited (headless regen unavailable). The `AppSettings` fields already exist — only add the two command methods.
- `settingsStore` REQUIRES a dedicated updater per key (generic fallback does not persist).
- Conventional commits; body trailer exactly: `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.
- Backend tests from `app/src-tauri`: `cargo test --lib`. Frontend from `app`: `bun run tsc`, `bun run lint` (must be clean — the prior DevInject errors are now fixed).
- **Testing reality:** the rebuild path requires real audio hardware/models and cannot be meaningfully unit-tested. The gate for the audio logic is: it compiles, the existing suite passes, and a documented manual verification. Do NOT write a fake/mock test that asserts nothing.

## File Structure

- **Modify** `app/src-tauri/src/managers/audio.rs` — add `pub fn rebuild_recorder(&self) -> Result<(), anyhow::Error>`.
- **Modify** `app/src-tauri/src/shortcut/mod.rs` — add `change_noise_suppression_setting` + `change_vad_threshold_setting` commands.
- **Modify** `app/src-tauri/src/lib.rs` — register both commands.
- **Modify** `app/src/bindings.ts` — add `changeNoiseSuppressionSetting` + `changeVadThresholdSetting` command methods (hand-edit).
- **Modify** `app/src/stores/settingsStore.ts` — add `noise_suppression` + `vad_threshold` updaters.

## Task order

T1 (rebuild_recorder method) → T2 (commands + registration, call rebuild) → T3 (frontend wiring).

---

### Task 1: `rebuild_recorder()` on AudioRecordingManager

**Files:**
- Modify: `app/src-tauri/src/managers/audio.rs` (add the method in the `impl AudioRecordingManager`, near `update_selected_device` ~line 461)

**Interfaces:**
- Produces: `pub fn rebuild_recorder(&self) -> Result<(), anyhow::Error>`
- Consumes (existing fields): `self.recorder`, `self.is_open`, `self.state` (`RecordingState`), `self.close_generation`, and existing methods `preload_vad`, `stop_microphone_stream`, `start_microphone_stream`.

- [ ] **Step 1: Read the surrounding code** to confirm field names and the `update_selected_device` pattern (`audio.rs:300-469`), and that `RecordingState` is imported in scope. (No test — this is an audio-hardware method; see Global Constraints.)

- [ ] **Step 2: Add the method.** Insert into the `impl AudioRecordingManager` block, right after `update_selected_device`:

```rust
    /// Drop the cached recorder and rebuild it from current settings, so changes to
    /// construction-time audio settings (vad_threshold, noise_suppression) take effect
    /// without an app restart. Safe-guarded:
    ///   - Never rebuilds while recording — dropping the AudioRecorder would stop the
    ///     live cpal stream and lose the in-flight audio. The change then applies the
    ///     next time the recorder is rebuilt (e.g. a later settings change while idle).
    ///   - If the mic stream is open (always-on mode), it is stopped and restarted
    ///     around the rebuild, mirroring `update_selected_device`.
    pub fn rebuild_recorder(&self) -> Result<(), anyhow::Error> {
        // Snapshot state without holding the guards across the helper calls below
        // (stop/start_microphone_stream lock `is_open` themselves).
        let is_idle = matches!(*self.state.lock().unwrap(), RecordingState::Idle);
        if !is_idle {
            debug!("rebuild_recorder skipped: recording in progress");
            return Ok(());
        }
        let was_open = *self.is_open.lock().unwrap();

        if was_open {
            self.close_generation.fetch_add(1, Ordering::SeqCst);
            self.stop_microphone_stream();
        }

        // Drop the cached recorder so preload_vad rebuilds it with current settings.
        *self.recorder.lock().unwrap() = None;
        self.preload_vad()?;

        if was_open {
            self.start_microphone_stream()?;
        }

        debug!("recorder rebuilt from current settings");
        Ok(())
    }
```

- [ ] **Step 3: Build**

Run: `cd app/src-tauri && cargo test --lib`
Expected: PASS (compiles; existing suite unaffected). If `Ordering` or `debug!` is not already imported in this file, confirm they are (they are used by `update_mode`/elsewhere in this file) — do not add unused imports.

- [ ] **Step 4: Commit**

```bash
git add app/src-tauri/src/managers/audio.rs
git commit -m "feat(audio): add rebuild_recorder to apply construction-time settings live

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: Persist commands for noise_suppression + vad_threshold

**Files:**
- Modify: `app/src-tauri/src/shortcut/mod.rs` (add two commands near the other `change_*` commands)
- Modify: `app/src-tauri/src/lib.rs` (register both in `collect_commands![`)

**Interfaces:**
- Consumes: `crate::settings::{get_settings, write_settings}`, `AudioRecordingManager` via `app.state::<Arc<AudioRecordingManager>>()`, `crate::managers::audio::AudioRecordingManager` (use the same path the existing audio commands use), `rebuild_recorder` (Task 1).
- Produces:
  - `pub fn change_noise_suppression_setting(app: AppHandle, enabled: bool) -> Result<(), String>`
  - `pub fn change_vad_threshold_setting(app: AppHandle, threshold: f32) -> Result<(), String>`

- [ ] **Step 1: Confirm the manager access path.** Look at `commands/audio.rs:set_selected_microphone` to copy the exact import + `app.state::<Arc<AudioRecordingManager>>()` usage (it does `rm.update_selected_device()`). Mirror that for `rebuild_recorder`. Confirm whether `shortcut/mod.rs` already imports `AudioRecordingManager` / `std::sync::Arc`; add the imports only if missing.

- [ ] **Step 2: Add the two commands** in `shortcut/mod.rs`:

```rust
#[tauri::command]
#[specta::specta]
pub fn change_noise_suppression_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.noise_suppression = enabled;
    settings::write_settings(&app, settings);
    let rm = app.state::<std::sync::Arc<crate::managers::audio::AudioRecordingManager>>();
    rm.rebuild_recorder()
        .map_err(|e| format!("Failed to rebuild recorder: {e}"))?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_vad_threshold_setting(app: AppHandle, threshold: f32) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.vad_threshold = threshold;
    settings::write_settings(&app, settings);
    let rm = app.state::<std::sync::Arc<crate::managers::audio::AudioRecordingManager>>();
    rm.rebuild_recorder()
        .map_err(|e| format!("Failed to rebuild recorder: {e}"))?;
    Ok(())
}
```

(Use whatever the canonical path to `AudioRecordingManager` is in this crate — match `commands/audio.rs`. If `State`/`Manager` is accessed via `tauri::Manager` trait, ensure it's in scope as in the existing audio commands.)

- [ ] **Step 3: Register both** in `app/src-tauri/src/lib.rs` `collect_commands![`, next to the other `shortcut::change_*` entries:

```rust
        shortcut::change_noise_suppression_setting,
        shortcut::change_vad_threshold_setting,
```

- [ ] **Step 4: Build**

Run: `cd app/src-tauri && cargo test --lib`
Expected: PASS; crate compiles with both commands registered.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/shortcut/mod.rs app/src-tauri/src/lib.rs
git commit -m "feat(audio): persist noise_suppression and vad_threshold + rebuild on change

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Frontend wiring (bindings + store updaters)

**Files:**
- Modify: `app/src/bindings.ts` (hand-add the two command methods)
- Modify: `app/src/stores/settingsStore.ts` (add the two updaters)

**Interfaces:**
- Consumes: Task 2 commands `change_noise_suppression_setting`, `change_vad_threshold_setting`; settings keys `noise_suppression` (boolean), `vad_threshold` (number). Both fields already exist in `AppSettings` (no field additions).

- [ ] **Step 1: Hand-edit `bindings.ts` — two commands.** In the `commands` object, mirroring `changeMuteWhileRecordingSetting`:

```typescript
async changeNoiseSuppressionSetting(enabled: boolean) : Promise<Result<null, string>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("change_noise_suppression_setting", { enabled }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async changeVadThresholdSetting(threshold: number) : Promise<Result<null, string>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("change_vad_threshold_setting", { threshold }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
```

- [ ] **Step 2: Add the store updaters.** In `app/src/stores/settingsStore.ts` `settingUpdaters`, next to the other audio settings:

```typescript
  noise_suppression: (value) =>
    commands.changeNoiseSuppressionSetting(value as boolean),
  vad_threshold: (value) => commands.changeVadThresholdSetting(value as number),
```

- [ ] **Step 3: Typecheck + lint**

Run: `cd app && bun run tsc && bun run lint`
Expected: clean (no errors — the DevInject lint errors were fixed on main).

- [ ] **Step 4: Commit**

```bash
git add app/src/bindings.ts app/src/stores/settingsStore.ts
git commit -m "feat(audio): wire noise-suppression and mic-sensitivity to backend persistence

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Final verification (after all tasks)

- [ ] `cd app/src-tauri && cargo test --lib` — full suite passes.
- [ ] `cd app && bun run tsc && bun run lint` — clean.
- [ ] **Manual (the real gate, on Windows):** run `bun run tauri dev`. (1) Drag **Mic Sensitivity** → speak immediately (no restart) → VAD behavior changes; close and reopen the app → the slider keeps its value (persistence). (2) Toggle **Noise Suppression** off → speak → recognition reflects the denoiser being off, immediately. (3) Both: change a setting *while recording* → it does not crash the active recording (the change applies afterward).

## Self-Review (plan vs. findings)

**Coverage:** persistence gap (no command/updater) → T2 + T3; cached-recorder-never-rebuilt → T1 + the rebuild calls in T2; safety (no mid-recording rebuild, always-on stream restart, lock ordering) → T1's guards. No gaps.

**Placeholder scan:** none — full code in each step. The "confirm the manager path / imports" notes are integration checks against real code (the canonical access pattern lives in `commands/audio.rs`), not placeholders.

**Type consistency:** `rebuild_recorder(&self) -> Result<(), anyhow::Error>` defined T1, called in T2. Command names `change_noise_suppression_setting`/`change_vad_threshold_setting` and setting keys `noise_suppression`/`vad_threshold` match across Rust (T2), bindings (T3), and store (T3). `vad_threshold` is `f32`/`number`, `noise_suppression` is `bool`/`boolean` — consistent.
