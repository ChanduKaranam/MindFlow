# M6 — v1 Definition-of-Done Checklist

Maps each v1 Definition-of-Done item (master design `2026-06-24-mindflow-design.md` §4) to its verification. Automated items are gated by `cargo test`; manual items reference the 3-OS runbook (`m6-3os-install-runbook.md`).

> **DoD (verbatim):** On a clean Win/macOS/Linux laptop with no GPU, a user can (1) install, onboard, and have a model auto-selected that runs ≥ real-time; (2) hold the hotkey in any app, speak, release → punctuated, capitalized, filler-free text within ~1–2s, fully offline; (3) add a dictionary word and a snippet, both take effect; (4) all of the above with **zero network traffic** after model download.

## Item-by-item

| # | DoD item | How verified | Evidence |
|---|----------|--------------|----------|
| 1a | Install + complete onboarding on a clean machine | Manual | Runbook §"Install & onboard" (per OS) |
| 1b | A model is auto-selected for the CPU | Automated (logic) + manual | `recommended_tier_cmd` / `stt_tier.rs` tier pick; runbook confirms a model is recommended & downloads |
| 1c | Model runs ≥ real-time on CPU | Manual (timing) | Runbook §"Dictate (airplane mode)" — note the round-trip latency (~1–2s for a short utterance) |
| 2a | Hotkey records in **any** app, output injected | Manual | Runbook §"Dictate into a third-party app" |
| 2b | Output is punctuated, capitalized, filler-free | Automated (offline) + manual | `tests/zero_network.rs` (real STT on JFK clip; model-native punctuation/casing); filler filtering `audio_toolkit` `filter_transcription_output`; runbook visual check |
| 2c | Fully offline, ~1–2s | Automated (no-network) + manual | `tests/no_network_in_hot_path.rs` (no network in path) + runbook airplane-mode dictation |
| 3a | Add a dictionary word → takes effect | Automated (unit + offline gate) | `src/replace/replacements.rs` `#[cfg(test)]` (8 tests) + `tests/zero_network.rs` offline replacement assertion + `apply_custom_words` (transcription.rs) |
| 3b | Add a snippet / replacement → takes effect | Automated (unit) + manual | `apply_replacements` unit tests (8) + `apply_spoken_commands` unit tests (19); runbook snippet step |
| 4 | **Zero network traffic** after model download | Automated (guard) + audit + manual | `tests/no_network_in_hot_path.rs` (hot-path guard) + `2026-06-27-m6-zero-network-audit.md` + runbook airplane-mode dictation |

## Automated gate

All of the following must be green (run from `app/src-tauri`):

- [ ] `cargo test` — full suite, including:
  - [ ] `no_network_in_hot_path::dictation_hot_path_has_no_network_symbols`
  - [ ] `zero_network::transcribes_offline_on_cpu` (skips cleanly without `MINDFLOW_TEST_MODEL`)
  - [ ] `replace::replacements::tests::*` (8)
  - [ ] `format::spoken_commands::tests::*` (19)
- [ ] With a real model: `MINDFLOW_TEST_MODEL=<path> cargo test --test zero_network -- --nocapture` passes (transcription + offline replacement). (Maintainer, once.)

## Manual gate

- [ ] `m6-3os-install-runbook.md` fully signed off on all three OSes.

## Notes / open items

_(none — coverage confirmed: dictionary/replacement and spoken-command behaviors have existing unit tests; offline transcription + no-network are covered by the two integration tests.)_
