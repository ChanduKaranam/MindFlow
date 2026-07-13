# Changelog

All notable changes to MindFlow are documented here.

## Unreleased — M7 accuracy stack

### Accuracy
- **Local LLM cleanup** — an always-on, on-device Qwen3 cleanup pass rewrites
  fillers, punctuation, and self-corrections in the transcript. Fallback-safe:
  if the model isn't downloaded or generation times out, dictation silently
  falls back to the existing rules-only formatting with no error surfaced.
- **Name correction** — Double-Metaphone phonetic matching against the
  custom-word dictionary, upgraded from exact match, with a 10k common-word
  guard so everyday words are never "corrected" into a dictionary name.
  Applies to transcripts from **all** STT engines (Whisper, Parakeet,
  Moonshine), not just one.
- **Phrase-loop hallucination collapse** — collapses repeating multi-word
  phrase loops (a known STT hallucination pattern) down to a single instance.

### Personalization
- **History transcript editing** — edit a mangled transcript directly in
  History.
- **Dictionary auto-learn** — editing a name correction in History
  auto-learns the corrected spelling into the custom-word dictionary
  (visible, reversible — undo removes the learned entry).

### Onboarding & settings
- New onboarding step offers the optional cleanup-model download (skippable).
- New **AI cleanup** settings card: on/off flags, model download/management,
  and name-correction sensitivity.

### Known issues
- **macOS (aarch64) builds currently fail at link time.** `whisper-rs-sys`
  and `llama-cpp-sys-2` each vendor a full static `ggml`; `build.rs` works
  around the resulting duplicate-symbol clash with
  `-Wl,--allow-multiple-definition` (GNU ld) / `/FORCE:MULTIPLE` (MSVC), but
  `ld64` has no direct equivalent. **The macOS build must be verified/fixed
  before any macOS release of this branch.** See the comment in
  `app/src-tauri/build.rs`.

## v1.0.0 — first release

MindFlow's first public release: a **free, fully-local, CPU-only, cross-platform
voice dictation** app — a Wispr Flow–style tool built on
[Handy](https://github.com/cjpais/Handy) by cjpais.

### Core
- **Dictate into any app** via a global hotkey; transcribed text is injected at
  the cursor (clipboard-paste fallback).
- **Local CPU speech-to-text** — Whisper / Parakeet / Moonshine via ONNX/whisper,
  with an automatic model tier picked for your CPU. No GPU required.
- **Recording modes:** Hold (push-to-talk), Toggle (tap to start/stop), and
  **Hands-free** (tap to start, press **Enter** to stop & transcribe).
- **Tier-1 formatting:** punctuation/capitalization, filler removal, spoken
  formatting commands ("new line", "comma", …), and number conversion.
- **Personalization:** custom-word dictionary, find/replace rules, and snippets.
- **Noise suppression** (GTCRN) and Silero VAD in the live capture pipeline.

### Privacy
- **Zero network during dictation.** Capture → VAD → STT → format → inject runs
  entirely on-device. A CI guard test (`no_network_in_hot_path`) fails the build
  if any network call ever enters the dictation path. The only network use is
  the one-time model download and an optional, off-able update check;
  cloud post-processing is off by default. See the
  [zero-network audit](docs/superpowers/audits/2026-06-27-m6-zero-network-audit.md).

### Identity & first-run
- New **MindFlow** visual identity — monochrome-gold + glassmorphism, line-art
  brain+waveform logo, gold recording overlay.
- Guided first-run onboarding (welcome → permission primers → model → try-it →
  features) and a polished settings experience with cross-tab search and
  reset-to-defaults.

### Platforms
- Windows (`.exe`/`.msi`), macOS (`.dmg`, Apple Silicon), Linux
  (`.AppImage`/`.deb`/`.rpm`). Built unsigned for this release — see the README
  for the install warnings.

### Credits
Built on [Handy](https://github.com/cjpais/Handy) by cjpais (MIT). Speech-to-text
via whisper.cpp and the ONNX models noted in the app's About screen.
