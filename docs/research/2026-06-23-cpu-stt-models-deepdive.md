# CPU STT Models Deep Dive (2026)

*Scope: smallest + fastest + latest + best-accuracy ASR on the CPU speed↔accuracy Pareto frontier for live dictation. CPU vs GPU flagged on every figure.*

## Methodology warnings
- **HF Open ASR Leaderboard `RTFx` is GPU** (A100, batched) — used for WER only, never CPU speed.
- Only genuine first-party CPU numbers: sherpa-onnx RTF tables (ARM), faster-whisper README (i7-12700K), whisper.cpp bench. Most "Nx faster" marketing is GPU.
- RTF = processing-time ÷ audio-duration; <1 = faster than real-time. Want RTF ≤ ~0.3 for comfortable live dictation.

## Key models (CPU-focused)

| Model | Size (int8) | License | WER (OpenASR avg ‖ LS clean/other) | CPU RTF | Streaming | Langs | Formats |
|---|---|---|---|---|---|---|---|
| **Parakeet-TDT-0.6B-v2** | ~630 MB | CC-BY-4.0 | **6.05 ‖ 1.69/3.19** | 0.22→0.088 (ARM 1→4thr); ~0.03-0.05 x86 | chunked batch | English | ONNX, GGUF |
| **Parakeet-TDT-0.6B-v3** | ~640 MB | CC-BY-4.0 | 6.34 | ~same | chunked batch | **25 EU, auto-LID** | ONNX, GGUF |
| **Moonshine Tiny** | 125 MB | **MIT** | ‖ 4.52/11.71 | 34ms(Mac)/69ms(x86)/clip | **stream variant** | English | ONNX |
| **Moonshine Base** | 250 MB | **MIT** | ‖ 3.23/8.18 | low | **stream variant** | English | ONNX |
| **Moonshine Medium-Streaming** (new) | 245M | **MIT** | 6.65 short-form (beats whisper-v3 7.44) | edge-targeted | **streaming** | English | ONNX |
| **Streaming Zipformer 20M** | ~20-30 MB | Apache-2.0 | ‖ 3.88 clean | **0.06-0.13** | **true streaming RNN-T** | English | ONNX |
| **faster-whisper small.en int8** | ~250 MB | MIT | ‖ ~3.05 clean | **0.13 (7.6x RT)** i7-12700K | batch+VAD | En | CTranslate2 |
| **whisper large-v3-turbo** | 547 MB q5 | MIT | 7.83 | borderline-RT x86 | batch+VAD | 99 + auto-LID | GGUF, CT2, ONNX |
| **SenseVoice-Small** | 228 MB | Custom (non-MIT) | ‖ ~2.57 clean | 0.099 (ARM 1-core) | non-streaming | 5 (zh/yue/en/ja/ko)+emotion | ONNX, GGUF |
| **Vosk small-en** | 40 MB | Apache-2.0 | ‖ 9.85 | ~RT Pi3B | **true streaming** | 20+ (no auto-LID) | Kaldi |
| Canary-Qwen-2.5B / Kyutai / Qwen3-ASR / Granite | 1-2.5B | varies | **5.5-5.8 (best)** | **GPU-only, no CPU runtime** | varies | varies | — |

## Key insight
**Parakeet-TDT-0.6B-v2 dominates the entire Whisper/distil-Whisper English lineup on CPU** — at ~600M it beats large-v3 (7.44), turbo (7.83), distil-large-v3.5 (7.21) on accuracy (6.05) AND runs fast via sherpa-onnx/ORT int8. Whisper only stays relevant for 99-language coverage or mature streaming wrappers. The true accuracy leaders (Canary, Granite, Qwen3-ASR) are **GPU-only — no CPU path** — so not usable today.

## Recommended three tiers
- **⚡ Turbo** (low-end laptops/SBC): **Streaming Zipformer 20M** (Apache-2.0) or **Moonshine Tiny** (MIT). ONNX. RTF ~0.06-0.13, true streaming, ~20-125 MB.
- **⭐ Balanced (DEFAULT)**: **Parakeet-TDT-0.6B-v2** (English) / **-v3** (25-lang auto-detect). ONNX int8, chunked + VAD. RTF ~0.03-0.09, WER 6.05/1.69. Fallback: faster-whisper small.en int8.
- **🎯 Max Accuracy** (strong CPU): Parakeet-v2 remains sweet spot; whisper large-v3-turbo (q5 GGUF) for 99-lang.

## Watch-list (latest, upgradable)
Moonshine Medium-Streaming (245M, MIT, ONNX, ready); Parakeet-v3 (25-lang, ready); NVIDIA Nemotron streaming (80-160ms, GPU today→watch ONNX); Kyutai STT (MLX-only); Qwen3-ASR-1.7B (GPU); IBM Granite Speech 4.0-1B (5.52, GPU); Voxtral-Mini-3B (GGUF emerging); Meta Omnilingual (1600+ langs, GPU).
