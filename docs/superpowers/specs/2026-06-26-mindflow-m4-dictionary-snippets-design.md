# MindFlow M4 — Dictionary + Snippets — Design Spec

*Created 2026-06-26. Status: approved design, ready for implementation planning.*

## 1. Summary

M4 is the **personalization** milestone (master design spec
`docs/superpowers/specs/2026-06-24-mindflow-design.md:123`): a **custom dictionary**
(word/phrase replacement, reimplemented from VoiceInk) plus **snippets/macros** (spoken
cue → text). It directly addresses the recurring real-world pain: proper-noun recognition
errors (e.g. "Purna Chandra Rao" → "Poor Nachem Darao") that no amount of formatting can fix
because they are a *vocabulary* problem.

The design unifies dictionary-fixes and snippets behind one deterministic replacement
engine, exposed as **three complementary tools**:

| Tool | Matching | Purpose | Status |
|---|---|---|---|
| **Vocabulary** | **Fuzzy** (Levenshtein + Soundex + n-gram) | Nudge *unpredictable* mishearings toward a known-correct spelling | Exists in Handy; M4 unblocks multi-word phrases |
| **Replacements** | **Exact** (case-insensitive, boundary-aware) | "I keep getting exactly *X*, always write *Y*" — stubborn names, abbreviations | Net-new engine |
| **Snippets** | **Exact** | Spoken cue → boilerplate ("my email" → an address) | Net-new (Phase 2; reuses the engine) |

Stays inside every MindFlow constraint: deterministic, CPU-only, **no new dependency, no
network, no LLM**.

## 2. Goals / Non-goals

### Goals
- Let the user fix proper nouns / jargon reliably (fuzzy *and* exact).
- Let the user define spoken-cue text expansions (snippets).
- One pure, testable replacement engine serving both Replacements and Snippets.
- Deterministic, idempotent, CPU-only, zero new deps.

### Non-goals (deferred)
- **Context-awareness / "Power Mode"** (app-specific behavior) — VoiceInk's other
  personalization feature is **deferred to v2** per the master spec
  (`2026-06-24-mindflow-design.md:94`). M4 is dictionary + snippets only.
- **LLM rewrite / tone / Command Mode** — Tier-2, v2.
- **Regex / scripting in replacements** — plain phrase→text only.
- **Per-entry fuzzy thresholds, categories, import/export** — YAGNI for M4; the existing
  global `word_correction_threshold` stays.

## 3. The three tools (how they relate)

1. **Vocabulary (existing, fuzzy).** Handy's `apply_custom_words`
   (`app/src-tauri/src/audio_toolkit/text.rs`) already does Levenshtein + Soundex + n-gram
   matching against `settings.custom_words` (a flat list of correct spellings), gated by
   `word_correction_threshold` (default 0.18). The backend already supports multi-word
   phrases (n-grams up to 3 words); **only the frontend blocks spaces.** M4 unblocks that so
   "Purna Chandra Rao" can be entered. The Soundex ×0.3 boost makes a fuzzy match for that
   name plausible once entry is possible. Kept as-is otherwise — strictly better than
   discarding a working fuzzy layer.

2. **Replacements (new, exact).** For mishearings the fuzzy layer misses, or deterministic
   substitutions, the user defines `from → to` pairs applied as an exact, case-insensitive,
   boundary-aware pass. Guarantees the fix.

3. **Snippets (new, exact — Phase 2).** Same engine, a second list, framed for expansion
   (cue → longer text). Separate UI list keeps the mental models distinct ("fix a word" vs
   "insert boilerplate").

## 4. Architecture

### The engine
A pure function in a new `replace` module
(`app/src-tauri/src/replace/mod.rs` + `replace/replacements.rs`), mirroring the M3
`format/spoken_commands.rs` pattern (pure, no I/O, no app handle, inline-unit-tested):

```rust
pub struct Replacement { pub from: String, pub to: String }

pub fn apply_replacements(text: &str, rules: &[Replacement]) -> String
```

### Matching rules (exact, deterministic)
- **Case-insensitive** match of `from` against the transcript.
- **Boundary-aware:** a match only fires when it is not embedded inside a larger
  alphanumeric run — i.e. the characters immediately before and after the match are
  non-alphanumeric or string ends. So `from = "cat"` does **not** match inside "category".
- **Whitespace-flexible** for multi-word `from`: internal runs of whitespace in the
  transcript match a single space in `from` (so "my   email" still matches "my email").
- **Longest `from` first:** rules are applied in order of descending `from` length, so a
  multi-word phrase wins over a shorter one it contains.
- **All occurrences** replaced.
- **`to` inserted verbatim** — the user controls the casing/content of the replacement.
- **Empty / whitespace-only `from` is ignored** (no-op rule).
- **No regex, no new deps** — plain std-library scanning over chars.

### Why pure
Holds the whole transform in one testable place; no coupling to audio/Tauri; idempotent and
position-independent enough to sit anywhere in the post-STT chain.

## 5. Phase 1 — Dictionary

1. **Unblock multi-word phrases** in the Custom Words UI
   (`app/src/components/settings/CustomWords.tsx`): relax the no-spaces sanitization to
   allow internal spaces (keep the length cap and the dangerous-char strip `<>"'&`). No
   backend change — `apply_custom_words` already handles phrases.
2. **Replacements engine + setting + UI.**
   - Setting `replacements: Vec<Replacement>` (`#[serde(default)]`, empty).
   - Backend command `update_replacements(replacements)` (mirrors `update_custom_words`).
   - `apply_replacements` called in the pipeline (§7).
   - UI: a two-column **Replacements** editor (rows of `from → to`, add/remove) in a new
     Dictionary settings area.

## 6. Phase 2 — Snippets

Reuse the engine with a second list:
- Setting `snippets: Vec<Replacement>` (`#[serde(default)]`, empty).
- Backend command `update_snippets(snippets)`.
- Pipeline: `apply_replacements(text, &settings.snippets)` runs right after the
  Replacements pass.
- UI: a **Snippets** editor (same `from → to` shape, labeled for expansion; allows longer
  `to` values).

Phase 2 is deliberately thin because the engine already exists.

## 7. Data flow (pipeline order)

Current post-STT chain (verified):

```
STT → fuzzy custom-words (existing) → filler filter → [actions.rs:]
      chinese variant → M3 apply_spoken_commands → (optional LLM) → inject
```

M4 inserts the replacement passes in `actions.rs::process_transcription_output`,
**after `apply_spoken_commands` (M3), before the optional LLM post-process**:

```
… → M3 apply_spoken_commands
    → apply_replacements(text, &settings.replacements)   // Phase 1
    → apply_replacements(text, &settings.snippets)       // Phase 2
    → (optional LLM) → inject
```

Rationale: exact `from` matching is case-insensitive so M3's capitalization does not break
it; running before the optional LLM means the LLM (if ever enabled) sees the user's intended
text. The fuzzy Vocabulary pass stays where it is (earlier, in `transcription.rs`).

## 8. Settings / UI

A new **"Dictionary"** area in settings (its own section/group):
- **Custom Words** (existing component, now phrase-capable) — the fuzzy Vocabulary.
- **Replacements** (new) — two-column `from → to` rows, add/remove, with validation.
- **Snippets** (new, Phase 2) — same shape, expansion-framed.

All user-facing strings via i18next (English populated; other locales fall back to English).
The Replacements/Snippets editors follow the existing settings-component idioms
(`useSettings` get/update, `ToggleSwitch`/list patterns).

## 9. Error handling

| Case | Handling |
|---|---|
| Empty / whitespace `from` | Rule ignored (engine skips it) |
| Duplicate `from` | Last rule wins (stable order); UI may warn |
| `to` empty | Allowed (acts as a deletion of `from`) |
| Dangerous chars in input | Sanitized in the UI like Custom Words (`<>"'&` stripped) |
| Old settings file lacking keys | serde defaults to empty lists — loads unchanged |
| No rules configured | Pass is a no-op (returns input) |

Zero network in any path.

## 10. Testing

Pure, deterministic unit tests on `apply_replacements`:
- Case-insensitive match; verbatim `to` casing.
- **Boundary correctness:** `from="cat"` does not alter "category"; matches standalone "cat".
- **Longest-first precedence:** overlapping rules — the longer `from` wins.
- Multi-word `from` with collapsed/extra whitespace in the transcript.
- All-occurrences replacement.
- Idempotency: running twice == once (when `to` does not re-introduce a `from`).
- Empty/whitespace `from` ignored; empty `to` deletes.
- Integration sanity: settings round-trip via the new commands (backend), and a multi-word
  custom word now survives the UI sanitizer (frontend).

Satisfies the master spec's "Unit … dictionary/snippets" testing line
(`2026-06-24-mindflow-design.md:141`) and the v1 Definition-of-Done item "Add a dictionary
word and a snippet, and have both take effect" (`:100`).

## 11. Global constraints

- CPU-only, fully local, **zero network, no LLM**, **no new Rust/JS dependencies** (pure
  std string logic; no `regex` crate).
- Idempotent; deterministic.
- English-only UI strings populated (i18next; others fall back).
- Settings backward-compat: `#[serde(default)]`; old files load unchanged; no
  `deny_unknown_fields`.
- Bindings (`app/src/bindings.ts`) hand-edited to match generated style (headless regen
  unavailable).
- Conventional commits; `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` trailer.

## 12. References

- Master design spec: `docs/superpowers/specs/2026-06-24-mindflow-design.md`
  (M4 row :123; modules :50; v1 scope :87–88; deferred Power Mode :94; DoD :100).
- Existing fuzzy matcher: `app/src-tauri/src/audio_toolkit/text.rs` (`apply_custom_words`);
  UI `app/src/components/settings/CustomWords.tsx`; setting `custom_words` +
  `word_correction_threshold` in `settings.rs`.
- Pipeline seam: `app/src-tauri/src/actions.rs` (`process_transcription_output`), after the
  M3 `crate::format::apply_spoken_commands` call.
- Engine precedent: M3 `app/src-tauri/src/format/spoken_commands.rs` (pure-function +
  inline-tests pattern).
