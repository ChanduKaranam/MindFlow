# Changelog

All notable changes to MindFlow are documented here.

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
