# MindFlow Noise Suppression (GTCRN Audio Front-End) — Design Spec

*Brainstormed 2026-06-25. Adds an automatic noise-suppression + auto-level front-end so MindFlow captures usable voice on real-world mics in noisy rooms, with no manual audio setup.*

## Problem

Real-world testing showed transcription garbling whose root cause was **input audio quality**: the microphone captured more background noise than the user's voice (low SNR). No local ASR model can transcribe noise-dominated audio. Telling users to fix Windows mic settings / buy a headset is not a product. MindFlow needs to clean the input automatically, the way every "it just works" voice app (Zoom, Discord, Wispr Flow) does.

The earlier per-frame gain experiment (now reverted) proved a key lesson: **you cannot just amplify** — that boosts noise equally. The fix must *separate voice from noise* (noise suppression), then optionally normalize level.

## Decisions (from brainstorming)

1. **Engine class:** lightweight ML denoiser that runs real-time on any CPU. → **GTCRN**.
2. **Routing:** the cleaned audio feeds **both** the VAD and the transcription model, controlled by **one toggle**, default ON. Validated by the user's real-world A/B test; the toggle is the escape hatch. (Rationale and the ASR caveat below.)
3. **Controls:** automatic, single "Noise suppression" on/off toggle. The existing mic-sensitivity slider stays as an advanced control.

## Critical finding that shaped the design (ASR vs VAD)

Deep research (multiple peer-reviewed 2024–2026 studies, including tests on **Parakeet-TDT**) found that feeding a single-channel neural denoiser *into* a modern, noise-robust ASR model often **raises** WER — the denoiser's non-linear artifacts are out-of-distribution for ASR trained on real noisy speech. PESQ/DNSMOS improve while WER worsens; they do not correlate. **However**, denoising reliably **helps a VAD** (Silero tolerates artifacts and benefits from higher SNR).

Implication: denoising is an unambiguous win for the VAD path; for the ASR path it is environment-dependent — a clear win in *severe* noise (voice buried, the user's case), a possible regression in clean/moderate noise. We therefore:

- **Always** route denoised audio to the VAD.
- Route denoised audio to ASR **by default** (the user's problem is severe noise), but make it the **same single toggle** so it can be turned off, and treat the real-hardware A/B test as the acceptance gate. If A/B shows a regression in the user's environment, the default can be revisited.

## Chosen engine: GTCRN

**Grouped Temporal Convolutional Recurrent Network** (Xiaobin-Rong/gtcrn, ICASSP 2024).

| Property | Value |
|---|---|
| Model | `gtcrn_simple.onnx`, ~523 KB (535,190 bytes) |
| Params | ~24–48K (tiny) |
| Sample rate | **16 kHz native** (no resampling — matches our pipeline) |
| Speed | RTF ~0.07 on a desktop core (≈14× real-time; trivial on weak laptops) |
| Streaming | Fully causal, frame-by-frame, exposed state caches (same pattern as Silero v6) |
| Runtime | ONNX, opset 11, no complex/custom ops → runs on our pinned `ort = "=2.0.0-rc.12"` |
| License | **MIT** (weights + code) |
| Reference | sherpa-onnx ships a ready model + complete C++ streaming implementation to port |

**Model source (prefer the sherpa-onnx packaged build — it carries STFT metadata):**
`https://github.com/k2-fsa/sherpa-onnx/releases/tag/speech-enhancement-models` (or HF `csukuangfj/speech-enhancement-models`). Upstream raw: `https://github.com/Xiaobin-Rong/gtcrn/raw/main/stream/onnx_models/gtcrn_simple.onnx`. The exact SHA256 and tensor I/O **must be verified against the downloaded model at implementation time** (lesson from the Silero v6 `sr` input that `strings` inspection missed — introspect the real ONNX session).

**Runners-up considered:** DeepFilterNet 3 (better quality, but 48 kHz → forces resampling + heavier integration; MIT/Apache); DTLN (fine 16 kHz fallback, MIT, but two stacked models). GTCRN wins on fit. Avoid: Facebook/Demucs denoiser (CC-BY-NC, non-commercial), NSNet2 (state not exposed → not streamable as shipped), CRN/FSPEN (unlicensed).

### GTCRN integration facts (from research — verify against the real model)

- **STFT (done in Rust, outside the model):** n_fft = 512, hop = 256 (50% overlap, 16 ms hop), window = **sqrt-Hann** for both analysis and synthesis (WOLA, unity gain at 50% overlap), 257 one-sided bins. Implement with `realfft` (not sherpa's O(n²) DFT).
- **Tensor I/O:**
  - Inputs: `mix [1,257,1,2]`, `conv_cache [2,1,16,16,33]`, `tra_cache [2,3,1,1,16]`, `inter_cache [2,1,33,16]`.
  - Outputs: `enh [1,257,1,2]`, and the three caches (same shapes) as next-state.
  - `mix`/`enh` layout = `[batch, freq=257, time=1, complex=2]`; real at `[...,0]`, imag at `[...,1]`.
- **State threading:** zero-init all three caches at stream start; feed each output cache back as the next frame's input; re-zero on `reset()` / utterance boundary. (Identical pattern to our Silero v6 `state`/`stateN`.)
- **Causality/latency:** one STFT frame per call, latency = the 512-sample window (32 ms), no extra lookahead. The first output frame is warm-up (suppress/discard).
- **Sign convention gotcha:** sherpa's reference stores imag as `−Im`; the model was trained against `torch.stft(..., return_complex=False)` (imag = `+Im`). The Rust STFT must match training, or output is garbage — covered by a targeted test.
- **Rust references to port:** sherpa-onnx `csrc/online-speech-denoiser-stft-impl.h`, `online-speech-denoiser-gtcrn-impl.h`, `offline-speech-denoiser-gtcrn-model.cc`, `scripts/gtcrn/add_meta_data.py`.

## Architecture & data flow

```
mic (cpal, native rate, multi-channel)
  → mono downmix                                   (existing)
  → resample to 16 kHz                             (existing)
  → [GTCRN denoise]   stream-in/out, 256-hop WOLA  (NEW; skipped if toggle off)
  → cleaned 16 kHz stream
      ├─ re-framed to 512 → Silero v6 VAD   (detect on cleaned audio)
      └─ speech frames → output buffer (cleaned)
  on stop: output buffer
      → [whole-clip loudness normalize]            (NEW; one gain factor for the clip)
      → Parakeet v2 transcription
```

- **Two principles (lessons from the failed gain):** denoise *before* gain (never amplify noise); gain is **one factor for the whole captured clip**, not per-frame (no pumping, no warping the VAD's temporal input).
- **Toggle OFF** → denoise and normalize are skipped; the pipeline is byte-identical to current behavior.
- The denoiser is a clean **stream stage**: it consumes the 16 kHz stream in 256-sample hops and emits a cleaned 16 kHz stream. The VAD keeps its existing **512-sample** frame contract by re-framing the *cleaned* stream. The same cleaned stream is what gets buffered for transcription (the "denoise → both" decision means one cleaned copy serves both consumers).

## Components

| Module | Responsibility | New/changed |
|---|---|---|
| `audio_toolkit/audio/stft.rs` | Streaming sqrt-Hann STFT/iSTFT (WOLA), n_fft 512 / hop 256 / 257 bins, via `realfft`; forward (window→rFFT) and inverse (irFFT→window→overlap-add) with internal overlap state. | NEW |
| `audio_toolkit/denoise/mod.rs` | `trait Denoiser { fn process(&mut self, frame: &[f32]) -> Vec<f32>; fn reset(&mut self); }` + module wiring. | NEW |
| `audio_toolkit/denoise/gtcrn.rs` | `GtcrnDenoiser`: holds `ort` Session + 3 state caches; per call does STFT → pack `{1,257,1,2}` → `session.run` → unpack `enh` → iSTFT → emit cleaned samples; threads/zeros caches. Mirrors `SileroV6Vad`. Reuses `ort` + `ndarray`. | NEW |
| `audio_toolkit/audio/gain.rs` | Add `normalize_clip(samples, target_dbfs)` — one gain factor for the whole denoised clip, clamped, applied before transcription. Reuses the existing tested helper. | EXTEND |
| `audio_toolkit/audio/recorder.rs` + `managers/audio.rs` | Insert the denoiser stage between resample-to-16k and the VAD framing; gate it on the setting; apply clip normalize on the captured speech buffer before transcription. | CHANGE |
| `settings.rs` | `noise_suppression: bool` (default `true`) with the existing `#[serde(default = "…")]` idiom; regenerate `bindings.ts`. | CHANGE |
| `src/components/settings/...` + i18n | "Noise suppression" toggle in Settings → Sound. | CHANGE |
| model + `AGENTS.md` | `gtcrn_simple.onnx` downloaded (gitignored like the VAD model, auto-bundled by the `resources/**` glob); dev-setup doc updated to fetch it. | CHANGE |

## Settings & controls

- `noise_suppression: bool`, default `true`. One toggle in Settings → Sound controls denoising for both the VAD and transcription paths.
- The existing `vad_threshold` mic-sensitivity slider remains as an advanced control. Note the synergy: on *cleaned* audio it is safe to lower the threshold (more sensitive) because there is no noise to false-trigger — so denoising makes the sensitivity control behave more predictably.

## Testing & success criteria

- **STFT/iSTFT round-trip** (unit, CI): analyze→synthesize reconstructs a test signal within tolerance — proves WOLA unity gain before any model runs.
- **Sign-convention check** (unit, CI): one reference frame through STFT → assert the real/imag convention matches GTCRN's training expectation (the `−Im` vs `+Im` gotcha).
- **Denoiser SNR test** (unit, CI, no mic): mix `tests/fixtures/jfk.wav` with known noise at a set SNR, run GTCRN frame-by-frame, assert output SNR (or correlation with the clean reference) improves vs the noisy input; plus a no-NaN / stable-state check across the full clip.
- **Toggle-off parity** (unit/integration): with `noise_suppression = false`, the captured buffer is byte-identical to the current pipeline.
- **Real-hardware A/B (acceptance gate, user-run):** denoised-vs-original transcription on the user's noisy setup — does transcription become usable? This is the decision criterion for the ASR-routing default.

## Risks & mitigations

- **Artifacts hurting clean-audio ASR** → single toggle + the user's A/B test; default revisitable if A/B regresses.
- **Integration complexity** (STFT/iSTFT, state threading, 256-hop vs 512-frame framing, sign convention) → port from the sherpa-onnx C++ reference; land and test the STFT round-trip *first*, then the model.
- **Model I/O drift** (the v6 `sr` lesson) → implementer introspects the real ONNX session and verifies tensor names/shapes + SHA on download before wiring.
- **CPU** → GTCRN RTF ~0.07; negligible even with the VAD + ASR also running.

## Global constraints

- CPU-only, fully local, no network at runtime (model downloaded once at setup). No GPU features.
- `ort = "=2.0.0-rc.12"`, `ndarray = "0.17"` (already in tree); add `realfft` for STFT.
- Free/permissive license only: GTCRN is MIT.
- 16 kHz mono throughout; denoiser STFT n_fft 512 / hop 256 / sqrt-Hann; VAD keeps 512-sample frames.
- Builds on the current `main` shape plus the merged VAD work (Silero v6, no per-frame gain, threshold 0.5). The `mindflow-vad-gain-fix` branch (gain removal) should land before or as the base of this work.

## Out of scope (YAGNI)

- Strength/aggressiveness controls (single on/off only).
- A second denoiser tier (DeepFilterNet) — revisit only if GTCRN proves too weak in testing.
- Echo cancellation, beamforming, multi-channel — single-channel only.
- Blend/observation-adding for ASR — only if the simple A/B shows denoise-into-ASR regresses and a middle ground is needed.
