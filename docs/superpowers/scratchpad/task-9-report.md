# Task 9 Report — `onboarding_completed` Setting

## Files Changed

### `app/src-tauri/src/settings.rs`
- **Line ~446** (after `noise_suppression`): Added `#[serde(default)] pub onboarding_completed: bool,` to `AppSettings` struct.
- **Line ~851** (in `get_default_settings()` struct literal): Added `onboarding_completed: false,`.
- **Line ~988** (top of `mod tests`): Added new test `onboarding_completed_defaults_false`.

### `app/src-tauri/src/shortcut/mod.rs`
- **After `change_vad_threshold_setting`**: Added new command:
  ```rust
  #[tauri::command]
  #[specta::specta]
  pub fn set_onboarding_completed(app: AppHandle, completed: bool) -> Result<(), String> {
      let mut settings = settings::get_settings(&app);
      settings.onboarding_completed = completed;
      settings::write_settings(&app, settings);
      Ok(())
  }
  ```
  Shape mirrors `change_noise_suppression_setting` exactly, minus the audio-manager side-effect (not needed for a flag).

### `app/src-tauri/src/lib.rs`
- **Line ~408**: Added `shortcut::set_onboarding_completed,` to `collect_commands![` immediately after `shortcut::change_vad_threshold_setting,`.

### `app/src/bindings.ts`
- **After `changeNoiseSuppressionSetting`**: Added:
  ```ts
  async setOnboardingCompleted(completed: boolean) : Promise<Result<null, string>> {
      try {
      return { status: "ok", data: await TAURI_INVOKE("set_onboarding_completed", { completed }) };
  } catch (e) {
      if(e instanceof Error) throw e;
      else return { status: "error", error: e  as any };
  }
  },
  ```
  Exact same `try/catch` + `Result<null, string>` shape as `changeNoiseSuppressionSetting`.
- **`AppSettings` type** (line ~905): Added `onboarding_completed?: boolean` at end of the type literal.

### `app/src/stores/settingsStore.ts`
- **After `vad_threshold` entry in `settingUpdaters`**: Added:
  ```ts
  onboarding_completed: (value) =>
    commands.setOnboardingCompleted(value as boolean),
  ```
  Mirrors the `noise_suppression` entry shape exactly.

## Bindings Shape Mirrored

Source command: `changeNoiseSuppressionSetting(enabled: boolean) : Promise<Result<null, string>>`
New command:    `setOnboardingCompleted(completed: boolean) : Promise<Result<null, string>>`

Both use the `try { return { status: "ok", ... } } catch (e) { ... }` pattern.

## Test Output

```
running 1 test
test settings::tests::onboarding_completed_defaults_false ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 132 filtered out; finished in 0.01s
```

## Build Output

- `cd app && bun run build` — exit code 0 (clean)
- `cd app && bun run lint` — exit code 0 (clean, no output)
- `cargo test onboarding` — 1 passed

## Concerns

None. The field uses `#[serde(default)]` so existing settings files without the key deserialize correctly (defaults to `false`, showing onboarding on first run). No `unwrap` in the command. The store updater ensures the value is persisted when changed through the Zustand store.
