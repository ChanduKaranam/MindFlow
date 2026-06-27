# M6 — Zero-Network Audit

**Date:** 2026-06-27 · **Milestone:** M6 (v1 hardening) · **Scope:** `app/src-tauri/src`

## 1. Guarantee

> **Core dictation** — capture → VAD → STT → format → inject → output — makes **zero network calls.**

Verified two ways: this manual audit, and the always-on regression test `app/src-tauri/tests/no_network_in_hot_path.rs`, which fails the build if a network symbol enters any hot-path module.

The only code in the entire backend that opens the network is **two files**, both outside the dictation path and both opt-in:

```
$ grep -rnE "reqwest|hyper|ureq|std::net|TcpStream|TcpListener|UdpSocket" src --include=*.rs
src/llm_client.rs:3,100,102      reqwest  (post-processing LLM)
src/managers/model.rs:1082,1094,1111  reqwest  (model download)
```

No other file references any HTTP crate or `std::net`. There is no `hyper`, `ureq`, or raw socket use anywhere.

## 2. Network surface

| # | Site | File:line | Trigger | Default | Disposition |
|---|------|-----------|---------|---------|-------------|
| 1 | Model download | `managers/model.rs:1082` (`reqwest::Client::new()`) | User picks/downloads a model (first-run or Models settings) | n/a (explicit action) | **Allowed** — one-time, before offline use. Not in the dictation path. |
| 2 | App update check | tauri updater plugin, `tauri.conf.json:68` (`updater.endpoints`) | Optional; gated by `update_checks_enabled` | `true` (off-able) | **Allowed opt-in** — user can disable. Not a direct `reqwest` call in our code; handled by the tauri updater plugin. |
| 3 | Post-processing LLM | `llm_client.rs:100` (`create_client`), `:221` (`fetch_models`) | Only when `post_process_enabled` AND a provider is configured | `false` | **Opt-in cloud, off by default.** Sends transcript to the user-configured provider. Deferred to the **v2 Tier-2 LLM spec**; M6 verifies + flags only. |

## 3. Hot-path walk

Every module in the dictation path was scanned (see the guard test's `HOT_PATH`); none contains a network symbol:

| Module | Role | Network? |
|---|---|---|
| `audio_toolkit/` (audio, vad, denoise, resampling, text) | Capture, VAD, denoise, filler filtering | none |
| `managers/audio.rs` | Recording lifecycle | none |
| `managers/transcription.rs` | Local STT (whisper/transcribe-rs), custom-words prompt | none |
| `transcription_coordinator.rs` | Recording state machine (hold/toggle/hands-free) | none |
| `format/` (`spoken_commands.rs`) | Spoken-command formatting | none |
| `replace/` (`replacements.rs`) | Dictionary / replacements / snippets | none |
| `actions.rs` | Applies formatting + replacements, drives output | none |
| `clipboard.rs`, `input.rs` | Paste / key injection | none |
| `signal_handle.rs` | Shared transcription trigger (CLI/signals) | none |
| `shortcut/` | Global hotkey registration + dispatch | none |

The hot-path **never** imports `model.rs` or `llm_client.rs` for transcription output; STT runs fully in-process on CPU via ORT / whisper-rs.

## 4. Excluded files (legitimate opt-in network)

- `managers/model.rs` — model download. A model must exist locally before dictation; downloading it is a deliberate, one-time, pre-offline action.
- `commands/models.rs` — the command layer over model download/management.
- `llm_client.rs` — post-processing LLM client. Off by default; opt-in cloud.
- `lib.rs` — wires the tauri updater plugin (no direct socket code).

These are intentionally **not** in the guard test's `HOT_PATH`.

## 5. Post-processing — v1 disposition

Post-processing (cloud LLM rewriting/tone) is **off by default** (`default_post_process_enabled() -> false`) and is the single intentional cloud touchpoint. For v1 it is treated as an opt-in extra; the full design (Tier-2 formatting) is deferred to the **v2 Tier-2 LLM spec**. Core dictation never depends on it.

## 6. No telemetry · bundled fonts

- **No analytics/telemetry SDK.** The network grep above returns only the two opt-in sites — there is no analytics, crash-reporting, or tracking client anywhere in `src`.
- **Fonts are bundled** (M5 vendored Geist/Fraunces/Geist Mono as local `woff2` in `app/src/assets/fonts/`; `@font-face` `src` is local `url(...)`, no CDN/Google Fonts).

## Conclusion

With post-processing off (default) and update checks disabled or simply not invoked, MindFlow performs the full dictate-and-inject cycle with **zero outbound network traffic**. The regression guard (`no_network_in_hot_path.rs`) keeps this true as the code evolves.
