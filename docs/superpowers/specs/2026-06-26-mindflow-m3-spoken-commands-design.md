# MindFlow M3 — Spoken Formatting Commands — Design Spec

*Created 2026-06-26. Status: approved design, ready for implementation planning.*

## 1. Summary

M3 was originally specced (`docs/superpowers/specs/2026-06-24-mindflow-design.md:122`) as
**Tier-1 formatting** = regex/rules + a small ONNX punctuation/truecasing model
(`1-800-BAD-CODE/punctuation_fullstop_truecase_english`) + optional disfluency removal.

**Research during brainstorming retired most of that scope:**

- **Filler removal already exists.** `filter_transcription_output()`
  (`app/src-tauri/src/audio_toolkit/text.rs:288`) already does language-aware filler
  stripping + stutter collapse, wired into `TranscriptionManager.transcribe()`.
- **Punctuation + capitalization is already produced by the STT model.** An empirical
  sample dictated on the target Windows laptop came back fully punctuated, capitalized,
  and sentence-segmented:

  > "Let me see how this is going to work out. So I am Chandu and one, two, three.
  > One, two, three. This is a numbers test. One, two, three. I am Poor Nachem Darao."

  The only defect in that sample was a **proper-noun recognition error** ("Purna Chandra
  Rao" → "Poor Nachem Darao"), which is a dictionary/vocabulary problem (M4), not a
  formatting problem. Bolting the ONNX punctuation model onto already-punctuated text
  would be redundant and risk fighting the STT's own punctuation.

**M3 is therefore re-scoped to the one genuinely-missing piece the STT cannot do:
built-in spoken formatting commands** — converting spoken cues that the STT transcribes
as *literal words* ("new line", "new paragraph") into *actual formatting* (line breaks,
punctuation, capitalization, digits).

This stays inside every MindFlow constraint: deterministic, CPU-only, **no new model, no
new dependency, no network**, no LLM in the path.

## 2. Goals / Non-goals

### Goals
- A built-in, English command set that turns spoken cues into formatting:
  newlines/paragraphs, literal punctuation, capitalization, and (opt-in) number→digit.
- Deterministic and pure: a string→string transform, unit-testable without audio or models.
- Robust to the fact that the STT **already punctuates and capitalizes** (idempotent;
  never double-punctuates).
- Per-category matching tuned to its false-positive risk (see §5).

### Non-goals (deferred)
- **User-editable commands** → M4 snippets ("spoken cue → text" is literally user-defined
  spoken commands; M3 ships the built-ins, M4 adds the editable ones — no editing UI built
  twice).
- **Non-English command tables** — English-only for M3. The table is structured so a
  language key can be added later.
- **Fixing recognition errors** (names, jargon) → M4 dictionary (`apply_custom_words`
  already exists).
- **ONNX punctuation/truecasing model** — dropped as redundant with the STT.
- **LLM rewriting / tone / "make concise"** — Tier-2, v2 (unchanged).

## 3. Architecture

### The unit
A single pure function:

```rust
pub fn apply_spoken_commands(text: &str, config: &SpokenCommandsConfig) -> String
```

placed in a focused new module `app/src-tauri/src/format/spoken_commands.rs`
(a `format` module matching the master design spec's "Formatter" seam,
`docs/superpowers/specs/2026-06-24-mindflow-design.md:47`).

`SpokenCommandsConfig` is a small plain struct derived from settings
(`{ enabled: bool, number_conversion: bool }`) — the function takes config by reference and
has **no side effects, no I/O, no app handle**. This mirrors the pure `decide()` pattern
from the hands-free work, which made that logic trivially unit-testable.

### Pipeline placement
The STT→inject flow today is:

```
TranscriptionManager.transcribe()            // app/src-tauri/src/managers/transcription.rs
  → apply_custom_words() (if custom_words)    // fuzzy vocab correction
  → filter_transcription_output()             // filler + stutter removal
process_transcription_output()               // app/src-tauri/src/actions.rs
  → maybe_convert_chinese_variant()
  → (optional) cloud LLM post-process         // off by default; not M3's concern
clipboard.rs paste                            // optional trailing space, then inject
```

`apply_spoken_commands()` runs on the **finalized STT string, after filler-filtering /
custom-words, before injection**. The exact call site (inside the transcription manager
after filtering, vs. at the top of `process_transcription_output`) is left to the plan; the
requirement is: once, on the near-final text, before the trailing-space/paste step, and
before any optional LLM pass so the LLM (if ever enabled) sees already-formatted text.

### Why a pure function here
- Holds the entire transform in one testable place.
- No coupling to audio, models, or Tauri — a clear interface (`&str + config → String`).
- Idempotent and order-independent enough to reposition in the pipeline without ripple.

## 4. Command tables (English, built-in)

Indicative trigger → output. Final phrasings/coverage are pinned during implementation
**after the empirical STT probe** (§6), because the STT may already emit some of these.

| Category | Trigger phrases (examples) | Output |
|---|---|---|
| **Newlines** | "new line"; "new paragraph" / "next paragraph" | `\n`; `\n\n` |
| **Punctuation** | "period" / "full stop"; "comma"; "question mark"; "exclamation mark" / "exclamation point"; "colon"; "semicolon"; "open paren" / "open parenthesis"; "close paren" / "close parenthesis"; "dash" / "hyphen" | `.` `,` `?` `!` `:` `;` `(` `)` `-` |
| **Capitalization** | "all caps WORD" (next word); "caps on" / "caps off" (region) | uppercased text |
| **Numbers** (opt-in) | "twenty five" → 25; digit runs "one two three" → 123 | digits |

Tables live as static data keyed for future i18n; only the English table is populated in M3.

## 5. Matching strategy (hybrid per category)

Because there is no LLM to judge intent, matching aggressiveness is tuned per category to
its false-positive risk. All matching is **case-insensitive** and tolerant of STT-added
capitalization and trailing punctuation around a trigger (a command can arrive as
`"New paragraph."`).

- **Newlines — conservative.** Fire only when the trigger stands alone as its own
  utterance/segment or is clearly delimited (e.g., bounded by sentence boundaries / its own
  short fragment). Rationale: "...opens a new paragraph of the contract..." must remain
  literal words. Missing a mid-sentence newline is acceptable; corrupting prose is not.
- **Punctuation — aggressive.** Replace the trigger anywhere, with **smart spacing**: no
  space before `.,?!:;)`, no space after `(`, collapse the doubled spaces a replacement can
  create. Rationale: people rarely dictate the literal word "comma"; and if the STT already
  rendered the symbol, the pass is idempotent (no double punctuation).
- **Capitalization — stateful, modest.** "all caps WORD" uppercases the following word;
  "caps on"/"caps off" uppercases the enclosed region. Scoped narrowly; no attempt at
  smart title-casing.
- **Numbers — opt-in, conservative bounded parser.** Only convert multi-word number runs or
  standalone number sequences; leave isolated number-words in prose alone ("one idea" stays
  "one idea"). Best-effort by design; correctness over coverage. Gated behind its own toggle
  (default off).

## 6. The idempotency / STT-overlap requirement (mandatory first step)

The STT already punctuates and capitalizes, and **may already convert some spoken
punctuation** (it might write "." when the user says "period"). Therefore the **first
implementation task is an empirical probe**, not coding:

1. On the target Windows laptop, dictate each candidate command phrase ("new line",
   "new paragraph", "period", "comma", "question mark", "open paren", "all caps test",
   "one two three", …) and record the exact string the STT emits.
2. Build the command tables to **fill gaps only** and be **idempotent**: if the STT already
   produced the symbol/format, the pass must be a no-op, never a doubling.
3. Confirm the capitalization/trailing-punctuation shapes the matcher must tolerate
   (e.g., `"New line."`).

The probe results are recorded in the implementation plan and drive the final table.

## 7. Settings

Two toggles, following the `RecordingMode` end-to-end plumbing pattern established in the
hands-free work (struct field + `#[serde(default = ...)]` + dedicated change command +
regenerated/hand-edited bindings + a settings UI control + i18next strings):

| Setting | Type | Default | Meaning |
|---|---|---|---|
| `spoken_commands_enabled` | `bool` | `true` | Master on/off for the whole pass |
| `number_conversion_enabled` | `bool` | `false` | Opt-in for the risky number→digit category |

When `spoken_commands_enabled` is false, `apply_spoken_commands` returns the input
unchanged. When true but `number_conversion_enabled` is false, all categories except numbers
run. Old settings files lacking these keys default as above (serde default; no
`deny_unknown_fields`), preserving backward compatibility.

All user-facing strings go through i18next (English populated; other locales fall back to
English per the repo's CONTRIBUTING_TRANSLATIONS workflow).

## 8. Testing

Pure, deterministic, no audio/model required — the whole point of the `&str → String`
design:

- **Per-category golden cases** — each trigger maps to its output.
- **Ambiguity / false-positive cases** — "a new line of work", "a new paragraph of the
  contract" survive unchanged under conservative newline matching.
- **Smart spacing** — punctuation replacements produce no space before `.,?!:;)` and no
  stray double spaces.
- **Idempotency** — feeding already-punctuated / already-formatted STT output (e.g.
  `"New paragraph."`) does not double anything; running the function twice == once.
- **Toggle behavior** — master off → identity; number toggle off → numbers untouched while
  other categories still apply.
- **Capitalization** — "all caps" next-word and "caps on/off" region behavior.

This satisfies the master design spec's "Unit (fast, deterministic): format module"
testing line (`docs/superpowers/specs/2026-06-24-mindflow-design.md:141`).

## 9. Phasing (by risk)

1. Module scaffold + `SpokenCommandsConfig` + master toggle wired end-to-end + **newlines**
   (highest value, lowest false-positive risk).
2. **Punctuation** category + smart spacing.
3. **Capitalization** category (stateful).
4. **Number conversion** category + `number_conversion_enabled` opt-in toggle (last; riskiest).

The empirical STT probe (§6) precedes step 1's table content.

## 10. Risks & mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| Newline command corrupts literal prose ("a new line of...") | Medium | Conservative standalone-only matching; ambiguity unit tests as a gate |
| STT already converts a spoken-punctuation phrase → double output | Medium | Mandatory empirical probe (§6) + idempotent table; idempotency tests |
| Number conversion mangles prose ("one idea" → "1 idea") | High if naive | Opt-in toggle, default off; conservative bounded parser; correctness over coverage |
| Command phrase arrives capitalized/punctuated by STT and misses match | Medium | Case-insensitive, trailing-punctuation-tolerant matcher |
| Scope creep into user-editable commands | Low | Editability explicitly fenced to M4 snippets |

## 11. References

- Master design spec: `docs/superpowers/specs/2026-06-24-mindflow-design.md`
  (M3 row line 122; formatting strategy §3; testing §7).
- Formatting research: `docs/research/2026-06-23-cpu-llm-formatting-deepdive.md`
  (Tier-1 vs Tier-2; punctuation-model rationale now superseded for the default path).
- Existing post-processing: `app/src-tauri/src/audio_toolkit/text.rs`
  (`apply_custom_words`, `filter_transcription_output`),
  `app/src-tauri/src/actions.rs` (`process_transcription_output`),
  `app/src-tauri/src/managers/transcription.rs` (`transcribe`).
- Settings/plumbing precedent: the `RecordingMode` end-to-end pattern from the
  hands-free mode work.
