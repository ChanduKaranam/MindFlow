# MindFlow — Design Spec

*Created 2026-06-24. Status: approved design, ready for implementation planning.*

## 1. Summary

**MindFlow is a free, fully-local, CPU-friendly, cross-platform open-source clone of Wispr Flow.** It lets a user hold a hotkey, speak into any text field in any app, and have clean, punctuated text inserted — entirely on-device, with no cloud APIs and no GPU required.

The core insight from research: Wispr Flow is **100% cloud** (both speech recognition and AI formatting run on their servers). MindFlow is therefore not a port but a re-architecture around local CPU models. We build it by **fusing the best open-source dictation projects** so each fills another's gaps, assembled in one Rust+Tauri codebase.

### Goals
- Hold-to-talk dictation into any application, on Windows, macOS, and Linux.
- Fully offline after first-run model download; zero network traffic during use.
- Runs on any laptop CPU, no GPU. Speed *and* accuracy maximized via swappable model tiers.
- Free and open-source (GPL/AGPL copyleft).

### Non-goals (v1)
No cloud, no telemetry, no account/login, no GPU requirement, no LLM in the default dictation path, no team/sync features.

## 2. The Fusion Set (which projects, and why)

| Role | Project | License | Contributes | Liftability |
|---|---|---|---|---|
| 🫀 Host body | **Handy** (cjpais/Handy) | MIT | global hotkey, push-to-talk, text injection, VAD, `transcribe-rs` STT, tray, model download | *is the base* |
| 🧠 AI layer (v2) | **Whispering** (EpicenterHQ/epicenter) | AGPL | chainable formatting/transform pipeline over local LLMs | Lift directly (same `transcribe-rs`/Tauri stack) |
| 📚 Personalization | **VoiceInk** (Beingpax/VoiceInk) | GPL (Swift) | context "Power Mode" + custom dictionary UX | Reimplement in Rust |
| ⚙️ Model UX | **Hyprnote/OWhisper** (fastrepl/hyprnote) | MIT | "Ollama-for-STT" model management pattern | Adapt (same stack) |

**Why Handy is the host:** most popular (~24.6k stars), most active, MIT, explicitly "most forkable", and it already solves the hardest/riskiest parts (cross-platform hotkey + injection + VAD + CPU-first STT). Critically, **Handy and Whispering already share `transcribe-rs`**, so the most important donor (Whispering's AI layer) is code-compatible.

License posture: MindFlow ships **GPL/AGPL (copyleft)** so we may freely lift/reimplement from any OSS project. A `NOTICE` file credits all upstreams.

## 3. Architecture

### Pipeline
```
[hotkey held] → Capture (mic+VAD) → STT (transcribe-rs/Parakeet) → Format (Tier-1 rules+punctuation) → Inject (type/paste) → [text in app]
                                                                                          ↑ Tier-2 LLM only on explicit command (v2)
Tray UI + Settings: owns config, models, dictionary, snippets (always running)
```

### Modules (each maps 1:1 to a harvest source)
| Module | One job | Source |
|---|---|---|
| Capture | mic + Silero VAD | Handy |
| STT Engine | audio→text, CPU, swappable model registry | Handy (`transcribe-rs`) + new |
| Formatter | clean/format text, pluggable steps | new (Tier-1); Whispering (Tier-2, v2) |
| Injector | text into active app + clipboard fallback | Handy |
| Hotkey | global push-to-talk | Handy |
| Dictionary | word-replace + snippets | new (VoiceInk-inspired) |
| Models | downloader, verify, auto-pick by CPU | Handy + new |
| Core/UI | tray, settings, onboarding, config | Handy shell + new |

**Key seam:** STT outputs plain text; the Formatter never knows how it was produced. This lets v1 ship a rules-only/Tier-1 formatter and v2 drop in the LLM behind the same interface with zero ripple.

### STT model strategy (speed + accuracy, both maximized)
On CPU, speed vs accuracy trade off — resolved by making the model a swappable, downloadable, upgradable component with tiers and per-laptop auto-pick:

| Preset | Model | Profile |
|---|---|---|
| ⚡ Turbo | Streaming Zipformer 20M (Apache-2.0) or Moonshine Tiny (MIT) | smallest, true streaming, low-end laptops |
| ⭐ Balanced (default) | **Parakeet-TDT-0.6B** v2 (English) / v3 (25-lang auto-detect), ONNX int8 | best speed+accuracy combined; RTF ~0.03–0.09, WER 6.05 |
| 🎯 Max Accuracy | Parakeet-v2 (still best); whisper-large-v3-turbo for 99-lang | strong CPUs |

Key research finding: **Parakeet-TDT-0.6B beats every Whisper variant on CPU** (more accurate than large-v3 at ⅓ the size, and faster). True accuracy kings (Canary, Granite, Qwen3-ASR) are GPU-only → on the upgrade watch-list, not usable today.

Two runtimes behind one interface: ONNX Runtime (`ort`) for Parakeet/Moonshine, whisper.cpp for Whisper fallback. New/better models = config entries, not code changes. Auto-pick on first run by CPU capability (cores, AVX2).

### Formatting strategy (tiered — research-driven)
No local LLM hits a sub-2s feel on CPU, so the LLM stays out of the default path:
- **Tier 1 (always on, ~10–80ms):** regex/rules + small ONNX punctuation/truecasing model (`1-800-BAD-CODE/punctuation_fullstop_truecase_english`) + optional disfluency-BERT. Handles punctuation, capitalization, filler removal instantly; benchmarks beat GPT-4 on that narrow task.
- **Tier 2 (on-demand, v2):** local LLM (Llama 3.2 3B / Phi-4-mini MIT / Qwen3-4B Apache-2.0) only for explicit rewrite commands ("make concise", "bulletize"). 2–8s latency acceptable because occasional.

### STT runtime
`transcribe-rs` (whisper.cpp + ONNX Runtime), exactly as Handy ships it — inherited, battle-tested by both Handy and Whispering. Parakeet-on-ONNX is the CPU default. `sherpa-onnx` is optional, added only if v2 wants true live partials.

## 4. v1 MVP Scope

### In scope
1. Global push-to-talk hotkey (Handy)
2. Dictate into any app — text injection (Handy)
3. Clipboard-paste fallback (Handy)
4. Local CPU STT — Parakeet default, swappable (`transcribe-rs`)
5. Model tiers + auto-pick by CPU (new + Handy)
6. Model downloader with verify (Handy)
7. Tier-1 formatting — rules + ONNX punctuation + filler (new)
8. Custom dictionary — word/phrase replacement (reimplement from VoiceInk)
9. Snippets/macros — spoken cue → text (new)
10. Tray UI + Settings (Handy shell)
11. Onboarding — permissions, language, practice (Handy + new)
12. Cross-platform Win/macOS/Linux

### Deferred to v2
Local LLM rewriting/tone (Tier-2); Command Mode; context-awareness/Power Mode; true streaming live partials (sherpa-onnx); whisper-mode/backtracking/style-learning; team/sync (out of scope entirely).

### Definition of Done (v1)
On a clean Win/macOS/Linux laptop with no GPU, a user can:
1. Install, complete onboarding, and have an STT model auto-selected that runs ≥ real-time on their CPU.
2. Hold the hotkey in **any** app, speak, release → punctuated, capitalized, filler-free text appears within ~1–2s, fully offline.
3. Add a dictionary word and a snippet, and have both take effect.
4. All above with **zero network traffic** after model download (verifiable).

## 5. Project structure & integration sequence

### Repo layout (Tauri + Rust workspace)
```
mindflow/
├─ src-tauri/src/{capture,stt,format,inject,hotkey,dictionary,models,config}/ + pipeline.rs
├─ src/ (Tauri frontend: settings, onboarding, dictionary, snippets, IPC)
├─ models/ (gitignored, downloaded at runtime)
├─ docs/ (specs, research, ADRs)
└─ CONTEXT.md (domain glossary)
```
Module folders map 1:1 to harvest sources, so each integrates and tests independently. `Cargo.toml` pins `ort =2.0.0-rc.12` and feature-gates whisper.cpp per-OS.

### Milestones (riskiest-first)
| M | Milestone | Proves |
|---|---|---|
| M0 | Fork Handy, build & run clean on 3 OSes | baseline |
| M1 | **Injection + hotkey verified on all 3 OSes** | #1 risk retired first |
| M2 | STT pipeline: Parakeet default + tiers + auto-pick | core dictation offline on CPU |
| M3 | Tier-1 formatting (rules + ONNX punctuation + filler) | clean, instant output |
| M4 | Dictionary + snippets | personalization |
| M5 | Tray UI, settings, onboarding polish | shippable UX |
| M6 | v1 hardening — zero-network audit, 3-OS install test, DoD | release gate |

## 6. Error handling

The transcription is sacred — every path ends with the user's words reaching them.
| Failure | Handling |
|---|---|
| Injection fails | clipboard paste; else copy + toast "paste manually" |
| Mic permission denied / no device | detect at startup + hotkey; tray ⚠️; onboarding re-prompts |
| Model missing/corrupt | SHA-256 verify; re-download; manual-download link |
| CPU too weak for tier | auto-pick downgrades to Turbo; warn |
| Hotkey conflict | detect registration failure; prompt to re-bind |
| Silence captured | VAD reports no speech; subtle cue; no empty paste |

## 7. Testing

- **Unit (fast, deterministic):** format module, dictionary/snippets, model auto-pick logic.
- **Golden-transcript tests:** fixed audio clips → assert STT+format output stable.
- **Integration (per-OS):** automated where possible + manual smoke checklist for injection into real apps.
- **Zero-network test (first-class CI gate):** full flow with network blocked → assert success + zero outbound connections. Proves the core promise.
- **Cross-platform CI:** GitHub Actions matrix (Win/Mac/Linux) per-OS Tauri build; catches MSVC-linking + feature-gate issues.

## 8. Risks & mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| Cross-platform injection (Wayland/macOS-TCC/Windows-UIPI) | High | M1 retires first; clipboard fallback; inherit Handy code |
| Windows MSVC linking conflict | Certain | Parakeet-ONNX baseline on Windows; whisper.cpp feature-gated off |
| CPU latency on weak laptops | Medium | tiered models + auto-pick; Tier-1 instant; LLM out of hot path |
| `ort` RC instability | Medium | pin exact version; track `transcribe-rs` upstream |
| Upstream divergence | Medium | vendor code; document provenance |
| Scope creep into v2 | Medium | DoD checklist is the hard gate; v2 fenced off |

## 9. References

Research backing this design lives in `docs/research/`:
- `2026-06-23-wispr-flow-feature-inventory.md`
- `2026-06-23-oss-dictation-landscape.md`
- `2026-06-23-cpu-stt-models-deepdive.md`
- `2026-06-23-cpu-llm-formatting-deepdive.md`
- `2026-06-23-cpu-stt-runtime-deepdive.md`
