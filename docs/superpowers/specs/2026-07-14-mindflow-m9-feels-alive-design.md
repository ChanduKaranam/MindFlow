# M9 — "Feels alive": zero-toll performance, live dictation feel, trust & context

**Date:** 2026-07-14 · **Mode:** autonomous · **Branch:** mindflow-m7-accuracy-stack
**User mandate:** all gap-analysis items in one milestone + "the app must never make
the laptop feel slow."

## 0. Zero system toll (FIRST — regression fix + budget)

Root causes identified in code review of M8:
- `CleanupManager::preload()` fires at **app startup** (lib.rs), reading + paging a
  1.1–2.4 GB GGUF the moment the app launches → disk thrash + RAM pressure with
  zero user benefit until the first dictation.
- `resolve_model_id()` runs `detect_cpu_profile()` (a sysinfo full-scan) on every
  dictation and every preload.
- LLM generation uses `physical_cores - 1` threads at normal priority — during the
  background polish the whole machine stutters.

Fixes (design rules, enforced by tests where pure):
- **No model bytes touched at startup.** Preload moves to **recording start**
  (`TranscribeAction::start`), delayed ~1 s so the STT model load wins the first
  disk seconds; the load overlaps the user speaking, so the engine is warm by
  transcription end. App startup does zero model I/O.
- `cleanup()` must compute its timeout budget with `try_lock` (assume the load
  allowance on contention) — the preload holds the engine mutex for the whole
  load and must never stall a dictation's async task on `lock()`.
- Cache the CPU-profile/tier in a `OnceCell` (hardware doesn't change mid-run).
- Generation thread count: `generate()` gains an n_threads parameter
  (LlamaContextParams is per-call; no reload needed): `max(1, cores/2)` for
  background polish (no latency SLA once instant paste delivered), `cores - 1`
  for Command Mode (user waiting). CPU-profile caching lives in `stt_tier`
  itself (engine.rs calls it too).
- Idle unload stays on the existing `model_unload_timeout` policy.
- **Resource budget going forward:** with the default unload policy, idle app =
  0% measurable CPU and no model RAM mapped (users choosing `Never` opt out
  knowingly); any new feature violating this needs an explicit default-off setting.

## 1. "Scratch that" — delete last utterance (open lane)
- Deterministic, pre-LLM: if a dictation's rules-only text normalizes (lowercase,
  punctuation-stripped) to a cue phrase ("scratch that", "delete that",
  "undo that", "never mind"), do not paste; instead select-back-VERIFY the
  previous dictation's pasted text and send ONE Delete keypress (an empty-
  clipboard paste is a no-op in most apps — never "paste nothing").
- `LastPaste` state (text incl. trailing space + timestamp): written/updated
  ONLY inside the main-thread closures that perform paste and replace-in-place
  (serializes against the in-flight polish — the polish updates LastPaste to the
  polished text, so scratch-that always targets what is really on screen).
  Not written when `auto_submit` is on (Enter already fired; deletion is
  meaningless). Expires after 120 s. On Wayland (no key simulation) scratch-that
  emits a "not supported here" toast instead of silently swallowing the utterance.
- Mid-utterance "…some text scratch that" keeps LLM behavior (already a cleanup
  cue) — this feature is only the standalone spoken command.

## 2. View Diff + undo AI edits (trust)
- History already stores raw (`transcription_text`) and final. Add:
  (a) per-entry **diff view** in History (word-level, insert/delete highlighting,
  frontend-only); (b) **"Use raw"** button → re-copies raw text to clipboard;
  (c) toast after replace-in-place with an **Undo** action for ~8 s → puts the
  rules-only text back via the same select-back-verify swap.

## 3. Cleanup intensity knob (parity, simplification)
- One dropdown — Off / Light / Medium (default) / High — mapped onto existing
  flags + a new prompt hint:
  Off = ai_cleanup_enabled false; Light = smart only; Medium = smart +
  self_correction + preserve_technical (today's default); High = Medium + a
  "rewrite for clarity: tighten rambling phrasing, split run-ons" prompt rule.
- High's aggressive shrinkage must survive `is_sane_output`: the deletion-only
  floor (0.15 word ratio) is verified by test against a High-style rewrite; if
  it can't pass, High relaxes the floor explicitly rather than silently falling
  back on every long dictation.
- The three advanced toggles stay (Advanced accordion) — the knob is a preset
  writer over them; when the flags match no preset the knob displays "Custom".

## 4. Transforms (presets + custom prompts on selection)
- Extends Command Mode: settings list of named transforms (name, prompt).
  Ship 4 presets: Polish, Shorten, Bullet points, Fix grammar. User can add
  custom ones. UI: transforms picker in settings; spoken Command Mode can also
  say a transform name ("polish") to run it.
- No auto-run-after-dictation in M9 (needs more UX thought — deferred).

## 5. Auto-learn v2 — CUT after design review
- The delayed re-read cannot work: if the user edited the span (the only case
  worth learning from), the caret/char-count no longer match and the select-back
  fires wrong-span selections + Ctrl+C into an app the user is actively typing
  in. History-pane learning (M7) remains the learning path; make it more
  discoverable instead (History button in tray + a hint toast after a dictation
  is edited in history). Revisit only with an OS-level read API (AX/UIA text
  read, no key simulation) in M10's context work.

## 6. Privacy-safe context (counter-positioning headline)
- New settings group "Context" with **independent, default-OFF** toggles:
  window title, selected text (captured at recording start via existing
  `capture_selection`), clipboard text. Selected-text capture is SKIPPED when
  the focused app is a terminal (Ctrl+C = SIGINT would kill the user's process;
  `context::detect_active_app_category` already identifies terminals/code) and
  the toggle's help text says so.
- Enabled context is injected into the cleanup prompt as tagged data
  (`Context (do not transcribe, use only to resolve names/terms): …`, truncated
  to 400 chars per source).
- **Audit log:** every dictation's history entry records which context sources
  were used (not the content); the History pane shows small badges. No
  screenshots, no OCR, nothing leaves the machine (hot-path guard already
  enforces no network).

## 7. Streaming partials (open lane, riskiest — LAST)
- Overlay shows live partial words while recording; final text still goes
  through the normal pipeline on release. Partials render ONLY in our overlay,
  never typed into the target app.
- Engine: transcribe-rs `MoonshineStreaming` already exists in the codebase's
  engine enum; Parakeet TDT chunked streaming is not exposed by transcribe-rs.
  M9 ships overlay streaming **for Moonshine-streaming models only**. The
  periodic-retranscription fallback for other engines is CUT (O(n²) CPU during
  recording violates rule 0 by construction). Parakeet streaming = M10, via
  parakeet-rs or a transcribe-rs contribution.

## 8. Smaller items
- **Whisper-quiet preset:** `quiet_mode` toggle → input gain boost (existing
  gain stage) + lower VAD threshold preset (0.25).
- **Usage insights:** per-dictation word count + duration already derivable
  from history; add a small Stats card (total words, WPM estimate, streak) —
  frontend aggregation over history DB, no new collection.
- **Mouse triggers / scratchpad tabs:** explicitly deferred to M10 (mouse
  needs rdev/handy-keys work; scratchpad tabs are cosmetic) — documented.

## Order of implementation (value ÷ risk)
0 (perf) → 1 (scratch that, builds the shared verified-select-back helper)
→ 2 (diff/undo, reuses it) → 3 (intensity) → 4 (transforms) → 6 (context)
→ 8 (quiet + insights) → 7 (streaming, Moonshine-only). Item 5 cut (see above).
Realistic cut line for one session is after 8; 7 ships if budget remains.
Each lands with tests + lint before the next starts; anything unfinished at
session end is reported honestly, never half-wired.

## Quality bars (unchanged from M8)
CPU-only; zero-network hot path; i18next; settings recipe; pure-logic unit
tests; cross-platform with graceful degradation (Wayland/macOS notes per item).
