# CPU STT Inference Runtimes for Rust+Tauri Deep Dive (2026)

*Scope: embeddable-from-Rust runtimes for low-latency CPU-only streaming dictation, cross-platform Win/Mac/Linux. CPU/GPU kept separate.*

## Three dominant findings
1. **"Streaming" splits in two.** Only **sherpa-onnx (online Zipformer/Paraformer)** and **Vosk (Kaldi)** do *true* incremental streaming. Everything Whisper-based (whisper.cpp, faster-whisper, CTranslate2, OpenVINO, Candle) is a **30s-window offline model** with a ~3.3s latency floor; their "streaming" is sliding-window/VAD chunking.
2. **The real apps converged on ONE crate.** Both **Handy** and **Whispering** route local STT through **`transcribe-rs`** (cjpais) = **whisper.cpp (`whisper-rs`) + ONNX Runtime (`ort`)** — NOT sherpa-onnx — shipping **Parakeet-on-ONNX as the CPU/no-GPU default** + Whisper as multilingual/GPU path.
3. **Binding landscape shifted 2026.** Official **k2-fsa `sherpa-onnx` crate v1.13.3** now exists; community `sherpa-rs` **archived**; whisper.cpp moved to `ggml-org/whisper.cpp` v1.9.1 (Parakeet support added v1.9.0).

## Comparison (condensed)

| Runtime | Rust binding | True streaming? | CPU perf | Notes |
|---|---|---|---|---|
| **sherpa-onnx** | Official crate v1.13.3 (Apache-2.0), best official binding | **YES** (online Zipformer/Paraformer only) | Streaming Zipformer int8 RTF 0.04-0.15; ~44ms latency M-series | Also runs offline Parakeet/Moonshine/SenseVoice/Whisper via VAD |
| **whisper.cpp + whisper-rs** | whisper-rs v0.16 (Unlicense), mature, static link | **NO** (sliding window) | ~15x RT base, ~6x RT small (x86); Q4_0 fastest | GGML .bin; Parakeet added v1.9.0; no Moonshine; single binary |
| **faster-whisper / CTranslate2** | ct2rs v0.9 (single maintainer, cxx+CMake) | **NO** | ~1.23x un-batched vs whisper.cpp fp32, +41% RAM | faster-whisper itself Python + stagnating |
| **OpenVINO** | openvino crate (single maintainer, pre-1.0, version-locked) | **NO** | 2-6x INT8 vs PyTorch only | **No INT8 speedup on Apple Silicon; Win-on-ARM unsupported** — disqualifiers |
| **ONNX Runtime (`ort`)** | ort 2.0.0-rc.12 (dominant Rust ONNX, ~12M dl) | inference only (DIY loop) | runs Parakeet/Moonshine on CPU | used UNDER transcribe-rs; RC churns import paths |
| **Vosk (Kaldi)** | vosk v0.3 (thin stable FFI) | **YES native** | excellent CPU, real-time 1 core | accuracy below Whisper/Parakeet; ship libvosk yourself |
| **Candle / Burn** (pure Rust) | native | **NO** (batch) | heavy CPU Whisper / not production | immature for low-latency dictation |

## Recommendation
- **Default / safest (mirror Handy):** **`transcribe-rs`** — Parakeet (ONNX/CPU) primary + whisper.cpp multilingual fallback, VAD-gated push-to-talk. One crate, two engines, proven by BOTH Handy and Whispering. Push-to-talk/paste-on-release; latency = utterance + one inference (sub-second to few seconds). **If push-to-talk is acceptable, this is the answer.**
- **If true live partials (words as you speak) are a hard requirement:** add **official `sherpa-onnx` crate** streaming Zipformer for partials + finalize the utterance with an offline Parakeet/Whisper accuracy pass.
- **Avoid:** OpenVINO (ARM/Win-ARM gaps, no streaming, single-maintainer), CTranslate2/faster-whisper (no streaming, Python-centric, stagnating), Candle/Burn (batch-only, immature).

## Integration risks
1. **Windows MSVC C-runtime conflict (will bite):** `whisper-rs-sys` + `tokenizers` use incompatible MSVC CRT linking; can't coexist in one Windows binary — why Whispering ships **Parakeet-only on Windows**. Plan: ONNX/Parakeet = Windows-CPU baseline; feature-gate whisper.cpp per-platform. Also handle "vulkan-1.dll not found" crash on CPU-only Windows (don't require GPU backend at load).
2. **Streaming chunking:** Whisper sliding-window re-transcribes at boundaries (duplicated/revised words) → need LocalAgreement confirmation layer + accept ~3.3s floor. Push-to-talk sidesteps this. True streaming only via sherpa-onnx online models.
3. **Binding maturity:** pin `ort =2.0.0-rc.12` exactly (import paths churn); use official sherpa-onnx crate (sherpa-rs archived); ct2rs/openvino single-maintainer; vosk crate doesn't bundle libvosk.
4. **Threading:** run inference off UI/Tauri main thread (worker + channels); tune thread count vs cores for capture/UI.
5. **Model management (roll your own):** custom HTTP downloader, resumable + SHA256 + tar.gz extract, per-platform app-data dirs, manual-download fallback. Plus a separate Silero VAD model. Download-on-first-run (models 20MB-1.5GB), don't embed.
6. **Packaging:** whisper.cpp static-links (clean single binary); ONNX Runtime ships ~15-17MB native lib (static-link or minimal build). Enable CPU SIMD (AVX/AVX2/FMA x86, NEON ARM), disable all GPU features for CPU build. Tauri shell ~8MB.
