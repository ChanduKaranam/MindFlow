# MindFlow M4 — Dictionary + Snippets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add user-defined text personalization — an exact `from → to` replacement engine (fixes stubborn proper-noun mishearings like "Poor Nachem Darao" → "Purna Chandra Rao") and spoken-cue snippets — plus unblock multi-word phrases in the existing fuzzy custom-words.

**Architecture:** One pure function `apply_replacements(text, &[Replacement]) -> String` in a new `replace` module (mirrors the M3 `format/spoken_commands.rs` pure-function pattern). It is called twice in `actions.rs::process_transcription_output` — once for the `replacements` list (Phase 1, dictionary) and once for the `snippets` list (Phase 2) — after the M3 spoken-commands pass and before the optional LLM. Two list settings, two `update_*` commands, two settings editors.

**Tech Stack:** Rust (Tauri 2 backend, `cargo test --lib`), tauri-specta bindings (hand-edited), React + TypeScript (`useSettings`, i18next), `bun` tooling.

## Global Constraints

- **CPU-only, fully local, zero network, no LLM** in this path.
- **No new Rust or JS dependencies.** Pure std-library string logic (no `regex` crate).
- **Deterministic & idempotent** (running twice == once, when `to` does not re-introduce a `from`).
- Replacement matching is **case-insensitive**, **boundary-aware** (never matches inside a larger alphanumeric word — `from="cat"` must not touch "category"), **whitespace-flexible** for multi-word `from` (runs of whitespace between matched words collapse), **longest-`from`-first**, **all occurrences**, **`to` inserted verbatim**, **empty/whitespace `from` ignored**.
- **English-only** UI strings (i18next; other locales fall back to English).
- **Settings backward-compat:** new fields use `#[serde(default)]`; old settings files lacking the keys load unchanged (no `deny_unknown_fields`). `get_default_settings()` is a **struct literal** — new fields must also be added there.
- **Bindings** `app/src/bindings.ts` are **hand-edited** to match the generated style (headless regen unavailable). The `Replacement` struct, the two `update_*` commands, and the two `AppSettings` fields are added by hand.
- **Conventional commits**; every commit body ends with: `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.
- Backend tests from `app/src-tauri`: `cargo test --lib`. Frontend checks from `app`: `bun run tsc` and `bun run lint` (2 pre-existing DevInject.tsx lint errors are out of scope).

## Pipeline seam (verified)

`app/src-tauri/src/actions.rs::process_transcription_output` currently does:

```rust
    final_text = crate::format::apply_spoken_commands(&final_text, &spoken_cfg);
    // <-- M4 replacement passes go HERE (Task 3 + Task 6), before the post_process block -->
    if post_process {
```

`settings` is already bound at the top of the fn (`let settings = get_settings(app);`).

## File Structure

- **Create** `app/src-tauri/src/replace/mod.rs` — module root; re-exports `apply_replacements`, `Replacement`.
- **Create** `app/src-tauri/src/replace/replacements.rs` — `Replacement` struct + pure engine + inline tests.
- **Modify** `app/src-tauri/src/lib.rs` — `mod replace;`; register `update_replacements` (Task 2) and `update_snippets` (Task 6).
- **Modify** `app/src-tauri/src/settings.rs` — `replacements` + `snippets` fields (+ struct-literal defaults) + a defaults test.
- **Modify** `app/src-tauri/src/shortcut/mod.rs` — `update_replacements` + `update_snippets` commands.
- **Modify** `app/src-tauri/src/actions.rs` — call `apply_replacements` for replacements (Task 3) and snippets (Task 6).
- **Modify** `app/src/bindings.ts` — `Replacement` type, two commands, two `AppSettings` fields (hand-edit).
- **Modify** `app/src/stores/settingsStore.ts` — `replacements` + `snippets` updaters.
- **Modify** `app/src/components/settings/CustomWords.tsx` — allow multi-word phrases (Task 4).
- **Create** `app/src/components/settings/ReplacementsEditor.tsx` — from→to list editor (Task 5).
- **Create** `app/src/components/settings/SnippetsEditor.tsx` — cue→expansion list editor (Task 7).
- **Modify** `app/src/components/settings/advanced/AdvancedSettings.tsx` — mount the two editors under a "Dictionary" `SettingsGroup`.
- **Modify** `app/src/components/settings/index.ts` — export the two new editors.
- **Modify** `app/src/i18n/locales/en/translation.json` — strings for both editors.

## Task order

T1 (engine) → T2 (replacements setting+command) → T3 (wire replacements) → T4 (custom-words phrases) → T5 (replacements UI) **[Phase 1 done — dictionary works]** → T6 (snippets setting+command+wire) → T7 (snippets UI).

---

### Task 1: Pure `replace` engine + `Replacement` struct

**Files:**
- Create: `app/src-tauri/src/replace/mod.rs`
- Create: `app/src-tauri/src/replace/replacements.rs`
- Modify: `app/src-tauri/src/lib.rs` (add `mod replace;` among the top-level `mod` declarations)
- Test: inline `#[cfg(test)] mod tests` in `replacements.rs`

**Interfaces:**
- Produces:
  - `pub struct Replacement { pub from: String, pub to: String }` (derives `Serialize, Deserialize, Debug, Clone, specta::Type`)
  - `pub fn apply_replacements(text: &str, rules: &[Replacement]) -> String`

- [ ] **Step 1: Module root.** `app/src-tauri/src/replace/mod.rs`:

```rust
mod replacements;

pub use replacements::{apply_replacements, Replacement};
```

- [ ] **Step 2: Register module.** In `app/src-tauri/src/lib.rs`, add among the other top-level `mod` lines:

```rust
mod replace;
```

- [ ] **Step 3: Write failing tests** in `app/src-tauri/src/replace/replacements.rs`:

```rust
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}

pub fn apply_replacements(_text: &str, _rules: &[Replacement]) -> String {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(from: &str, to: &str) -> Replacement {
        Replacement { from: from.into(), to: to.into() }
    }

    #[test]
    fn case_insensitive_verbatim_replacement() {
        let rules = vec![rule("poor nachem darao", "Purna Chandra Rao")];
        assert_eq!(
            apply_replacements("i am poor nachem darao", &rules),
            "i am Purna Chandra Rao"
        );
        assert_eq!(
            apply_replacements("I AM POOR NACHEM DARAO", &rules),
            "I AM Purna Chandra Rao"
        );
    }

    #[test]
    fn does_not_match_inside_larger_word() {
        let rules = vec![rule("cat", "dog")];
        assert_eq!(apply_replacements("category cat", &rules), "category dog");
    }

    #[test]
    fn longest_from_wins() {
        let rules = vec![rule("york", "Y"), rule("new york", "NYC")];
        assert_eq!(apply_replacements("new york", &rules), "NYC");
    }

    #[test]
    fn whitespace_between_words_is_flexible() {
        let rules = vec![rule("my email", "x@y.com")];
        assert_eq!(apply_replacements("my   email", &rules), "x@y.com");
    }

    #[test]
    fn replaces_all_occurrences() {
        let rules = vec![rule("cat", "dog")];
        assert_eq!(apply_replacements("cat and cat", &rules), "dog and dog");
    }

    #[test]
    fn non_whitespace_separator_does_not_match_phrase() {
        let rules = vec![rule("my email", "X")];
        assert_eq!(apply_replacements("my-email", &rules), "my-email");
        assert_eq!(apply_replacements("my, email", &rules), "my, email");
    }

    #[test]
    fn empty_from_is_ignored() {
        let rules = vec![rule("   ", "x")];
        assert_eq!(apply_replacements("hello", &rules), "hello");
    }

    #[test]
    fn empty_to_deletes_the_match() {
        let rules = vec![rule("um", "")];
        assert_eq!(apply_replacements("a um b", &rules), "a  b");
    }
}
```

- [ ] **Step 4: Run to verify failure**

Run: `cd app/src-tauri && cargo test --lib replace::replacements`
Expected: FAIL (`unimplemented!`).

- [ ] **Step 5: Implement the engine.** Replace the `apply_replacements` stub in `replacements.rs` (keep the `Replacement` struct above it):

```rust
/// Byte-offset spans of maximal alphanumeric runs ("words") in `text`.
fn word_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if c.is_alphanumeric() {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            spans.push((s, i));
        }
    }
    if let Some(s) = start {
        spans.push((s, text.len()));
    }
    spans
}

/// Replace every boundary-aligned, whitespace-flexible occurrence of the word
/// sequence `from_words` (already lowercased) with `to`.
fn replace_phrase(text: &str, from_words: &[String], to: &str) -> String {
    let spans = word_spans(text);
    let n = from_words.len();
    if n == 0 {
        return text.to_string();
    }
    let mut result = String::new();
    let mut copied_to = 0usize;
    let mut i = 0usize;
    while i + n <= spans.len() {
        let mut matches = true;
        for k in 0..n {
            let (s, e) = spans[i + k];
            if text[s..e].to_lowercase() != from_words[k] {
                matches = false;
                break;
            }
            if k > 0 {
                let prev_end = spans[i + k - 1].1;
                if !text[prev_end..s].chars().all(|c| c.is_whitespace()) {
                    matches = false;
                    break;
                }
            }
        }
        if matches {
            let match_start = spans[i].0;
            let match_end = spans[i + n - 1].1;
            result.push_str(&text[copied_to..match_start]);
            result.push_str(to);
            copied_to = match_end;
            i += n;
        } else {
            i += 1;
        }
    }
    result.push_str(&text[copied_to..]);
    result
}

pub fn apply_replacements(text: &str, rules: &[Replacement]) -> String {
    // Prepare: drop empty `from`; precompute lowercased word lists.
    let mut prepared: Vec<(Vec<String>, &str)> = rules
        .iter()
        .filter_map(|r| {
            let words: Vec<String> =
                r.from.split_whitespace().map(|w| w.to_lowercase()).collect();
            if words.is_empty() {
                None
            } else {
                Some((words, r.to.as_str()))
            }
        })
        .collect();
    // Longest phrase first (by word count, then by character length) so a
    // multi-word rule wins over a shorter rule it contains.
    prepared.sort_by(|a, b| {
        b.0.len()
            .cmp(&a.0.len())
            .then_with(|| b.0.join(" ").len().cmp(&a.0.join(" ").len()))
    });

    let mut current = text.to_string();
    for (from_words, to) in &prepared {
        current = replace_phrase(&current, from_words, to);
    }
    current
}
```

- [ ] **Step 6: Run to verify pass**

Run: `cd app/src-tauri && cargo test --lib replace::replacements`
Expected: PASS (8 tests).

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src/replace/mod.rs app/src-tauri/src/replace/replacements.rs app/src-tauri/src/lib.rs
git commit -m "feat(m4): add pure replacement engine and Replacement struct

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: `replacements` setting + `update_replacements` command

**Files:**
- Modify: `app/src-tauri/src/settings.rs` (add field near `custom_words` ~line 372; add to the `get_default_settings` struct literal ~line 809; add a defaults test)
- Modify: `app/src-tauri/src/shortcut/mod.rs` (add command next to `update_custom_words` ~line 652)
- Modify: `app/src-tauri/src/lib.rs` (register in `collect_commands![` next to `shortcut::update_custom_words` ~line 386)

**Interfaces:**
- Consumes: `crate::replace::Replacement` (Task 1); `crate::settings::{get_settings, write_settings}`.
- Produces:
  - `AppSettings.replacements: Vec<crate::replace::Replacement>` (default empty)
  - `pub fn update_replacements(app: AppHandle, replacements: Vec<crate::replace::Replacement>) -> Result<(), String>`

- [ ] **Step 1: Import the struct + add the field.** In `app/src-tauri/src/settings.rs`, ensure `Replacement` is in scope (add `use crate::replace::Replacement;` near the top imports), then add next to `custom_words`:

```rust
    #[serde(default)]
    pub replacements: Vec<Replacement>,
```

- [ ] **Step 2: Add to the default struct literal.** In `get_default_settings()` (the struct literal, near `custom_words: Vec::new(),`):

```rust
        replacements: Vec::new(),
```

- [ ] **Step 3: Write a failing defaults test.** In the existing `#[cfg(test)] mod tests` in `settings.rs`:

```rust
    #[test]
    fn replacements_default_empty() {
        let settings = get_default_settings();
        assert!(settings.replacements.is_empty());
    }
```

- [ ] **Step 4: Run to verify (compiles + passes once fields exist)**

Run: `cd app/src-tauri && cargo test --lib replacements_default_empty`
Expected: PASS after Steps 1–2.

- [ ] **Step 5: Add the command.** In `app/src-tauri/src/shortcut/mod.rs`, mirroring `update_custom_words`:

```rust
#[tauri::command]
#[specta::specta]
pub fn update_replacements(
    app: AppHandle,
    replacements: Vec<crate::replace::Replacement>,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.replacements = replacements;
    settings::write_settings(&app, settings);
    Ok(())
}
```

- [ ] **Step 6: Register the command.** In `app/src-tauri/src/lib.rs` `collect_commands![`, next to `shortcut::update_custom_words,`:

```rust
        shortcut::update_replacements,
```

- [ ] **Step 7: Build + test**

Run: `cd app/src-tauri && cargo test --lib`
Expected: PASS (existing suite + new test); crate compiles with the registered command.

- [ ] **Step 8: Commit**

```bash
git add app/src-tauri/src/settings.rs app/src-tauri/src/shortcut/mod.rs app/src-tauri/src/lib.rs
git commit -m "feat(m4): add replacements setting and update_replacements command

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Wire replacements into the pipeline

**Files:**
- Modify: `app/src-tauri/src/actions.rs` (in `process_transcription_output`, after the `apply_spoken_commands` line, before `if post_process`)

**Interfaces:**
- Consumes: `crate::replace::apply_replacements` (Task 1); `settings.replacements` (Task 2).

- [ ] **Step 1: Insert the pass.** In `actions.rs`, immediately after:

```rust
    final_text = crate::format::apply_spoken_commands(&final_text, &spoken_cfg);
```

add:

```rust
    // M4: user-defined exact replacements (dictionary fixes for stubborn mishearings,
    // proper nouns, abbreviations). Deterministic, CPU-only, no network.
    final_text = crate::replace::apply_replacements(&final_text, &settings.replacements);
```

- [ ] **Step 2: Build**

Run: `cd app/src-tauri && cargo test --lib`
Expected: PASS; crate compiles.

- [ ] **Step 3: Commit**

```bash
git add app/src-tauri/src/actions.rs
git commit -m "feat(m4): apply user replacements before injection

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: Allow multi-word phrases in Custom Words

**Files:**
- Modify: `app/src/components/settings/CustomWords.tsx` (relax the no-spaces guards in `handleAddWord` and the Add-button `disabled`)

**Interfaces:** none new — behavior change only.

- [ ] **Step 1: Relax the add handler.** In `CustomWords.tsx`, change `handleAddWord` so a sanitized phrase with internal spaces is accepted (keep the dangerous-char strip and the 50-char cap):

```tsx
    const handleAddWord = () => {
      const trimmedWord = newWord.trim();
      const sanitizedWord = trimmedWord.replace(/[<>"'&]/g, "");
      if (sanitizedWord && sanitizedWord.length <= 50) {
        if (customWords.includes(sanitizedWord)) {
          toast.error(
            t("settings.advanced.customWords.duplicate", {
              word: sanitizedWord,
            }),
          );
          return;
        }
        updateSetting("custom_words", [...customWords, sanitizedWord]);
        setNewWord("");
      }
    };
```

- [ ] **Step 2: Relax the button disabled state.** Remove the `newWord.includes(" ")` clause:

```tsx
              disabled={
                !newWord.trim() ||
                newWord.trim().length > 50 ||
                isUpdating("custom_words")
              }
```

- [ ] **Step 3: Typecheck + lint**

Run: `cd app && bun run tsc && bun run lint`
Expected: no new errors (the 2 pre-existing DevInject.tsx errors are out of scope).

- [ ] **Step 4: Commit**

```bash
git add app/src/components/settings/CustomWords.tsx
git commit -m "feat(m4): allow multi-word phrases in custom words

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: Replacements editor UI

**Files:**
- Modify: `app/src/bindings.ts` (hand-add `Replacement` type, `updateReplacements` command, `replacements` field)
- Modify: `app/src/stores/settingsStore.ts` (add `replacements` updater)
- Create: `app/src/components/settings/ReplacementsEditor.tsx`
- Modify: `app/src/components/settings/index.ts` (export it)
- Modify: `app/src/components/settings/advanced/AdvancedSettings.tsx` (mount under a "Dictionary" `SettingsGroup`)
- Modify: `app/src/i18n/locales/en/translation.json` (`settings.advanced.replacements.*`)

**Interfaces:**
- Consumes: Task 2 command `update_replacements`; setting key `replacements`.

- [ ] **Step 1: Hand-edit `bindings.ts` — the type.** Near the other exported types (e.g. after `LLMPrompt`):

```typescript
export type Replacement = { from: string; to: string }
```

- [ ] **Step 2: Hand-edit `bindings.ts` — the command.** In the `commands` object, mirroring `updateCustomWords`:

```typescript
async updateReplacements(replacements: Replacement[]) : Promise<Result<null, string>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("update_replacements", { replacements }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
```

- [ ] **Step 3: Hand-edit `bindings.ts` — the `AppSettings` field.** Next to `custom_words?: string[];`:

```typescript
replacements?: Replacement[];
```

- [ ] **Step 4: Add the store updater.** In `app/src/stores/settingsStore.ts`, next to the `custom_words` updater:

```typescript
  replacements: (value) => commands.updateReplacements(value as Replacement[]),
```

(Add `Replacement` to the existing `@/bindings` import in that file.)

- [ ] **Step 5: Create the editor** `app/src/components/settings/ReplacementsEditor.tsx`:

```tsx
import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettings } from "../../hooks/useSettings";
import type { Replacement } from "@/bindings";

interface ReplacementsEditorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const ReplacementsEditor: React.FC<ReplacementsEditorProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const replacements = (getSetting("replacements") ?? []) as Replacement[];
    const [from, setFrom] = useState("");
    const [to, setTo] = useState("");

    const sanitize = (s: string) => s.replace(/[<>"'&]/g, "");

    const handleAdd = () => {
      const f = sanitize(from.trim());
      const tt = sanitize(to.trim());
      if (!f || f.length > 100 || tt.length > 200) return;
      if (replacements.some((r) => r.from.toLowerCase() === f.toLowerCase())) {
        toast.error(t("settings.advanced.replacements.duplicate", { word: f }));
        return;
      }
      updateSetting("replacements", [...replacements, { from: f, to: tt }]);
      setFrom("");
      setTo("");
    };

    const handleRemove = (index: number) => {
      updateSetting(
        "replacements",
        replacements.filter((_, i) => i !== index),
      );
    };

    return (
      <SettingContainer
        title={t("settings.advanced.replacements.title")}
        description={t("settings.advanced.replacements.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        layout="vertical"
      >
        <div className="flex flex-col gap-2 w-full">
          <div className="flex gap-2">
            <input
              type="text"
              value={from}
              onChange={(e) => setFrom(e.target.value)}
              placeholder={t("settings.advanced.replacements.fromPlaceholder")}
              className="flex-1 px-2 py-1 rounded border border-mid-gray/30 bg-transparent"
            />
            <input
              type="text"
              value={to}
              onChange={(e) => setTo(e.target.value)}
              placeholder={t("settings.advanced.replacements.toPlaceholder")}
              className="flex-1 px-2 py-1 rounded border border-mid-gray/30 bg-transparent"
              onKeyDown={(e) => {
                if (e.key === "Enter") handleAdd();
              }}
            />
            <button
              type="button"
              onClick={handleAdd}
              disabled={!from.trim() || isUpdating("replacements")}
              className="px-3 py-1 rounded bg-background-ui disabled:opacity-50"
            >
              {t("settings.advanced.replacements.add")}
            </button>
          </div>
          <div className="flex flex-col gap-1">
            {replacements.map((r, i) => (
              <div
                key={`${r.from}-${i}`}
                className="flex items-center justify-between px-2 py-1 rounded bg-mid-gray/10"
              >
                <span className="text-sm">
                  {r.from} → {r.to || "∅"}
                </span>
                <button
                  type="button"
                  onClick={() => handleRemove(i)}
                  aria-label={t("settings.advanced.replacements.remove")}
                  className="text-mid-gray hover:text-logo-primary"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        </div>
      </SettingContainer>
    );
  },
);
```

(If `SettingContainer` does not accept `layout="vertical"`, check `app/src/components/ui/SettingContainer.tsx` for the supported props and use the matching one — mirror how `CustomWords.tsx` lays out its input + tag list.)

- [ ] **Step 6: Export it.** In `app/src/components/settings/index.ts`:

```typescript
export { ReplacementsEditor } from "./ReplacementsEditor";
```

- [ ] **Step 7: Mount under a Dictionary group.** In `app/src/components/settings/advanced/AdvancedSettings.tsx`, import `ReplacementsEditor` and `SettingsGroup` (if not already imported), and add near the existing `<CustomWords ... />`:

```tsx
      <SettingsGroup title={t("settings.advanced.dictionary.title")}>
        <CustomWords descriptionMode="tooltip" grouped />
        <ReplacementsEditor descriptionMode="tooltip" grouped />
      </SettingsGroup>
```

(If `CustomWords` is already inside another group, move its `<CustomWords ... />` line into this new group rather than rendering it twice. Confirm by reading the current `AdvancedSettings.tsx` structure first.)

- [ ] **Step 8: Add i18n strings.** In `app/src/i18n/locales/en/translation.json`, under `settings.advanced`:

```json
"dictionary": {
  "title": "Dictionary"
},
"replacements": {
  "title": "Replacements",
  "description": "Always replace a misheard word or phrase with the exact text you want.",
  "fromPlaceholder": "Heard (e.g. poor nachem darao)",
  "toPlaceholder": "Replace with (e.g. Purna Chandra Rao)",
  "add": "Add",
  "remove": "Remove",
  "duplicate": "A replacement for \"{{word}}\" already exists"
}
```

- [ ] **Step 9: Typecheck + lint**

Run: `cd app && bun run tsc && bun run lint`
Expected: no new errors.

- [ ] **Step 10: Commit**

```bash
git add app/src/bindings.ts app/src/stores/settingsStore.ts app/src/components/settings/ReplacementsEditor.tsx app/src/components/settings/index.ts app/src/components/settings/advanced/AdvancedSettings.tsx app/src/i18n/locales/en/translation.json
git commit -m "feat(m4): add replacements editor UI under a Dictionary group

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: Snippets setting + command + pipeline wiring (Phase 2 backend)

**Files:**
- Modify: `app/src-tauri/src/settings.rs` (add `snippets` field + struct-literal default + defaults test)
- Modify: `app/src-tauri/src/shortcut/mod.rs` (add `update_snippets` command)
- Modify: `app/src-tauri/src/lib.rs` (register `update_snippets`)
- Modify: `app/src-tauri/src/actions.rs` (apply snippets right after the replacements pass)

**Interfaces:**
- Consumes: `crate::replace::{apply_replacements, Replacement}`.
- Produces:
  - `AppSettings.snippets: Vec<crate::replace::Replacement>` (default empty)
  - `pub fn update_snippets(app: AppHandle, snippets: Vec<crate::replace::Replacement>) -> Result<(), String>`

- [ ] **Step 1: Add the field + default.** In `settings.rs`, next to `replacements`:

```rust
    #[serde(default)]
    pub snippets: Vec<Replacement>,
```

and in the `get_default_settings` literal, next to `replacements: Vec::new(),`:

```rust
        snippets: Vec::new(),
```

- [ ] **Step 2: Failing defaults test.** In `settings.rs` tests:

```rust
    #[test]
    fn snippets_default_empty() {
        let settings = get_default_settings();
        assert!(settings.snippets.is_empty());
    }
```

- [ ] **Step 3: Add the command.** In `shortcut/mod.rs`, mirroring `update_replacements`:

```rust
#[tauri::command]
#[specta::specta]
pub fn update_snippets(
    app: AppHandle,
    snippets: Vec<crate::replace::Replacement>,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.snippets = snippets;
    settings::write_settings(&app, settings);
    Ok(())
}
```

- [ ] **Step 4: Register it.** In `lib.rs` `collect_commands![`, next to `shortcut::update_replacements,`:

```rust
        shortcut::update_snippets,
```

- [ ] **Step 5: Wire into the pipeline.** In `actions.rs`, immediately after the Task 3 replacements line:

```rust
    // M4 Phase 2: spoken-cue snippets (text expansion), same engine as replacements.
    final_text = crate::replace::apply_replacements(&final_text, &settings.snippets);
```

- [ ] **Step 6: Build + test**

Run: `cd app/src-tauri && cargo test --lib`
Expected: PASS (existing + new defaults test).

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src/settings.rs app/src-tauri/src/shortcut/mod.rs app/src-tauri/src/lib.rs app/src-tauri/src/actions.rs
git commit -m "feat(m4): add snippets setting, command, and pipeline pass

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 7: Snippets editor UI (Phase 2 frontend)

**Files:**
- Modify: `app/src/bindings.ts` (hand-add `updateSnippets` command + `snippets` field; reuse `Replacement` type)
- Modify: `app/src/stores/settingsStore.ts` (add `snippets` updater)
- Create: `app/src/components/settings/SnippetsEditor.tsx`
- Modify: `app/src/components/settings/index.ts` (export it)
- Modify: `app/src/components/settings/advanced/AdvancedSettings.tsx` (mount in the Dictionary group)
- Modify: `app/src/i18n/locales/en/translation.json` (`settings.advanced.snippets.*`)

**Interfaces:**
- Consumes: Task 6 command `update_snippets`; setting key `snippets`; type `Replacement` (Task 5).

- [ ] **Step 1: Hand-edit `bindings.ts` — command.** Mirroring `updateReplacements`:

```typescript
async updateSnippets(snippets: Replacement[]) : Promise<Result<null, string>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("update_snippets", { snippets }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
```

- [ ] **Step 2: Hand-edit `bindings.ts` — `AppSettings` field.** Next to `replacements?: Replacement[];`:

```typescript
snippets?: Replacement[];
```

- [ ] **Step 3: Store updater.** In `settingsStore.ts`, next to `replacements`:

```typescript
  snippets: (value) => commands.updateSnippets(value as Replacement[]),
```

- [ ] **Step 4: Create the editor** `app/src/components/settings/SnippetsEditor.tsx`. Identical structure to `ReplacementsEditor.tsx` but reads/writes the `snippets` key, allows a longer `to` (multi-word expansion), and uses `settings.advanced.snippets.*` strings:

```tsx
import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettings } from "../../hooks/useSettings";
import type { Replacement } from "@/bindings";

interface SnippetsEditorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const SnippetsEditor: React.FC<SnippetsEditorProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const snippets = (getSetting("snippets") ?? []) as Replacement[];
    const [from, setFrom] = useState("");
    const [to, setTo] = useState("");

    const sanitize = (s: string) => s.replace(/[<>"'&]/g, "");

    const handleAdd = () => {
      const f = sanitize(from.trim());
      const tt = sanitize(to.trim());
      if (!f || f.length > 100 || tt.length > 1000) return;
      if (snippets.some((r) => r.from.toLowerCase() === f.toLowerCase())) {
        toast.error(t("settings.advanced.snippets.duplicate", { word: f }));
        return;
      }
      updateSetting("snippets", [...snippets, { from: f, to: tt }]);
      setFrom("");
      setTo("");
    };

    const handleRemove = (index: number) => {
      updateSetting(
        "snippets",
        snippets.filter((_, i) => i !== index),
      );
    };

    return (
      <SettingContainer
        title={t("settings.advanced.snippets.title")}
        description={t("settings.advanced.snippets.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        layout="vertical"
      >
        <div className="flex flex-col gap-2 w-full">
          <div className="flex gap-2">
            <input
              type="text"
              value={from}
              onChange={(e) => setFrom(e.target.value)}
              placeholder={t("settings.advanced.snippets.fromPlaceholder")}
              className="flex-1 px-2 py-1 rounded border border-mid-gray/30 bg-transparent"
            />
            <input
              type="text"
              value={to}
              onChange={(e) => setTo(e.target.value)}
              placeholder={t("settings.advanced.snippets.toPlaceholder")}
              className="flex-1 px-2 py-1 rounded border border-mid-gray/30 bg-transparent"
              onKeyDown={(e) => {
                if (e.key === "Enter") handleAdd();
              }}
            />
            <button
              type="button"
              onClick={handleAdd}
              disabled={!from.trim() || isUpdating("snippets")}
              className="px-3 py-1 rounded bg-background-ui disabled:opacity-50"
            >
              {t("settings.advanced.snippets.add")}
            </button>
          </div>
          <div className="flex flex-col gap-1">
            {snippets.map((r, i) => (
              <div
                key={`${r.from}-${i}`}
                className="flex items-center justify-between px-2 py-1 rounded bg-mid-gray/10"
              >
                <span className="text-sm">
                  {r.from} → {r.to || "∅"}
                </span>
                <button
                  type="button"
                  onClick={() => handleRemove(i)}
                  aria-label={t("settings.advanced.snippets.remove")}
                  className="text-mid-gray hover:text-logo-primary"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        </div>
      </SettingContainer>
    );
  },
);
```

- [ ] **Step 5: Export it.** In `app/src/components/settings/index.ts`:

```typescript
export { SnippetsEditor } from "./SnippetsEditor";
```

- [ ] **Step 6: Mount it.** In `AdvancedSettings.tsx`, add inside the Dictionary `SettingsGroup` after `<ReplacementsEditor ... />`:

```tsx
        <SnippetsEditor descriptionMode="tooltip" grouped />
```

- [ ] **Step 7: i18n strings.** In `translation.json`, under `settings.advanced`:

```json
"snippets": {
  "title": "Snippets",
  "description": "Say a short cue and have it expand to longer boilerplate text.",
  "fromPlaceholder": "Spoken cue (e.g. my email)",
  "toPlaceholder": "Expands to (e.g. you@example.com)",
  "add": "Add",
  "remove": "Remove",
  "duplicate": "A snippet for \"{{word}}\" already exists"
}
```

- [ ] **Step 8: Typecheck + lint**

Run: `cd app && bun run tsc && bun run lint`
Expected: no new errors.

- [ ] **Step 9: Commit**

```bash
git add app/src/bindings.ts app/src/stores/settingsStore.ts app/src/components/settings/SnippetsEditor.tsx app/src/components/settings/index.ts app/src/components/settings/advanced/AdvancedSettings.tsx app/src/i18n/locales/en/translation.json
git commit -m "feat(m4): add snippets editor UI (text expansion)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Final verification (after all tasks)

- [ ] `cd app/src-tauri && cargo test --lib` — full suite passes (engine + defaults tests).
- [ ] `cd app && bun run tsc && bun run lint` — no new errors.
- [ ] Manual: add a replacement "poor nachem darao → Purna Chandra Rao", dictate it, confirm the corrected name; add a snippet "my email → an address", say the cue, confirm expansion; add a multi-word custom word and confirm it saves.

## Self-Review (plan vs. spec)

**Spec coverage:** §3 three tools → Vocabulary phrases (T4), Replacements engine (T1) + setting/command (T2) + wiring (T3) + UI (T5), Snippets (T6 backend + T7 UI); §4 engine + matching rules (T1, all encoded as tests: case-insensitive, boundary, longest-first, whitespace-flexible, all-occurrences, empty-from, empty-to); §5 Phase 1 (T1–T5); §6 Phase 2 (T6–T7); §7 pipeline order (T3 then T6, after M3, before LLM); §8 settings/UI Dictionary group (T5/T7); §9 error handling (empty from ignored — T1 test; duplicate — UI guard T5/T7; empty to deletes — T1 test; serde defaults — T2/T6); §10 testing (engine unit tests + defaults). No gaps. Power Mode correctly excluded.

**Placeholder scan:** none — every code step is complete. Conditional notes (SettingContainer props, AdvancedSettings structure) instruct the implementer to verify against real code and adapt, with the fallback named — these are integration checks, not placeholders.

**Type consistency:** `Replacement { from, to }` defined in T1, used identically in T2/T5/T6/T7 (Rust `crate::replace::Replacement`, TS `Replacement`). `apply_replacements(&str, &[Replacement]) -> String` consistent T1→T3→T6. Command names `update_replacements`/`update_snippets` and setting keys `replacements`/`snippets` match across Rust (T2/T6), bindings/store (T5/T7), and pipeline (T3/T6). Store updaters cast `value as Replacement[]`.
