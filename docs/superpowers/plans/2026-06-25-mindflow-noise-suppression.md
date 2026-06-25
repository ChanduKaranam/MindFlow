# MindFlow Noise Suppression (GTCRN Front-End) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an automatic, toggleable GTCRN noise-suppression front-end so MindFlow captures usable voice on real-world mics in noisy rooms, feeding cleaned audio to both the VAD and transcription.

**Architecture:** A streaming sqrt-Hann STFT/iSTFT stage (`realfft`) wraps the GTCRN ONNX model (run through the `ort` runtime already in the tree) to denoise the 16 kHz capture stream frame-by-frame. The cleaned stream feeds the existing Silero v6 VAD and the transcription buffer; a whole-clip loudness normalize is applied once before transcription. A single persisted `noise_suppression` setting (default on) gates the whole stage.

**Tech Stack:** Rust, Tauri 2, `ort = "=2.0.0-rc.12"` (ONNX Runtime), `ndarray = "0.17"`, `realfft = "3"` (on the `rustfft = "6"` already in tree), `hound` (tests), React + i18next frontend.

## Global Constraints

- **CPU-only**: no GPU features. `ort` uses the default CPU execution provider.
- **Pinned versions, exact**: `ort = "=2.0.0-rc.12"`, `ndarray = "0.17"` (already present). Add `realfft = "3"` (built on the in-tree `rustfft = "6"`).
- **Free licenses only**: GTCRN is MIT.
- **Model**: `gtcrn_simple.onnx`, **SHA256 `b4718df6228e7bdf1a8a435cf98f838636eb2fd331acabf86ba87c5192ebcb87`**, **535,190 bytes**, from `https://github.com/Xiaobin-Rong/gtcrn/raw/main/stream/onnx_models/gtcrn_simple.onnx`. Confirmed graph inputs: `input`, `conv_cache`, `tra_cache`, `inter_cache`; outputs: the enhanced signal + `conv_cache_out`, `tra_cache_out`, `inter_cache_out`. Exact output name and all tensor shapes **must be confirmed by introspecting the real `ort` session** (Task 2 Step 3) — the research's `mix`/`enh` names did not match the real model's `input`, so do not trust names from this plan over the live session.
- **STFT params**: 16 kHz, n_fft = 512, hop = 256, window = **sqrt-Hann** (periodic), 257 one-sided bins, WOLA overlap-add (unity gain at 50% overlap). Complex packed as a trailing `[..,2]` axis: real at index 0, imag at index 1.
- **Audio invariants**: 16 kHz mono throughout. The VAD keeps its 512-sample frame contract — it re-frames the *cleaned* stream.
- **Toggle off → byte-identical to current pipeline.**
- Builds on `main` + the merged VAD work (Silero v6, no per-frame gain, `vad_threshold` default 0.5). Base this work on the `mindflow-vad-gain-fix` branch (or `main` after it lands).
- All backend work under `app/src-tauri/`; frontend under `app/src/`. Bash commands assume CWD `app/src-tauri` unless stated.

---

### Task 1: Streaming STFT / iSTFT (sqrt-Hann WOLA)

**Files:**
- Modify: `app/src-tauri/Cargo.toml` (add `realfft = "3"`)
- Create: `app/src-tauri/src/audio_toolkit/audio/stft.rs`
- Modify: `app/src-tauri/src/audio_toolkit/audio/mod.rs` (add `pub mod stft;`)
- Test: inline `#[cfg(test)]` in `stft.rs`

**Interfaces:**
- Produces:
  - `pub struct Stft` with `pub fn new() -> Self`, `pub fn reset(&mut self)`.
  - `pub fn analyze(&mut self, hop: &[f32; 256]) -> [[f32; 2]; 257]` — push 256 new samples, return the latest frame's 257 complex bins (`[re, im]` each) computed over the trailing 512-sample sqrt-Hann window.
  - `pub fn synthesize(&mut self, bins: &[[f32; 2]; 257]) -> [f32; 256]` — inverse one frame, sqrt-Hann window, overlap-add, return the 256 samples that became final.
- Consumes: nothing.

- [ ] **Step 1: Add the dependency**

In `app/src-tauri/Cargo.toml` under `[dependencies]`, add:

```toml
realfft = "3"
```

- [ ] **Step 2: Declare the module**

In `app/src-tauri/src/audio_toolkit/audio/mod.rs`, alongside the existing `pub mod gain;`:

```rust
pub mod stft;
```

- [ ] **Step 3: Write the failing round-trip test**

Create `app/src-tauri/src/audio_toolkit/audio/stft.rs` with ONLY the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Feed a signal through analyze->synthesize unchanged; after the one-frame
    // (512-sample) warm-up latency, output must reconstruct the input.
    #[test]
    fn round_trip_reconstructs_after_latency() {
        let n = 256 * 40;
        let input: Vec<f32> = (0..n).map(|i| 0.3 * (i as f32 * 0.05).sin()).collect();

        let mut fwd = Stft::new();
        let mut inv = Stft::new();
        let mut out: Vec<f32> = Vec::new();
        for chunk in input.chunks_exact(256) {
            let mut hop = [0.0f32; 256];
            hop.copy_from_slice(chunk);
            let bins = fwd.analyze(&hop);
            let rec = inv.synthesize(&bins);
            out.extend_from_slice(&rec);
        }

        // One frame (512 samples = 2 hops) of latency; compare the steady-state middle.
        let lat = 512;
        let mut max_err = 0.0f32;
        for i in (lat..n - 256).step_by(1) {
            max_err = max_err.max((out[i] - input[i - lat]).abs());
        }
        assert!(max_err < 1e-3, "reconstruction error too high: {max_err}");
    }

    #[test]
    fn dc_bin_is_real_for_constant_input() {
        let mut fwd = Stft::new();
        let hop = [1.0f32; 256];
        let _ = fwd.analyze(&hop); // prime
        let bins = fwd.analyze(&hop);
        // Nyquist (bin 256) imag must be ~0; DC (bin 0) imag must be ~0.
        assert!(bins[0][1].abs() < 1e-4);
        assert!(bins[256][1].abs() < 1e-4);
    }
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cd app/src-tauri && cargo test --lib stft::tests 2>&1 | tail -15`
Expected: FAIL — `cannot find type Stft`.

- [ ] **Step 5: Implement the STFT**

Prepend to `stft.rs` (above the tests):

```rust
//! Streaming sqrt-Hann STFT / iSTFT with 50%-overlap WOLA reconstruction.
//! n_fft = 512, hop = 256, 257 one-sided bins. Used by the GTCRN denoiser.

use realfft::{RealFftPlanner, RealToComplex, ComplexToReal};
use realfft::num_complex::Complex;
use std::sync::Arc;

const N_FFT: usize = 512;
const HOP: usize = 256;
const BINS: usize = N_FFT / 2 + 1; // 257

fn sqrt_hann() -> [f32; N_FFT] {
    let mut w = [0.0f32; N_FFT];
    for (n, wn) in w.iter_mut().enumerate() {
        // periodic Hann, then sqrt (sqrt-Hann gives unity-gain WOLA at 50% overlap).
        let hann = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * n as f32 / N_FFT as f32).cos();
        *wn = hann.max(0.0).sqrt();
    }
    w
}

pub struct Stft {
    r2c: Arc<dyn RealToComplex<f32>>,
    c2r: Arc<dyn ComplexToReal<f32>>,
    window: [f32; N_FFT],
    in_buf: [f32; N_FFT],     // sliding analysis buffer (last 512 samples)
    ola: [f32; N_FFT],        // synthesis overlap-add accumulator
    scratch_c: Vec<Complex<f32>>,
    scratch_r: Vec<f32>,
}

impl Stft {
    pub fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(N_FFT);
        let c2r = planner.plan_fft_inverse(N_FFT);
        Self {
            r2c,
            c2r,
            window: sqrt_hann(),
            in_buf: [0.0; N_FFT],
            ola: [0.0; N_FFT],
            scratch_c: vec![Complex::new(0.0, 0.0); BINS],
            scratch_r: vec![0.0; N_FFT],
        }
    }

    pub fn reset(&mut self) {
        self.in_buf = [0.0; N_FFT];
        self.ola = [0.0; N_FFT];
    }

    pub fn analyze(&mut self, hop: &[f32; HOP]) -> [[f32; 2]; BINS] {
        // Slide in the new hop: drop oldest 256, append newest 256.
        self.in_buf.copy_within(HOP.., 0);
        self.in_buf[HOP..].copy_from_slice(hop);

        let mut windowed: Vec<f32> = (0..N_FFT).map(|i| self.in_buf[i] * self.window[i]).collect();
        self.r2c.process(&mut windowed, &mut self.scratch_c).unwrap();

        let mut out = [[0.0f32; 2]; BINS];
        for (i, c) in self.scratch_c.iter().enumerate() {
            out[i] = [c.re, c.im];
        }
        out
    }

    pub fn synthesize(&mut self, bins: &[[f32; 2]; BINS]) -> [f32; HOP] {
        for (i, b) in bins.iter().enumerate() {
            self.scratch_c[i] = Complex::new(b[0], b[1]);
        }
        self.c2r.process(&mut self.scratch_c, &mut self.scratch_r).unwrap();

        // realfft inverse is unnormalized: divide by N_FFT. Apply synthesis window, overlap-add.
        let norm = 1.0 / N_FFT as f32;
        for i in 0..N_FFT {
            self.ola[i] += self.scratch_r[i] * norm * self.window[i];
        }
        let mut out = [0.0f32; HOP];
        out.copy_from_slice(&self.ola[..HOP]);
        // shift the accumulator left by HOP, zero the tail.
        self.ola.copy_within(HOP.., 0);
        for v in self.ola[N_FFT - HOP..].iter_mut() {
            *v = 0.0;
        }
        out
    }
}

impl Default for Stft {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cd app/src-tauri && cargo test --lib stft::tests 2>&1 | tail -15`
Expected: PASS — 2 tests. If `round_trip` fails on the error bound, the most likely cause is window normalization; verify sqrt-Hann and the `1/N_FFT` inverse scale before changing the tolerance.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/Cargo.lock \
  app/src-tauri/src/audio_toolkit/audio/stft.rs app/src-tauri/src/audio_toolkit/audio/mod.rs
git commit -m "feat(denoise): add streaming sqrt-Hann STFT/iSTFT module"
```

---

### Task 2: GTCRN denoiser (ONNX via ort)

**Files:**
- Create: `app/src-tauri/resources/models/gtcrn_simple.onnx` (downloaded)
- Create: `app/src-tauri/src/audio_toolkit/denoise/mod.rs`
- Create: `app/src-tauri/src/audio_toolkit/denoise/gtcrn.rs`
- Modify: `app/src-tauri/src/audio_toolkit/mod.rs` (add `pub mod denoise;`)
- Test: inline `#[cfg(test)]` in `gtcrn.rs` (real model + `tests/fixtures/jfk.wav`)

**Interfaces:**
- Consumes: `crate::audio_toolkit::audio::stft::Stft` (Task 1).
- Produces:
  - `pub trait Denoiser: Send { fn process(&mut self, input: &[f32]) -> Vec<f32>; fn reset(&mut self); }` in `denoise/mod.rs`.
  - `pub struct GtcrnDenoiser` (`denoise/gtcrn.rs`) with `pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self>`, implementing `Denoiser`. `process` accepts an arbitrary-length 16 kHz stream, buffers internally to 256-sample hops, denoises each, and returns the cleaned samples ready so far (stream-in/out).

- [ ] **Step 1: Declare the module**

In `app/src-tauri/src/audio_toolkit/mod.rs`, add alongside the existing `pub mod vad;`:

```rust
pub mod denoise;
```

- [ ] **Step 2: Download and verify the model**

Run from the repo root:

```bash
curl -fsSL -o app/src-tauri/resources/models/gtcrn_simple.onnx \
  https://github.com/Xiaobin-Rong/gtcrn/raw/main/stream/onnx_models/gtcrn_simple.onnx
sha256sum app/src-tauri/resources/models/gtcrn_simple.onnx
```

Expected: `b4718df6228e7bdf1a8a435cf98f838636eb2fd331acabf86ba87c5192ebcb87  app/src-tauri/resources/models/gtcrn_simple.onnx` (535,190 bytes). If the hash differs, STOP — report BLOCKED.

- [ ] **Step 3: Introspect the real model I/O (do this before writing inference code)**

Write a throwaway test that loads the model and prints input/output names + shapes, run it, and record the real names/shapes. This is mandatory — the plan's names are from `strings`, not a live session.

Add temporarily to `gtcrn.rs`:

```rust
#[cfg(test)]
mod introspect {
    use ort::session::{builder::GraphOptimizationLevel, Session};
    use std::path::PathBuf;

    #[test]
    fn print_io() {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/models/gtcrn_simple.onnx");
        let s = Session::builder().unwrap()
            .with_optimization_level(GraphOptimizationLevel::Level3).unwrap()
            .commit_from_file(p).unwrap();
        for i in &s.inputs { println!("IN  {} {:?}", i.name, i.input_type); }
        for o in &s.outputs { println!("OUT {} {:?}", o.name, o.output_type); }
    }
}
```

Run: `cd app/src-tauri && cargo test --lib gtcrn::introspect::print_io -- --nocapture 2>&1 | grep -E '^(IN|OUT) '`
Record the exact input names (expected: `input`, `conv_cache`, `tra_cache`, `inter_cache`), the exact enhanced-output name (expected `enh` — confirm), the cache-output names (expected `conv_cache_out`, `tra_cache_out`, `inter_cache_out`), and every shape. **Use the printed names/shapes verbatim in Step 5.** Delete this `introspect` module before committing.

- [ ] **Step 4: Write the failing denoiser test**

Replace the `introspect` module with the real test module in `gtcrn.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_toolkit::denoise::Denoiser;
    use std::path::PathBuf;

    fn model_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/models/gtcrn_simple.onnx")
    }

    fn load_jfk() -> Vec<f32> {
        let wav = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jfk.wav");
        let mut r = hound::WavReader::open(&wav).unwrap();
        assert_eq!(r.spec().sample_rate, 16000);
        assert_eq!(r.spec().channels, 1);
        match r.spec().sample_format {
            hound::SampleFormat::Int => r.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect(),
            hound::SampleFormat::Float => r.samples::<f32>().map(|s| s.unwrap()).collect(),
        }
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    // Mix speech with white noise, denoise, and assert the noise floor in the
    // SILENT lead-in of jfk.wav is reduced (the model removes noise where there
    // is no speech) without destroying the signal energy where there is speech.
    #[test]
    fn reduces_noise_floor_without_killing_speech() {
        let clean = load_jfk();
        // jfk.wav has ~0.1s of near-silence at the start; build a noisy copy.
        let noise_amp = 0.02f32;
        let noisy: Vec<f32> = clean
            .iter()
            .enumerate()
            .map(|(i, &s)| s + noise_amp * ((i as f32 * 12.9898).sin() * 43758.547).fract())
            .collect();

        let mut d = GtcrnDenoiser::new(model_path()).unwrap();
        let out = d.process(&noisy);
        // process is stream-in/out; flush any tail by processing a little silence.
        let mut out = out;
        out.extend(d.process(&vec![0.0f32; 512]));

        assert!(!out.iter().any(|v| v.is_nan()), "denoiser produced NaN");
        assert!(out.len() > clean.len() / 2, "denoiser dropped too much audio");

        // Lead-in window (first 1500 samples ~ pre-speech) should be quieter after denoise.
        let lead = 1500.min(out.len()).min(noisy.len());
        let noisy_floor = rms(&noisy[..lead]);
        let den_floor = rms(&out[..lead]);
        assert!(den_floor < noisy_floor * 0.8, "noise floor not reduced: {noisy_floor} -> {den_floor}");
    }
}
```

> The assertion targets the pre-speech noise floor because that is the unambiguous, environment-independent signature of denoising. If `jfk.wav`'s lead-in is shorter than assumed, the implementer may widen the window — but must keep the "floor drops materially" assertion.

- [ ] **Step 5: Run the test to verify it fails, then implement**

Run: `cd app/src-tauri && cargo test --lib gtcrn::tests 2>&1 | tail -10` → FAIL (`GtcrnDenoiser` not found).

Create `denoise/mod.rs`:

```rust
mod gtcrn;
pub use gtcrn::GtcrnDenoiser;

/// A streaming single-channel denoiser. `process` is stream-in/out: feed any
/// length of 16 kHz mono samples, receive the cleaned samples ready so far.
pub trait Denoiser: Send {
    fn process(&mut self, input: &[f32]) -> Vec<f32>;
    fn reset(&mut self);
}
```

Implement `denoise/gtcrn.rs` (prepend above the test module). Use the names/shapes you recorded in Step 3 — the code below uses the expected names; correct them if introspection differed:

```rust
use anyhow::Result;
use ndarray::{Array, ArrayD, IxDyn};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use std::collections::VecDeque;
use std::path::Path;

use super::Denoiser;
use crate::audio_toolkit::audio::stft::Stft;

const HOP: usize = 256;
const BINS: usize = 257;

pub struct GtcrnDenoiser {
    session: Session,
    fwd: Stft,
    inv: Stft,
    conv_cache: ArrayD<f32>,
    tra_cache: ArrayD<f32>,
    inter_cache: ArrayD<f32>,
    in_q: VecDeque<f32>,   // pending input samples not yet formed into a 256-hop
    warmup: bool,          // discard the first output frame (model warm-up)
}

impl GtcrnDenoiser {
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("ort builder: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("ort opt: {e}"))?
            .with_intra_threads(1)
            .map_err(|e| anyhow::anyhow!("ort intra: {e}"))?
            .commit_from_file(model_path.as_ref())
            .map_err(|e| anyhow::anyhow!("load GTCRN: {e}"))?;
        Ok(Self {
            session,
            fwd: Stft::new(),
            inv: Stft::new(),
            // Shapes confirmed in Step 3 introspection.
            conv_cache: Array::zeros(IxDyn(&[2, 1, 16, 16, 33])).into_dyn(),
            tra_cache: Array::zeros(IxDyn(&[2, 3, 1, 1, 16])).into_dyn(),
            inter_cache: Array::zeros(IxDyn(&[2, 1, 33, 16])).into_dyn(),
            in_q: VecDeque::new(),
            warmup: true,
        })
    }

    fn run_frame(&mut self, hop: &[f32; HOP]) -> [f32; HOP] {
        // STFT -> [1,257,1,2]
        let bins = self.fwd.analyze(hop);
        let mut mix = Array::zeros(IxDyn(&[1, BINS, 1, 2]));
        for f in 0..BINS {
            mix[[0, f, 0, 0]] = bins[f][0];
            mix[[0, f, 0, 1]] = bins[f][1];
        }
        let outputs = self
            .session
            .run(ort::inputs![
                "input" => Value::from_array(mix).unwrap(),
                "conv_cache" => Value::from_array(self.conv_cache.clone()).unwrap(),
                "tra_cache" => Value::from_array(self.tra_cache.clone()).unwrap(),
                "inter_cache" => Value::from_array(self.inter_cache.clone()).unwrap(),
            ])
            .expect("gtcrn run");

        // Use the enhanced-output name confirmed in Step 3 (expected "enh").
        let enh = outputs.get("enh").unwrap().try_extract_tensor::<f32>().unwrap();
        let (_, enh_data) = enh;
        let mut out_bins = [[0.0f32; 2]; BINS];
        for f in 0..BINS {
            out_bins[f][0] = enh_data[(f * 1 * 2) + 0];
            out_bins[f][1] = enh_data[(f * 1 * 2) + 1];
        }

        // Advance caches.
        let cc = outputs.get("conv_cache_out").unwrap().try_extract_tensor::<f32>().unwrap();
        self.conv_cache = Array::from_shape_vec(self.conv_cache.raw_dim(), cc.1.to_vec()).unwrap();
        let tc = outputs.get("tra_cache_out").unwrap().try_extract_tensor::<f32>().unwrap();
        self.tra_cache = Array::from_shape_vec(self.tra_cache.raw_dim(), tc.1.to_vec()).unwrap();
        let ic = outputs.get("inter_cache_out").unwrap().try_extract_tensor::<f32>().unwrap();
        self.inter_cache = Array::from_shape_vec(self.inter_cache.raw_dim(), ic.1.to_vec()).unwrap();

        self.inv.synthesize(&out_bins)
    }
}

impl Denoiser for GtcrnDenoiser {
    fn process(&mut self, input: &[f32]) -> Vec<f32> {
        self.in_q.extend(input.iter().copied());
        let mut out = Vec::new();
        while self.in_q.len() >= HOP {
            let mut hop = [0.0f32; HOP];
            for v in hop.iter_mut() {
                *v = self.in_q.pop_front().unwrap();
            }
            let cleaned = self.run_frame(&hop);
            if self.warmup {
                self.warmup = false; // discard the first (warm-up) output frame
            } else {
                out.extend_from_slice(&cleaned);
            }
        }
        out
    }

    fn reset(&mut self) {
        self.fwd.reset();
        self.inv.reset();
        self.conv_cache.fill(0.0);
        self.tra_cache.fill(0.0);
        self.inter_cache.fill(0.0);
        self.in_q.clear();
        self.warmup = true;
    }
}
```

> **Implementer notes:** (1) Mirror the exact `ort` call forms used in `audio_toolkit/vad/silero_v6.rs` (`ort::inputs!`, `try_extract_tensor::<f32>()` returning `(shape, &[f32])`). (2) The `enh` flat-index assumes shape `[1,257,1,2]`; if Step 3 shows a different layout, index accordingly. (3) **Sign convention:** GTCRN was trained with `torch.stft(..., return_complex=False)` (imag = +Im); our `Stft` uses `realfft`'s native sign — if the SNR test fails or output is garbage, negate the imaginary part at the STFT boundary and re-test (this is the documented GTCRN gotcha).

- [ ] **Step 6: Run the test to verify it passes**

Run: `cd app/src-tauri && cargo test --lib gtcrn::tests 2>&1 | tail -20`
Expected: PASS. If it fails on the noise-floor assertion, first try the sign-convention fix (note 3) before touching the test.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/Cargo.lock app/src-tauri/resources/models/gtcrn_simple.onnx \
  app/src-tauri/src/audio_toolkit/denoise/mod.rs app/src-tauri/src/audio_toolkit/denoise/gtcrn.rs \
  app/src-tauri/src/audio_toolkit/mod.rs
git commit -m "feat(denoise): add GTCRN ONNX speech-enhancement denoiser"
```

---

### Task 3: Whole-clip loudness normalization

**Files:**
- Modify: `app/src-tauri/src/audio_toolkit/audio/gain.rs` (add `normalize_clip`)
- Test: extend the existing `#[cfg(test)]` in `gain.rs`

**Interfaces:**
- Produces: `pub fn normalize_clip(samples: &[f32], target_dbfs: f32, max_gain: f32) -> Vec<f32>` — compute the clip's RMS, scale by a single gain toward `target_dbfs` clamped to `[1.0, max_gain]` (boost-only), output clamped to `[-1.0, 1.0]`. Empty/near-silent clips returned unchanged.

- [ ] **Step 1: Write the failing tests** (add to the existing tests module in `gain.rs`):

```rust
#[test]
fn normalize_clip_boosts_quiet_clip_toward_target() {
    let quiet: Vec<f32> = (0..16000).map(|i| 0.02 * (i as f32 * 0.05).sin()).collect();
    let before = (quiet.iter().map(|s| s * s).sum::<f32>() / quiet.len() as f32).sqrt();
    let out = normalize_clip(&quiet, -20.0, 10.0);
    let after = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
    assert!(after > before * 2.0, "clip not boosted: {before} -> {after}");
    assert!(out.iter().all(|&s| (-1.0..=1.0).contains(&s)));
}

#[test]
fn normalize_clip_leaves_loud_clip_unchanged() {
    let loud: Vec<f32> = (0..16000).map(|i| 0.5 * (i as f32 * 0.05).sin()).collect();
    let out = normalize_clip(&loud, -20.0, 10.0);
    assert_eq!(out, loud);
}

#[test]
fn normalize_clip_handles_empty() {
    assert!(normalize_clip(&[], -20.0, 10.0).is_empty());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd app/src-tauri && cargo test --lib gain::tests::normalize_clip 2>&1 | tail -10`
Expected: FAIL — `cannot find function normalize_clip`.

- [ ] **Step 3: Implement** (add to `gain.rs`, above the test module):

```rust
/// Normalize a whole captured clip toward `target_dbfs` with a single gain,
/// clamped to `[1.0, max_gain]` (boost-only). Output clamped to [-1, 1].
pub fn normalize_clip(samples: &[f32], target_dbfs: f32, max_gain: f32) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    if rms <= 1e-9 {
        return samples.to_vec();
    }
    let target_rms = 10f32.powf(target_dbfs / 20.0);
    let gain = (target_rms / rms).clamp(1.0, max_gain);
    if gain == 1.0 {
        return samples.to_vec();
    }
    samples.iter().map(|s| (s * gain).clamp(-1.0, 1.0)).collect()
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd app/src-tauri && cargo test --lib gain::tests 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/audio_toolkit/audio/gain.rs
git commit -m "feat(denoise): add whole-clip loudness normalization"
```

---

### Task 4: `noise_suppression` setting

**Files:**
- Modify: `app/src-tauri/src/settings.rs` (field + default + tests)
- Modify: `app/src/bindings.ts` (regenerate / hand-add field)
- Test: inline `#[cfg(test)]` in `settings.rs`

**Interfaces:**
- Produces: `AppSettings.noise_suppression: bool` (default `true`), consumed by Task 5.

- [ ] **Step 1: Write the failing tests** (add a test module in `settings.rs`, mirroring `vad_threshold_tests`):

```rust
#[cfg(test)]
mod noise_suppression_tests {
    #[test]
    fn default_noise_suppression_is_true() {
        assert!(super::default_noise_suppression());
    }

    #[test]
    fn noise_suppression_round_trip() {
        #[derive(serde::Deserialize)]
        struct Probe {
            #[serde(default = "super::default_noise_suppression")]
            noise_suppression: bool,
        }
        let p: Probe = serde_json::from_str(r#"{"noise_suppression": false}"#).unwrap();
        assert!(!p.noise_suppression);
        let p2: Probe = serde_json::from_str("{}").unwrap();
        assert!(p2.noise_suppression);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd app/src-tauri && cargo test --lib noise_suppression 2>&1 | tail -10`
Expected: FAIL — `cannot find function default_noise_suppression`.

- [ ] **Step 3: Add the field + default**

In `AppSettings` (after `vad_threshold`):

```rust
    #[serde(default = "default_noise_suppression")]
    pub noise_suppression: bool,
```

Near the other `default_*` fns:

```rust
fn default_noise_suppression() -> bool {
    true
}
```

- [ ] **Step 4: Patch any explicit `AppSettings { ... }` literal**

Run: `cd app/src-tauri && cargo check 2>&1 | tail -20`. If the compiler reports `missing field noise_suppression` at a struct literal (e.g. `get_default_settings()`), add `noise_suppression: default_noise_suppression(),` there.

- [ ] **Step 5: Run tests**

Run: `cd app/src-tauri && cargo test --lib noise_suppression 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 6: Regenerate bindings**

`AppSettings` is exported to `app/src/bindings.ts` via tauri-specta. Run the generation (e.g. `cd app && bun run tauri dev` briefly, then stop) and confirm `noise_suppression?: boolean` appears in the `AppSettings` type. If the headless environment cannot run the GUI generation, hand-add `noise_suppression?: boolean` to the `AppSettings` type, matching the existing serde-default field style (e.g. `vad_threshold?: number`), and note it for later verification.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src/settings.rs app/src/bindings.ts
git commit -m "feat(denoise): persist noise_suppression setting (default on)"
```

---

### Task 5: Wire the denoiser into the recording pipeline

**Files:**
- Modify: `app/src-tauri/src/audio_toolkit/audio/recorder.rs` (denoise stage + 512-reframe)
- Modify: `app/src-tauri/src/managers/audio.rs` (construct denoiser, read setting, normalize before transcription)
- Test: `cargo check` + a toggle-off parity unit test + existing suites; real-hardware A/B is manual.

**Interfaces:**
- Consumes: `GtcrnDenoiser` (Task 2), `normalize_clip` (Task 3), `AppSettings.noise_suppression` (Task 4).
- Produces: a recorder that, when `noise_suppression` is on, denoises the 16 kHz stream before the VAD and normalizes the captured clip before returning it.

- [ ] **Step 1: Add an optional denoiser to the recorder**

In `recorder.rs`, the recorder holds an `Option<VAD>` already; add a parallel `Option<Box<dyn Denoiser>>` (import `crate::audio_toolkit::denoise::Denoiser`) and a `with_denoiser(...)` builder mirroring `with_vad(...)`. In the capture loop, the resampler currently emits 512-sample frames to `handle_frame`. Change the flow so that, **when a denoiser is present**, each resampled frame is first passed through `denoiser.process(frame)`, the returned cleaned samples are pushed into a re-frame buffer, and full 512-sample frames are pulled from that buffer and passed to the existing `handle_frame`/VAD path. When no denoiser is present, behavior is unchanged (frame goes straight to `handle_frame`).

Concretely, add a `reframe: Vec<f32>` accumulator in the consumer state and replace the direct `handle_frame(frame, ...)` call:

```rust
// inside the resampler emit closure, `frame` is the 512-sample resampled frame
let cleaned: Vec<f32> = match denoiser.as_ref() {
    Some(d) => d.lock().unwrap().process(frame),
    None => frame.to_vec(),
};
reframe.extend_from_slice(&cleaned);
while reframe.len() >= 512 {
    let block: Vec<f32> = reframe.drain(..512).collect();
    handle_frame(&block, recording, &vad, &mut processed_samples);
}
```

Call `denoiser.lock().unwrap().reset()` wherever the VAD is reset (`Cmd::Start`), so denoiser state does not carry across utterances.

- [ ] **Step 2: Construct the denoiser and gate it in `managers/audio.rs`**

In `create_audio_recorder`, resolve `gtcrn_simple.onnx` (same `resolve(... BaseDirectory::Resource)` pattern as the VAD model) and build the denoiser only when the setting is on. Add a `noise_suppression: bool` parameter to `create_audio_recorder` and at its call site read `settings.noise_suppression` (alongside the existing `settings.vad_threshold`). When on, attach the denoiser via `.with_denoiser(Box::new(GtcrnDenoiser::new(denoise_path)?))`; when off, do not attach it.

- [ ] **Step 3: Normalize the captured clip before transcription**

Find where `stop_recording` returns the captured `Vec<f32>` and it is handed to transcription. When `noise_suppression` is on, apply `crate::audio_toolkit::audio::gain::normalize_clip(&samples, -20.0, 10.0)` to that buffer before transcription. (When off, pass the samples through unchanged.) Keep this in `managers/audio.rs`/the transcription hand-off, not inside the recorder.

- [ ] **Step 4: Toggle-off parity test**

Add a unit test (in `managers/audio.rs` or a small `recorder.rs` test) that runs a fixed sample buffer through the re-frame/`handle_frame` path with `denoiser = None` and asserts the bytes reaching `processed_samples` equal the input frames unchanged (no denoise/normalize when off). If the re-frame logic is not unit-accessible, extract the `process-or-passthrough + reframe` into a small free function and test that function directly.

- [ ] **Step 5: Build + tests**

Run: `cd app/src-tauri && cargo check 2>&1 | tail -15 && cargo test --lib 2>&1 | tail -15`
Expected: clean build, all tests pass (including the new parity test and the existing `gtcrn`/`stft`/`gain`/`silero_v6` suites).

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/audio_toolkit/audio/recorder.rs app/src-tauri/src/managers/audio.rs
git commit -m "feat(denoise): wire GTCRN denoiser + clip normalize into the capture pipeline"
```

- [ ] **Step 7: Manual A/B (real hardware, documented in the PR)**

On a real machine with `noise_suppression` on vs off, dictate the same sentence in a noisy room and compare transcription. Record the A/B result in the PR — this is the acceptance gate for the ASR-routing default.

---

### Task 6: Settings toggle (frontend) + model-setup docs

**Files:**
- Modify: a settings component under `app/src/components/settings/` (Sound section)
- Modify: `app/src/i18n/locales/en/translation.json`
- Modify: `app/AGENTS.md` (download the GTCRN model in dev setup)
- Test: `bun run lint` + manual

**Interfaces:**
- Consumes: `AppSettings.noise_suppression` (Task 4) via the settings hook/store.

- [ ] **Step 1: Add i18n strings** to `app/src/i18n/locales/en/translation.json` under the Sound settings section (match the nesting used by the mic-sensitivity entry):

```json
"noiseSuppression": {
  "title": "Noise suppression",
  "description": "Automatically remove background noise so your voice is captured clearly."
}
```

- [ ] **Step 2: Add the toggle** in the Sound settings component (find where the mic-sensitivity slider / `MuteWhileRecording` toggle render). Reuse the exact toggle idiom and the `useSettings` `getSetting`/`updateSetting` pattern those use:

```tsx
// reuse the existing Toggle/Switch component + useSettings hook idiom in this file
const enabled = getSetting("noise_suppression") ?? true;
<ToggleRow
  title={t("settings.sound.noiseSuppression.title")}
  description={t("settings.sound.noiseSuppression.description")}
  checked={enabled}
  onChange={(v) => updateSetting("noise_suppression", v)}
/>
```

Match the actual component name/props used by neighboring toggles (e.g. `MuteWhileRecording`); do not invent a new pattern.

- [ ] **Step 3: Update the dev model-setup doc** in `app/AGENTS.md` — add, next to the Silero v6 download:

```bash
# GTCRN noise-suppression model (MIT)
curl -L -o src-tauri/resources/models/gtcrn_simple.onnx \
  https://github.com/Xiaobin-Rong/gtcrn/raw/main/stream/onnx_models/gtcrn_simple.onnx
```

- [ ] **Step 4: Lint**

Run: `cd app && bun run lint 2>&1 | tail -15`
Expected: no new errors (all strings i18n-keyed).

- [ ] **Step 5: Commit**

```bash
git add app/src/components/settings app/src/i18n/locales/en/translation.json app/AGENTS.md
git commit -m "feat(denoise): add noise-suppression toggle and dev model-setup doc"
```

---

## Self-Review

**Spec coverage:**
- GTCRN engine + ONNX via ort → Task 2. ✓
- 16 kHz STFT/iSTFT front-end → Task 1. ✓
- Denoise → both VAD and ASR, one toggle → Task 5 (wiring) + Task 4 (setting) + Task 6 (toggle). ✓
- Whole-clip normalize after denoise → Task 3 + Task 5 Step 3. ✓
- Toggle off = byte-identical → Task 5 Step 4 (parity test). ✓
- Model download/gitignore/bundle + AGENTS.md → Task 2 Step 2 + Task 6 Step 3. ✓
- CPU-only / ort pin / MIT license → Global Constraints. ✓
- Real-hardware A/B acceptance gate → Task 5 Step 7. ✓
- Sign-convention gotcha → Task 2 Step 5 note 3 + covered by the SNR test. ✓

**Placeholder scan:** No TBD/TODO. The model output name (`enh`) and tensor shapes are explicitly verified live in Task 2 Step 3 before use — that is a mandated verification step, not a placeholder. The recorder re-frame integration (Task 5 Step 1) gives the concrete accumulator code; exact insertion lines are located against the real file because `recorder.rs`'s consumer loop must be matched as-is.

**Type consistency:** `Stft::{analyze,synthesize,reset}` defined in Task 1 are used identically in Task 2. `Denoiser::{process,reset}` defined in Task 2 is used in Task 5. `normalize_clip(samples, target_dbfs, max_gain)` defined in Task 3 is called with `(-20.0, 10.0)` in Task 5 Step 3. `noise_suppression: bool` (default true) is consistent across Tasks 4–6. Model filename `gtcrn_simple.onnx` consistent across Tasks 2 and 6. STFT constants (512/256/257) consistent across Tasks 1–2.
