# Changelog

All notable changes to MindFlow are documented here.

## Unreleased — M9 "Feels alive"

### Zero system toll
- **The app no longer slows the machine at startup** — the cleanup LLM used to
  page 1–2.4 GB from disk the moment the app launched; it now loads while you
  speak your first dictation (starting ~1 s after the STT model so they don't
  fight for the disk). Idle app = no model RAM, no measurable CPU.
- Background polish runs on half the CPU cores (Command Mode keeps full
  speed); the hardware scan is cached instead of re-run per dictation.

### Live dictation feel
- **"Scratch that"** (also "delete that" / "undo that" / "never mind") spoken
  on its own deletes the previous dictation — verified select-back, one Delete
  keypress, never fires blind.
- **Cleanup intensity knob** — Off / Light / Medium / High presets over the
  advanced flags (High adds a clarity rewrite); shows "Custom" when the flags
  are hand-tuned.

### Trust
- **History diff view** — see exactly what the AI changed (word-level
  highlights) and copy the raw transcription with one click.
- **Usage insights** — dictations, total words, words per dictation, computed
  locally from History.

### Power
- **Transforms** — named rewrite prompts (Polish, Shorten, Bullet points, Fix
  grammar + your own): speak a transform's name in Command Mode to apply it to
  the selection.
- **Privacy-safe context** (all default OFF) — optionally let the cleanup LLM
  see the window title, selected text, and/or clipboard to resolve names and
  terms. Local-only by construction, per-source toggles, a badge shows what
  the last dictation used; selection capture is skipped in terminals. No
  screenshots, ever.
- **Quiet mode** — boosts mic normalization and lowers the voice threshold for
  whispered dictation.

### Deferred
- Streaming partial text (Moonshine/Parakeet) and mouse-button triggers moved
  to M10; auto-learn-from-typed-corrections cut after design review (unsafe
  via key simulation — needs OS accessibility read APIs).

## Unreleased — M8 Wispr-parity interaction layer

### Feel
- **Instant paste** (default on) — the deterministic text hits the cursor
  immediately; when the AI polish lands it replaces the pasted text in place.
  Replacement is self-verifying (select-back + compare): if you typed, moved
  the caret, or switched apps in between, the raw text is safely kept. Skipped
  on Wayland (left as raw + polish in History).
- **Warm engine** — the cleanup LLM preloads in the background at startup and
  on model change, so the first dictation no longer pays the model-load cost;
  it unloads again on the same idle policy as the STT model.

### Intelligence
- **Per-app tone** (default on) — dictation adapts to the focused app:
  professional in email clients, casual in chat apps, verbatim-technical in
  code editors and terminals, concise in note apps. Detection degrades
  gracefully (e.g. Wayland) to the neutral default.
- **Command Mode** — new shortcut (default `ctrl+alt+space`): select text
  anywhere, hold the shortcut, and speak an instruction ("make this shorter",
  "turn this into bullet points"); the local LLM transforms the selection and
  pastes the result over it. With nothing selected, the instruction's output
  is typed at the cursor. Never types anything on failure.

### Scratchpad
- **Scratchpad window** — a small always-on-top notepad (Footer → Scratchpad)
  to dictate into without a target app; copy the result with one click.

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

### Platform notes
- **macOS (aarch64):** `whisper-rs-sys` and `llama-cpp-sys-2` each vendor a
  full static `ggml`, which duplicate-symbol-clashes at link. GNU ld/MSVC use
  `--allow-multiple-definition` / `/FORCE:MULTIPLE`; `ld64` has no equivalent,
  so on macOS `llama-cpp-2` is built with its `dynamic-link` feature instead —
  llama.cpp and its ggml become dylibs bundled into `Contents/Frameworks`
  (see `app/src-tauri/build.rs` and `bundle.macOS.frameworks` in
  `tauri.conf.json`). Verified by the full `tauri build` macOS job in CI.

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
