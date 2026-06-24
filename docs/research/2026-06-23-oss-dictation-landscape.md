# Open-Source Local Dictation Landscape — Building Blocks for MindFlow

*Research compiled 2026-06-23. Star counts verified via GitHub API on that date unless flagged. Uncertain claims marked.*

## Project profiles (condensed)

| Project | Repo | License | Platforms | Stack | Stars | CPU STT | Local LLM cleanup | Standout |
|---|---|---|---|---|---|---|---|---|
| **Handy** | cjpais/Handy | **MIT** | Win/Mac/Linux | Rust+Tauri+React | ~24,650 | Yes (Parakeet V3 ~5x RT) | Partial/experimental (debug menu) | **Best forkable, fully-local base; best Linux/Wayland; global hotkey + injection** |
| **Whispering** | EpicenterHQ/epicenter | AGPL-3.0 | Win/Mac/Linux+web | Svelte5+Tauri | ~4,645 | Yes (whisper.cpp/Parakeet/Moonshine) | **Yes — best chainable transform pipeline (Ollama/LM Studio/vLLM)** | Best AI post-processing |
| **VoiceInk** | Beingpax/VoiceInk | GPL-3.0 | macOS only | Swift/SwiftUI | ~5,332 | Yes (whisper.cpp/Parakeet/Apple) | **Yes (Ollama)** | **Best context-aware "Power Mode"; best dictionary UX** |
| **Hyprnote/OWhisper** | fastrepl/hyprnote | MIT | Win/Mac/Linux | Tauri Rust/React | ~8,695 | Yes (whisper.cpp+Moonshine) | **Yes (core; HyprLLM Qwen3 1.7B)** | Best local meeting pipeline; "Ollama for STT" server |
| **Vibe** | thewh1teagle/vibe | MIT | Win/Mac/Linux | Rust+Tauri | ~6,538 | Yes (whisper.cpp, AVX2) | Optional (Ollama) | Best file/media transcription UI + diarization (NOT injection) |
| **WhisperWriter** | savbell/whisper-writer | GPL-3.0 | Win/Mac/Linux | Python+PyQt5 | ~1,069 | Yes (faster-whisper) | No | 4 recording modes — STALE (idle since 2024-08) |
| **nerd-dictation** | ideasman42/nerd-dictation | GPL-3.0 | Linux only | Python (1 file) | ~1,878 | Yes (VOSK) | No | Lightweight low-latency Linux; broadest Linux injectors |
| **Speech Note** | mkiol/dsnote | MPL-2.0 | Linux/Sailfish | C++/Qt | ~1,516 | Yes (5 engines) | No | Best offline engine breadth + TTS + NMT |
| **BlahST** | QuantiusBenignus/BlahST | BSD-3 | Linux only | zsh | ~174 | Yes (whisper.cpp) | Yes (llama.cpp) | Leanest Linux injection — dormant |
| **Buzz** | chidiwilliams/buzz | MIT | Win/Mac/Linux | Python+PyQt | ~19,806 | Yes | Partial/unconfirmed | Best file/YouTube/live transcription UI (NOT injection) |
| **WhisperLive** | collabora/WhisperLive | MIT | cross + browser | Python WS server | ~4,104 | Yes (faster-whisper/OpenVINO) | No | Best streaming-STT server / multi-backend (library, not app) |

**Other notable:** OpenWhispr (MIT, Electron, 3,942★), Amical (MIT, local LLM formatting via Ollama, 1,378★), Murmure (AGPL, Parakeet+LLM, 878★), VoiceTypr (source-available paid, 388★). Smaller macOS: FluidVoice, Voquill, MacParakeet.

**Closed/excluded as base:** Wispr Flow, Aqua Voice, Superwhisper, MacWhisper, Willow Voice, Spokenly, FUTO Voice Input, Talon.

## A) Best-in-class by capability

| Capability | Best | Why |
|---|---|---|
| Text injection | Handy (cross-platform), nerd-dictation/BlahST (Linux) | `enigo` type-or-paste; widest Linux injectors |
| Global hotkey | Handy | tauri-plugin-global-shortcut + rdev fork; CLI/signal fallback |
| CPU STT engine | Handy (Parakeet V3) / whisper.cpp / **sherpa-onnx** runtime | Parakeet ~5x RT; sherpa-onnx = one runtime, true streaming |
| Local LLM formatting | Whispering / VoiceInk / Hyprnote | chainable transforms / Power Mode / bundled HyprLLM |
| Custom vocabulary | VoiceInk | dictionary + word replacement + context prompts |
| Cross-platform UI | Handy (dictation); Buzz/Vibe (files) | cleanest cross-platform dictation GUI |
| Model management | Vibe / Speech Note / OWhisper | in-app download; "Ollama for STT" |
| Command mode | weak in OSS; closest VoiceInk / BlahST | no full match for Wispr Command Mode |
| Streaming server | WhisperLive / OWhisper | multi-backend WS streaming; Deepgram-compatible local |

## B) Recommended foundation

**Primary base to fork: Handy (cjpais/Handy).** MIT (permissive), Rust+Tauri, truly cross-platform, already solves global hotkey + push-to-talk + text injection (`enigo`) + VAD, explicitly "most forkable", CPU-first STT (Parakeet V3 ~5x RT), largest community + active. Missing piece = first-class AI formatting → exactly the layer to add.

**Reference for AI layer: Whispering** (chainable local-LLM transforms; AGPL → reimplement pattern, don't lift code). **VoiceInk** for Power Mode + dictionary UX (GPL → reference only). **OWhisper** for local model-server pattern.

**Stack: Tauri + Rust** (every active cross-platform dictation app uses it; small binaries, native hotkey/injection).
- STT runtime: **sherpa-onnx** (Moonshine/Vosk/Whisper/Parakeet, streaming) and/or whisper.cpp via `whisper-rs`. Default Moonshine-streaming or Parakeet V3 for CPU real-time; whisper.cpp small as accuracy fallback.
- Local LLM cleanup: **Ollama** or bundled `llama-server`; default Llama 3.2 3B or Gemma 2/3 2B @ Q4_K_M. UX: insert raw text immediately, refine asynchronously, only refine longer utterances.
- Injection/hotkey: `enigo` + tauri-plugin-global-shortcut + rustdesk `rdev` fork.

**Implementation risks:** Wayland (hotkey+injection messy — needs portals/libei or wtype/ydotool); macOS TCC permissions + Sequoia/Tahoe synthetic-event filtering; Windows UIPI (SendInput fails into elevated windows; use KEYEVENTF_UNICODE); keyboard-layout/Unicode correctness (prefer clipboard-paste).

## Licensing note for forking

Permissive (safe to fork): Handy, Vibe, Hyprnote, Buzz, WhisperLive, OpenWhispr, Amical (all MIT). Copyleft (reference only, don't lift): Whispering (AGPL), VoiceInk/WhisperWriter/nerd-dictation/Murmure (GPL/AGPL). MPL/BSD: Speech Note, BlahST.
