# M8 — Wispr-parity interaction layer (instant paste, per-app tone, Command Mode, language UX + scratchpad, LLM polish)

**Date:** 2026-07-14 · **Mode:** autonomous (user waived approval, 3-hour budget)
**Goal:** close the remaining interaction gaps with Wispr Flow, cross-platform (Windows/macOS/Linux), while hardening the M7 LLM cleanup integration.

## Scope (value-ordered — earlier items must ship even if later ones slip)

### 1. LLM integration polish (bug-fix tier)
- **Warm start:** preload the cleanup engine in the background at app startup and
  whenever the cleanup model changes, so the first dictation never pays the
  load cost. Reuse the existing `(model_id, engine)` slot; loading must never
  block dictation (skip-if-busy, silent fallback preserved).
- **Unload timer parity:** cleanup engine honors the same
  `model_unload_timeout` idea as STT (drop engine after N minutes idle) so RAM
  is returned on laptops.
- **Status events:** emit `cleanup-state-changed` (loading/polishing/idle) so
  the UI can show activity instead of appearing hung.
- **Onboarding:** the cleanup-model step exists (M7 T10); verify wiring end-to-end
  and fix anything broken found while testing.

### 2. Instant paste + background polish (Wispr-feel latency)
- Paste the rules-only text immediately after STT (existing paste path).
- Run LLM cleanup in the background; when it lands and differs, **replace the
  pasted text in place**: simulate N backspaces (N = chars of raw text as
  typed) then type the polished text — works on all three OSes through the
  existing typing infrastructure.
- **Abort guards** (any → leave raw text): user pressed any key/mouse since
  paste (rdev listener already global), focused app changed (active-window
  check), cleanup failed/fell back, or replacement disabled in settings.
- Setting `instant_paste` (default **on**). History stores raw + polished as
  today (`post_processed_text`).

### 3. Context awareness / per-app tone (Wispr signature)
- Detect the focused application at dictation end (crate per research;
  Wayland gracefully degrades to "unknown" → default tone).
- Map process names to categories: `email`, `chat`, `code`, `notes`,
  `browser`, `default` via a built-in table + user-overridable per-category
  tone toggle in settings (`app_tone_enabled`, default on).
- Category feeds one extra prompt section (tone: formal for email, casual for
  chat, technical-verbatim for code/terminal). No screen-content reading (out
  of scope: privacy).

### 4. Command Mode
- New binding `command_mode` (default `ctrl+shift+c`, configurable like other
  bindings): capture current selection (simulate copy + clipboard read with
  save/restore), record the spoken instruction, run the LLM with an
  instruction-application prompt (selection = data), paste the result over the
  selection. No selection → treat the dictation itself as the instruction and
  type the generated result.
- Uses the same engine slot/timeouts as cleanup; refuses (types nothing,
  notifies) on fallback instead of typing a raw echo.

### 5. Language auto-detect UX + scratchpad (polish tier)
- Language UX: history entries store and show the language the engine detected
  (Whisper reports it; Parakeet is en-only); settings language dropdown gets a
  "detected: X" hint after each dictation.
- Scratchpad: a small always-on-top window (existing multi-window setup à la
  overlay) with a textarea; dictations while it is focused land there; copy
  button; content persists per session only.

## Out of scope
Screen-content context, voice cloning, custom per-app prompt editor, Wayland
active-window support beyond graceful degradation, mobile.

## Quality bars
- CPU-only, zero-network hot path (existing CI guard must stay green).
- All strings i18next (en at minimum), settings via the established recipe.
- Every behavior change unit-tested where pure; cross-platform code behind
  `#[cfg]` with a single shared trait/facade and a stub for unsupported OSes.
- Full suite + eval harness (recording) re-run before push.
