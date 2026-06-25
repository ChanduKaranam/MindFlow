# MindFlow Audio Front-End Upgrade (AGC + Silero v6 + Tunable Sensitivity) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop the VAD from dropping soft-spoken words and make it robust in noisy rooms by adding gain-normalization in front of detection, upgrading Silero VAD v4 → v6, and exposing a user-tunable sensitivity control.

**Architecture:** Three layers, all CPU-only and local. (1) A pure RMS gain-normalization helper boosts each frame toward a target level *for detection only* — the audio handed to transcription stays untouched, protecting ASR quality. (2) A new `SileroV6Vad` runs the Silero v6 ONNX model directly through `ort` (the existing `vad-rs` fork is hard-wired to v4's `h`/`c` tensor interface and cannot load v6, which uses a single `state` tensor and a fixed 512-sample window). (3) A persisted `vad_threshold` setting flows from a frontend slider into recorder construction.

**Tech Stack:** Rust, Tauri 2, `ort = "=2.0.0-rc.12"` (ONNX Runtime, same version `vad-rs` already pulls in), `ndarray = "0.17"`, `hound` (already a dep), Silero VAD v6 (MIT), React + i18next frontend.

## Global Constraints

- **CPU-only**: no GPU features anywhere. `ort` must use the default CPU execution provider (no `coreml`/`directml`/`cuda` features). One line each, verbatim.
- **Free licenses only**: Silero v6 is MIT. No TEN VAD (Agora non-compete license — disqualified).
- **ort version pinned**: `ort = "=2.0.0-rc.12"` exactly, to match the version `vad-rs` and the ONNX path already resolve to (avoids a duplicate native onnxruntime link).
- **ndarray version**: `ndarray = "0.17"` exactly (matches `vad-rs`).
- **Frame size**: Silero v6 at 16 kHz requires **exactly 512 samples (32 ms) per inference**. The recording resampler must emit 512-sample frames.
- **Transcription audio must stay un-normalized**: gain is applied to the copy the VAD *sees*, never to the buffer sent to the transcription engine.
- **Model file**: `silero_vad_v6.onnx`, SHA256 `597d30b3ec076608d059477bb14cfeffdf951bf5cae370d38f65d33bbfe82004`, 2,327,524 bytes, from `https://github.com/snakers4/silero-vad/raw/v6.0/src/silero_vad/data/silero_vad.onnx` (pinned to the `v6.0` tag). v6 ONNX interface: inputs `input` (f32 `[1,512]`) + `state` (f32 `[2,1,128]`); outputs `output` (f32 `[1,1]` probability) + `stateN` (f32 `[2,1,128]`). **No `sr` input.**
- All work is under `app/src-tauri/` unless a path says otherwise. Commands assume CWD `app/src-tauri`.

---

### Task 1: RMS gain-normalization helper

**Files:**
- Create: `app/src-tauri/src/audio_toolkit/audio/gain.rs`
- Modify: `app/src-tauri/src/audio_toolkit/audio/mod.rs` (add `pub mod gain;`)
- Test: inline `#[cfg(test)]` module in `gain.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `pub fn rms_normalized(frame: &[f32], target_dbfs: f32, noise_gate_dbfs: f32, max_gain: f32) -> Vec<f32>` — returns a boosted copy of `frame` for VAD detection. Boost-only (never attenuates). Frames quieter than `noise_gate_dbfs` are returned unchanged so the silence noise floor is not pumped up. Gain is clamped to `max_gain`; output samples are clamped to `[-1.0, 1.0]`.

- [ ] **Step 1: Add the module declaration**

In `app/src-tauri/src/audio_toolkit/audio/mod.rs`, add this line alongside the other `mod` declarations (e.g. near `mod recorder;` / `mod resampler;`):

```rust
pub mod gain;
```

- [ ] **Step 2: Write the failing tests**

Create `app/src-tauri/src/audio_toolkit/audio/gain.rs` with ONLY the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn rms(frame: &[f32]) -> f32 {
        (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt()
    }

    #[test]
    fn silence_is_unchanged() {
        let silence = vec![0.0f32; 512];
        let out = rms_normalized(&silence, -20.0, -50.0, 8.0);
        assert!(out.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn quiet_speech_is_boosted_toward_target() {
        // ~ -40 dBFS sine (rms ~0.01), above the -50 dBFS gate.
        let quiet: Vec<f32> = (0..512)
            .map(|i| 0.01 * (i as f32 * 0.2).sin())
            .collect();
        let before = rms(&quiet);
        let out = rms_normalized(&quiet, -20.0, -50.0, 8.0);
        let after = rms(&out);
        assert!(after > before * 2.0, "expected boost, before={before} after={after}");
    }

    #[test]
    fn noise_floor_below_gate_is_not_amplified() {
        // ~ -60 dBFS (rms ~0.001), below the -50 dBFS gate.
        let floor: Vec<f32> = (0..512)
            .map(|i| 0.001 * (i as f32 * 0.2).sin())
            .collect();
        let out = rms_normalized(&floor, -20.0, -50.0, 8.0);
        assert_eq!(out, floor, "frames below the gate must be returned unchanged");
    }

    #[test]
    fn loud_speech_is_not_attenuated() {
        // ~ -6 dBFS (rms ~0.5), already above the -20 dBFS target.
        let loud: Vec<f32> = (0..512)
            .map(|i| 0.5 * (i as f32 * 0.2).sin())
            .collect();
        let out = rms_normalized(&loud, -20.0, -50.0, 8.0);
        assert_eq!(out, loud, "boost-only: loud frames must be unchanged");
    }

    #[test]
    fn gain_is_clamped_and_output_stays_in_range() {
        // Just above the gate but very quiet → would want huge gain; must clamp to 8x.
        let v: Vec<f32> = (0..512)
            .map(|i| 0.004 * (i as f32 * 0.2).sin())
            .collect();
        let out = rms_normalized(&v, -20.0, -50.0, 8.0);
        assert!(out.iter().all(|&s| s >= -1.0 && s <= 1.0));
        // Gain capped at 8x, so the loudest sample is at most 8 * 0.004 = 0.032.
        let peak = out.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(peak <= 0.04, "gain not clamped: peak={peak}");
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cd app/src-tauri && cargo test --lib gain::tests 2>&1 | tail -20`
Expected: FAIL — `cannot find function rms_normalized in this scope`.

- [ ] **Step 4: Write the implementation**

Prepend the implementation above the test module in `gain.rs`:

```rust
//! Detection-only gain normalization placed in front of the VAD.
//! Quiet speech is boosted so the VAD scores it as speech; the audio handed to
//! transcription is never modified by this stage.

/// Boost `frame` toward `target_dbfs` for VAD detection.
///
/// - Boost-only: gain is clamped to `[1.0, max_gain]`, loud frames pass through.
/// - Frames whose RMS is below `noise_gate_dbfs` are returned unchanged so the
///   silence noise floor is not amplified ("noise breathing").
/// - Output samples are clamped to `[-1.0, 1.0]`.
pub fn rms_normalized(frame: &[f32], target_dbfs: f32, noise_gate_dbfs: f32, max_gain: f32) -> Vec<f32> {
    if frame.is_empty() {
        return Vec::new();
    }
    let sum_sq: f32 = frame.iter().map(|s| s * s).sum();
    let rms = (sum_sq / frame.len() as f32).sqrt();
    if rms <= 1e-9 {
        return frame.to_vec();
    }
    let rms_dbfs = 20.0 * rms.log10();
    if rms_dbfs < noise_gate_dbfs {
        return frame.to_vec();
    }
    let target_rms = 10f32.powf(target_dbfs / 20.0);
    let gain = (target_rms / rms).clamp(1.0, max_gain);
    frame.iter().map(|s| (s * gain).clamp(-1.0, 1.0)).collect()
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd app/src-tauri && cargo test --lib gain::tests 2>&1 | tail -20`
Expected: PASS — 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/audio_toolkit/audio/gain.rs app/src-tauri/src/audio_toolkit/audio/mod.rs
git commit -m "feat(vad): add RMS gain-normalization helper for soft-speech detection"
```

---

### Task 2: Silero v6 VAD via ort

**Files:**
- Modify: `app/src-tauri/Cargo.toml` (add `ort`, `ndarray` deps)
- Create: `app/src-tauri/resources/models/silero_vad_v6.onnx` (downloaded)
- Create: `app/src-tauri/src/audio_toolkit/vad/silero_v6.rs`
- Modify: `app/src-tauri/src/audio_toolkit/vad/mod.rs` (declare + export)
- Test: inline `#[cfg(test)]` module in `silero_v6.rs` (uses the real model + `tests/fixtures/jfk.wav`)

**Interfaces:**
- Consumes: `crate::audio_toolkit::audio::gain::rms_normalized` (Task 1).
- Produces: `pub struct SileroV6Vad` implementing `VoiceActivityDetector`, with:
  - `pub fn new<P: AsRef<Path>>(model_path: P, threshold: f32) -> Result<Self>`
  - `pub fn probability(&mut self, frame: &[f32]) -> Result<f32>` — runs one 512-sample frame, returns speech probability `[0.0, 1.0]`, advances internal LSTM `state`.
  - `push_frame` returns `VadFrame::Speech(frame)` (the **original** frame) when `probability > threshold`, else `VadFrame::Noise`.
  - `reset()` zeroes `state`.

- [ ] **Step 1: Add dependencies**

In `app/src-tauri/Cargo.toml`, under `[dependencies]`, add (place near the existing `sysinfo = "0.33"` line):

```toml
ort = "=2.0.0-rc.12"
ndarray = "0.17"
```

- [ ] **Step 2: Download and verify the v6 model**

Run from the repo root:

```bash
curl -fsSL -o app/src-tauri/resources/models/silero_vad_v6.onnx \
  https://github.com/snakers4/silero-vad/raw/v6.0/src/silero_vad/data/silero_vad.onnx
sha256sum app/src-tauri/resources/models/silero_vad_v6.onnx
```

Expected: `597d30b3ec076608d059477bb14cfeffdf951bf5cae370d38f65d33bbfe82004  app/src-tauri/resources/models/silero_vad_v6.onnx`

If the hash differs, STOP — do not proceed with a mismatched model.

- [ ] **Step 3: Write the failing tests**

Create `app/src-tauri/src/audio_toolkit/vad/silero_v6.rs` with ONLY the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn model_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/models/silero_vad_v6.onnx")
    }

    #[test]
    fn silence_is_not_speech() {
        let mut vad = SileroV6Vad::new(model_path(), 0.5).unwrap();
        let silence = vec![0.0f32; 512];
        let mut max_p = 0.0f32;
        for _ in 0..10 {
            max_p = max_p.max(vad.probability(&silence).unwrap());
        }
        assert!(max_p < 0.2, "silence probability too high: {max_p}");
    }

    #[test]
    fn jfk_fixture_contains_speech() {
        let wav = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jfk.wav");
        let mut reader = hound::WavReader::open(&wav).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, 16000, "fixture must be 16 kHz");
        assert_eq!(spec.channels, 1, "fixture must be mono");

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => reader
                .samples::<i16>()
                .map(|s| s.unwrap() as f32 / 32768.0)
                .collect(),
            hound::SampleFormat::Float => {
                reader.samples::<f32>().map(|s| s.unwrap()).collect()
            }
        };

        let mut vad = SileroV6Vad::new(model_path(), 0.5).unwrap();
        let mut speech_frames = 0;
        for chunk in samples.chunks_exact(512) {
            if vad.probability(chunk).unwrap() > 0.5 {
                speech_frames += 1;
            }
        }
        assert!(speech_frames > 10, "expected speech to be detected, got {speech_frames} frames");
    }

    #[test]
    fn rejects_wrong_frame_size() {
        let mut vad = SileroV6Vad::new(model_path(), 0.5).unwrap();
        let wrong = vec![0.0f32; 480];
        assert!(vad.push_frame(&wrong).is_err());
    }
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cd app/src-tauri && cargo test --lib silero_v6::tests 2>&1 | tail -20`
Expected: FAIL — `cannot find type SileroV6Vad`.

- [ ] **Step 5: Write the implementation**

Prepend to `silero_v6.rs` (above the test module):

```rust
use anyhow::Result;
use ndarray::{Array2, Array3};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use std::path::Path;

use super::{VadFrame, VoiceActivityDetector};
use crate::audio_toolkit::audio::gain;

/// Silero v6 requires exactly 512 samples (32 ms) per inference at 16 kHz.
const V6_FRAME_SAMPLES: usize = 512;

// Detection-only gain defaults (see audio_toolkit::audio::gain).
const GAIN_TARGET_DBFS: f32 = -20.0;
const GAIN_NOISE_GATE_DBFS: f32 = -50.0;
const GAIN_MAX: f32 = 8.0;

pub struct SileroV6Vad {
    session: Session,
    state: Array3<f32>, // [2, 1, 128]
    threshold: f32,
}

impl SileroV6Vad {
    pub fn new<P: AsRef<Path>>(model_path: P, threshold: f32) -> Result<Self> {
        if !(0.0..=1.0).contains(&threshold) {
            anyhow::bail!("threshold must be between 0.0 and 1.0");
        }
        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("ort session builder: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("ort opt level: {e}"))?
            .with_intra_threads(1)
            .map_err(|e| anyhow::anyhow!("ort intra threads: {e}"))?
            .with_inter_threads(1)
            .map_err(|e| anyhow::anyhow!("ort inter threads: {e}"))?
            .commit_from_file(model_path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to load Silero v6 model: {e}"))?;

        Ok(Self {
            session,
            state: Array3::<f32>::zeros((2, 1, 128)),
            threshold,
        })
    }

    /// Run one 512-sample frame; returns speech probability and advances state.
    pub fn probability(&mut self, frame: &[f32]) -> Result<f32> {
        // Boost soft speech for detection only (the original `frame` is what the
        // caller keeps for transcription).
        let boosted = gain::rms_normalized(frame, GAIN_TARGET_DBFS, GAIN_NOISE_GATE_DBFS, GAIN_MAX);

        let input = Array2::from_shape_vec((1, boosted.len()), boosted)?;
        let input_value = Value::from_array(input)?;
        let state_value = Value::from_array(self.state.clone())?;

        let outputs = self
            .session
            .run(ort::inputs![
                "input" => input_value,
                "state" => state_value,
            ])
            .map_err(|e| anyhow::anyhow!("Silero v6 inference error: {e}"))?;

        // Advance LSTM state from `stateN`.
        let state_out = outputs
            .get("stateN")
            .ok_or_else(|| anyhow::anyhow!("model output 'stateN' missing"))?
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow::anyhow!("extract stateN: {e}"))?;
        self.state = Array3::from_shape_vec((2, 1, 128), state_out.1.to_vec())?;

        let prob_out = outputs
            .get("output")
            .ok_or_else(|| anyhow::anyhow!("model output 'output' missing"))?
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow::anyhow!("extract output: {e}"))?;
        Ok(prob_out.1[0])
    }
}

impl VoiceActivityDetector for SileroV6Vad {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>> {
        if frame.len() != V6_FRAME_SAMPLES {
            anyhow::bail!("expected {V6_FRAME_SAMPLES} samples, got {}", frame.len());
        }
        let prob = self.probability(frame)?;
        if prob > self.threshold {
            Ok(VadFrame::Speech(frame))
        } else {
            Ok(VadFrame::Noise)
        }
    }

    fn reset(&mut self) {
        self.state.fill(0.0);
    }
}
```

> **Implementer note:** the `ort` API here mirrors the existing `vad-rs` fork (`ort::inputs![ "name" => value ]`, `try_extract_tensor::<f32>()` returning `(shape, &[f32])` where `.1` is the data slice). If `try_extract_tensor`'s return shape differs in this exact rc, match the call form already used in `~/.cargo/git/checkouts/vad-rs-*/src/vad.rs`.

- [ ] **Step 6: Declare and export the module**

In `app/src-tauri/src/audio_toolkit/vad/mod.rs`, add after the existing `mod smoothed;`:

```rust
mod silero_v6;
```

and add after the existing `pub use smoothed::SmoothedVad;`:

```rust
pub use silero_v6::SileroV6Vad;
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cd app/src-tauri && cargo test --lib silero_v6::tests 2>&1 | tail -30`
Expected: PASS — 3 tests pass. (First run compiles `ort`/`ndarray`; allow several minutes.)

- [ ] **Step 8: Commit**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/Cargo.lock \
  app/src-tauri/resources/models/silero_vad_v6.onnx \
  app/src-tauri/src/audio_toolkit/vad/silero_v6.rs \
  app/src-tauri/src/audio_toolkit/vad/mod.rs
git commit -m "feat(vad): add Silero v6 ONNX VAD with detection-time gain normalization"
```

---

### Task 3: Switch the live recording pipeline to Silero v6 (512-sample frames)

**Files:**
- Modify: `app/src-tauri/src/audio_toolkit/audio/recorder.rs` (frame duration 30 → 32 ms)
- Modify: `app/src-tauri/src/managers/audio.rs` (`create_audio_recorder`, model path)
- Test: `cargo check` + the Task 2 suite (the live path has no automated GUI test; manual verification noted)

**Interfaces:**
- Consumes: `SileroV6Vad` (Task 2).
- Produces: a recorder whose VAD is Silero v6 fed exactly-512-sample frames.

- [ ] **Step 1: Make the VAD resampler emit 512-sample frames**

In `app/src-tauri/src/audio_toolkit/audio/recorder.rs`, find the `FrameResampler::new(...)` call that feeds the VAD (it passes `Duration::from_millis(30)` — currently around line 403-407). Change the frame duration:

```rust
let mut frame_resampler = FrameResampler::new(
    in_sample_rate as usize,
    constants::WHISPER_SAMPLE_RATE as usize,
    Duration::from_millis(32), // 512 samples @ 16 kHz — required by Silero v6
);
```

> If more than one `FrameResampler` exists, change ONLY the one whose emitted frames reach `handle_frame`/the VAD. Leave any spectrum/level-meter resampler untouched.

- [ ] **Step 2: Build `SileroV6Vad` in `create_audio_recorder`**

In `app/src-tauri/src/managers/audio.rs`, update the import block at the top to bring in `SileroV6Vad` (alongside `SmoothedVad`); you may leave `SileroVad` imported or remove it if now unused.

Replace the body of `create_audio_recorder` (lines ~124-126) so it uses v6 and accepts a threshold:

```rust
fn create_audio_recorder(
    vad_path: &str,
    threshold: f32,
    app_handle: &tauri::AppHandle,
) -> Result<AudioRecorder, anyhow::Error> {
    let silero = SileroV6Vad::new(vad_path, threshold)
        .map_err(|e| anyhow::anyhow!("Failed to create SileroV6Vad: {}", e))?;
    // Frames are now 32 ms: 14 frames ≈ 448 ms pre-roll / hang-over; onset 1 = snappier
    // start so the first quiet syllable is not clipped.
    let smoothed_vad = SmoothedVad::new(Box::new(silero), 14, 14, 1);
    // ... rest of the function unchanged (AudioRecorder::new()...with_vad(Box::new(smoothed_vad))...)
```

Keep the remainder of the function (the `AudioRecorder::new()...with_vad(...).with_level_callback(...)` chain) exactly as it was.

- [ ] **Step 3: Point the model path at v6 and pass the threshold at the call site**

In `app/src-tauri/src/managers/audio.rs`, around lines 269-278 where `vad_path` is resolved, change the resource filename:

```rust
"silero_vad_v6.onnx",
```

(from `"resources/models/silero_vad_v4.onnx"` — keep whatever path-prefix form the surrounding `resolve_resource`/`resource_dir` call uses; only the filename changes to `silero_vad_v6.onnx`).

At the call to `create_audio_recorder(vad_path..., app_handle)`, add the threshold argument. For this task pass the constant default:

```rust
create_audio_recorder(vad_path.to_str().unwrap(), 0.4, app_handle)
```

(Task 4 replaces this literal `0.4` with the persisted setting.)

- [ ] **Step 4: Verify it compiles**

Run: `cd app/src-tauri && cargo check 2>&1 | tail -20`
Expected: compiles with no errors. Fix any unused-import warning for `SileroVad` by removing it from the import list if it is no longer referenced.

- [ ] **Step 5: Re-run the VAD test suite**

Run: `cd app/src-tauri && cargo test --lib silero_v6::tests gain::tests 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/audio_toolkit/audio/recorder.rs app/src-tauri/src/managers/audio.rs
git commit -m "feat(vad): switch live recording pipeline to Silero v6 with 512-sample frames"
```

- [ ] **Step 7: Manual verification (real desktop)**

On a Windows/macOS/Linux machine: `cd app && bun run tauri dev`, hold the hotkey, speak softly, release. Confirm quiet words now appear that previously dropped. (No automated GUI test exists; record the result in the PR description.)

---

### Task 4: Persisted, tunable VAD threshold (backend)

**Files:**
- Modify: `app/src-tauri/src/settings.rs` (add `vad_threshold` field + default fn)
- Modify: `app/src-tauri/src/managers/audio.rs` (read the setting at the call site)
- Test: inline `#[cfg(test)]` in `settings.rs` (default + serde round-trip)

**Interfaces:**
- Consumes: `AppSettings` (existing).
- Produces: `AppSettings.vad_threshold: f32` (default `0.4`, valid range `0.0..=1.0`), persisted via the existing store; consumed by `create_audio_recorder`.

- [ ] **Step 1: Write the failing tests**

Add to `app/src-tauri/src/settings.rs` (in or alongside any existing `#[cfg(test)] mod tests`; if none exists, add one at the end of the file):

```rust
#[cfg(test)]
mod vad_threshold_tests {
    use super::*;

    #[test]
    fn default_vad_threshold_is_0_4() {
        assert_eq!(default_vad_threshold(), 0.4);
    }

    #[test]
    fn vad_threshold_survives_serde_round_trip() {
        let json = r#"{"vad_threshold": 0.25}"#;
        #[derive(serde::Deserialize)]
        struct Probe {
            #[serde(default = "default_vad_threshold")]
            vad_threshold: f32,
        }
        let p: Probe = serde_json::from_str(json).unwrap();
        assert_eq!(p.vad_threshold, 0.25);

        // Missing key falls back to the default.
        let p2: Probe = serde_json::from_str("{}").unwrap();
        assert_eq!(p2.vad_threshold, 0.4);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd app/src-tauri && cargo test --lib vad_threshold 2>&1 | tail -20`
Expected: FAIL — `cannot find function default_vad_threshold`.

- [ ] **Step 3: Add the field and default function**

In `app/src-tauri/src/settings.rs`, add a field to the `AppSettings` struct (after `extra_recording_buffer_ms` at line 432):

```rust
    #[serde(default = "default_vad_threshold")]
    pub vad_threshold: f32,
```

Add the default function near the other `default_*` fns (e.g. after `default_model`):

```rust
fn default_vad_threshold() -> f32 {
    0.4
}
```

- [ ] **Step 4: Set the field wherever `AppSettings` is constructed with explicit fields**

Run: `cd app/src-tauri && cargo check 2>&1 | tail -30`. If the compiler reports `missing field vad_threshold` at a struct literal (e.g. a `get_default_settings()`/`Default` constructor), add `vad_threshold: default_vad_threshold(),` there. If `AppSettings` is only ever built via serde deserialization, no literal needs changing.

- [ ] **Step 5: Read the setting at the recorder call site**

In `app/src-tauri/src/managers/audio.rs`, at the call site updated in Task 3 Step 3, replace the literal `0.4` with the persisted value. Locate how settings are read nearby (mirror the existing pattern used to obtain `selected_model`/`always_on_microphone` from the settings store via `app_handle`), then:

```rust
let vad_threshold = settings.vad_threshold; // from the loaded AppSettings
create_audio_recorder(vad_path.to_str().unwrap(), vad_threshold, app_handle)
```

If the surrounding code does not already have an `AppSettings` in scope at this point, load it using the same helper the rest of `audio.rs` uses (search the file for how settings are fetched, e.g. `get_settings`/`settings_store`), and read `.vad_threshold`.

- [ ] **Step 6: Verify build + tests**

Run: `cd app/src-tauri && cargo test --lib vad_threshold 2>&1 | tail -20 && cargo check 2>&1 | tail -10`
Expected: tests PASS, `cargo check` clean.

- [ ] **Step 7: Regenerate TypeScript bindings (if specta auto-gen is wired)**

If `AppSettings` is exposed to the frontend via `tauri-specta` (it is — `bindings.ts` is generated), run the app once in dev or the bindings-generation step so `vad_threshold` appears in `src/bindings.ts`. Run: `cd app && bun run tauri dev` briefly, then stop; confirm `vad_threshold` is present in `app/src/bindings.ts`. Commit the regenerated bindings.

- [ ] **Step 8: Commit**

```bash
git add app/src-tauri/src/settings.rs app/src-tauri/src/managers/audio.rs app/src/bindings.ts
git commit -m "feat(vad): persist tunable vad_threshold setting and wire it into the recorder"
```

---

### Task 5: Sensitivity slider in Settings (frontend)

**Files:**
- Modify: a settings panel component under `app/src/components/settings/` (the audio/recording section)
- Modify: `app/src/i18n/locales/en/translation.json` (new key)
- Test: manual (consistent with the project's existing frontend verification approach)

**Interfaces:**
- Consumes: `vad_threshold` from `AppSettings` (Task 4) via the settings store/hook (`useSettings`).
- Produces: a user-facing slider that writes `vad_threshold`.

- [ ] **Step 1: Add the i18n strings**

In `app/src/i18n/locales/en/translation.json`, add under the settings section (match the existing nesting used by other recording settings):

```json
"micSensitivity": "Microphone sensitivity",
"micSensitivityHelp": "Higher catches softer speech; lower ignores more background noise."
```

- [ ] **Step 2: Add the slider control**

In the recording/audio settings component (find the one rendering `push_to_talk` / `always_on_microphone` controls — search `app/src/components/settings/` for `push_to_talk`), add a slider bound to `vad_threshold`. Present it as **sensitivity** (inverted): sensitivity = `1 - vad_threshold`, so dragging right lowers the threshold.

```tsx
// `settings` and an update fn come from the existing settings hook/store in this file.
const sensitivity = 1 - (settings.vad_threshold ?? 0.4);

<label className="flex flex-col gap-1">
  <span>{t("settings.micSensitivity")}</span>
  <input
    type="range"
    min={0}
    max={1}
    step={0.05}
    value={sensitivity}
    onChange={(e) =>
      updateSetting("vad_threshold", 1 - parseFloat(e.target.value))
    }
  />
  <span className="text-xs opacity-70">{t("settings.micSensitivityHelp")}</span>
</label>
```

> Match the file's existing control/update idiom (the exact updater name — e.g. `updateSetting`, `setSetting`, or a Zustand action — and class names). Do not invent a new state pattern; reuse what the neighboring `push_to_talk` toggle uses.

- [ ] **Step 3: Lint check**

Run: `cd app && bun run lint 2>&1 | tail -20`
Expected: no new errors (i18next rule satisfied because all strings use `t(...)`).

- [ ] **Step 4: Manual verification**

`cd app && bun run tauri dev`. Open Settings → recording section. Confirm the slider appears, persists across restart, and that moving it toward "more sensitive" captures softer speech while "less sensitive" ignores more noise.

- [ ] **Step 5: Commit**

```bash
git add app/src/components/settings app/src/i18n/locales/en/translation.json
git commit -m "feat(vad): add microphone sensitivity slider to settings"
```

---

## Self-Review

**Spec coverage:**
- AGC / soft-speech fix → Task 1 (helper) + Task 2 (wired into v6 detection). ✓
- Silero v4 → v6 upgrade → Task 2 (model + impl) + Task 3 (live pipeline, 512 frames). ✓
- Tunable sensitivity → Task 4 (persisted backend setting) + Task 5 (frontend slider). ✓
- CPU-only / free-license constraints → Global Constraints; `ort` default CPU EP, Silero v6 MIT. ✓
- Transcription audio stays clean → enforced by detection-only gain returning the original frame in `push_frame`. ✓

**Placeholder scan:** No "TBD"/"handle edge cases"/"write tests for the above". Tasks 3 Step 3, 4 Step 5, and 5 Step 2 contain explicit *locate-then-edit* instructions (not placeholders) because exact line numbers/identifiers in `audio.rs` settings-load and the frontend updater must be matched to the surrounding code; concrete target code is given for each.

**Type consistency:** `rms_normalized(frame, target_dbfs, noise_gate_dbfs, max_gain)` is defined in Task 1 and called identically in Task 2. `SileroV6Vad::new(path, threshold)` and `probability(&mut self, frame)` are consistent between Task 2's definition, its tests, and Task 3's use. `create_audio_recorder(vad_path, threshold, app_handle)` signature is introduced in Task 3 and reused unchanged in Task 4. `vad_threshold: f32` (default `0.4`) is consistent across Task 4 (backend) and Task 5 (frontend, via `1 - vad_threshold`). Model filename `silero_vad_v6.onnx` is consistent across Tasks 2-3. Frame size `512` is consistent across Tasks 2-3.
