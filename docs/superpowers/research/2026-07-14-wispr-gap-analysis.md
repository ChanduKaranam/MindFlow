# MindFlow vs Wispr Flow — gap analysis & M9+ roadmap (2026-07-14)

Sources: full Wispr Flow feature inventory (docs.wisprflow.ai, whats-new through Jul 2026)
and competitor sweep (superwhisper, VoiceInk, Aqua Voice, Talon, Handy, HN/Reddit
wishlists). Cross-referenced against MindFlow @ commit 3acb976 (M7+M8).

## Where MindFlow already wins

| Axis | MindFlow | Wispr Flow |
|---|---|---|
| Privacy | 100% local, zero network in hot path (CI-enforced) | Cloud-only; 2025 screenshot/keylog scandal |
| Price | Free, open source | $12–15/mo; free tier 2k words/wk |
| Linux | Full support | None (waitlist) |
| Offline | Always works | Unusable without internet |
| STT choice | Whisper/Parakeet/Moonshine/SenseVoice/Canary/GigaAM, one-click | Fixed cloud pipeline |
| Indian-name accuracy | Phonetic dictionary layer (Double Metaphone, tuned M7) | Generic dictionary boost |

Feature parity already held: push-to-talk + hands-free, AI cleanup with
self-corrections, per-app tone, Command Mode, snippets + replacements, spoken
commands + number conversion, scratchpad, dictionary auto-learn (from history
edits), instant paste (ours replaces-in-place; Wispr pastes once at end),
dictation recovery (WAV + retry), history.

## Gaps vs Wispr Flow (ordered by user-visible impact)

1. **No streaming feedback** — Wispr pastes one block too, but Aqua streams
   partials (~850ms) and it dominates their reviews. Nobody local does it.
2. **Auto-learn only from History edits** — Wispr learns when you correct a
   spelling anywhere. We require the user to edit in the History pane.
3. **No View Diff / per-dictation AI-edit undo** — Wispr shows raw vs cleaned
   and Cmd+Z reverts AI edits. We keep both in history but no diff UX.
4. **No screen/selected-text context** — Wispr reads surrounding text to
   continue sentences and resolve names (their scandal was HOW they did it).
5. **No Transforms/custom prompt presets** — Wispr: highlight → preset or
   custom rewrite prompts, optionally auto-run. Our Command Mode is free-form only.
6. **No cleanup intensity levels** — Wispr: None/Light/Medium/High. We have
   three boolean flags (close, but a single intensity knob reads simpler).
7. **No quiet/whisper robustness work** — Wispr markets whispering heavily.
   We have a denoiser (unvalidated on hardware) + fixed VAD threshold.
8. **No "scratch that" live delete-last-utterance** — actually a Wispr gap too
   at the interaction level (theirs is cleanup-only); Talon users swear by it.
9. **No usage insights** (WPM, streaks, per-app stats) — retention candy.
10. **No mouse-button triggers**, no language picker UX, minimal scratchpad
    (no tabs/history), no mobile (out of scope), no team features (out of scope).

## Recommended roadmap

### M9 — "Feels alive" (feel + trust)
1. **Streaming partials in the overlay** (Parakeet TDT supports chunked
   streaming on CPU — parakeet-rs proves it; Moonshine streaming as alt).
   Partials render in OUR overlay, never typed into the app; finalized text
   pastes on release. Open lane: no local competitor does this.
2. **"Scratch that" + Talon-lite formatters** — deterministic pre-LLM:
   delete-last-utterance (we already track last pasted span for
   replace-in-place), "snake case X", "all caps X". Nobody mainstream has it.
3. **View Diff + AI-edit undo** — history has raw+cleaned already; add diff
   view + one-click revert-to-raw re-paste. Counterpart of instant paste.
4. **Auto-learn v2** — after replace-in-place verification we can read back
   the field; detect user typed-over corrections → dictionary suggestions.
5. **Quiet-speech preset** — AGC gain + VAD threshold "quiet mode" toggle;
   validate GTCRN denoiser on real hardware at the same time.
6. **KV-prefix cache** in the LLM engine (llama-cpp-2 0.1.139
   clear_kv_cache_seq pattern) — cuts per-chunk prompt eval, biggest
   remaining latency lever.

### M10 — "Knows your world" (context, safely)
7. **Accessibility-based context** (window title / selected text / clipboard —
   never screenshots), per-level toggles, off by default, visible audit log of
   exactly what was injected. Turns Wispr's scandal into our headline.
8. **Transforms**: preset + user-written rewrite prompts on selection;
   optional auto-run after dictation; shareable "mode packs" (superwhisper
   community model).
9. **Per-app profiles v2**: beyond tone — per-app model, prompt, language pin.
10. **File-based sync** for dictionary/settings (JSON in user folder →
    Syncthing/Dropbox; no accounts).

### M11 — expansion
11. Insights dashboard (WPM, streaks, per-app words).
12. Meeting/system-audio transcription with diarization (parakeet-rs has it) —
    the natural second surface, big scope.
13. Cleanup intensity single-knob (None/Light/Medium/High) folding the three
    flags into presets; mouse-button triggers; scratchpad tabs.

## Positioning line
"Everything Wispr Flow does, on YOUR machine: no cloud, no subscription, no
screenshots — and it runs on Linux."
