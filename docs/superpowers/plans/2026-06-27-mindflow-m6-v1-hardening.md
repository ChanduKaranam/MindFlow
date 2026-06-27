# MindFlow M6 — v1 Hardening / Release Gate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove and lock in MindFlow's "fully local, zero-network, CPU-only dictation" guarantee, and produce the v1.0 release-gate documentation.

**Architecture:** One new Rust integration test that statically guards the dictation hot-path against network symbols (always runs in CI); an extension to the existing offline transcription gate to also cover formatting + dictionary; and four documents (audit, DoD checklist, 3-OS install runbook, release checklist). No new product feature, no UI change.

**Tech Stack:** Rust (`cargo test`, integration tests under `app/src-tauri/tests/`), Markdown docs under `docs/superpowers/`.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-06-27-mindflow-m6-v1-hardening-design.md`.
- The "zero-network" guarantee is scoped to **core dictation** (capture → VAD → STT → format → inject). Opt-in network surfaces — model download, app-update check (`update_checks_enabled`, default `true`), post-processing LLM (`post_process_enabled`, default `false`) — are out of that path and allowed.
- Post-processing/LLM is **off by default and deferred to the v2 Tier-2 spec**; M6 only verifies + documents it. No UI removal.
- Do NOT touch `handy_keys` / `HandyKeys*` / crate names `handy` | `handy_app_lib`.
- Gate per task: `cargo test` (the new tests + existing suite) green; for any frontend touch (none expected) `bun run build` + `bun run lint`. The build is slow (~5 min) — allow ≥600s.
- Docs must be complete — no `TBD`/`TODO`/placeholder sections.
- Run all `cargo` commands from `app/src-tauri/`.

## File Structure

- Create `app/src-tauri/tests/no_network_in_hot_path.rs` — the static network-symbol guard (Task 2).
- Modify `app/src-tauri/tests/zero_network.rs` — extend the offline gate with format + replacements (Task 3).
- Create `docs/superpowers/audits/2026-06-27-m6-zero-network-audit.md` (Task 1).
- Create `docs/superpowers/checklists/m6-definition-of-done.md` (Task 4).
- Create `docs/superpowers/checklists/m6-3os-install-runbook.md` (Task 5).
- Create `docs/superpowers/checklists/m6-v1-release-gate.md` (Task 6).

Authoritative codebase facts (verified) used throughout:
- Hot-path dirs/files under `app/src-tauri/src/`: `audio_toolkit/`, `format/`, `replace/`, `managers/transcription.rs`, `managers/audio.rs`, `transcription_coordinator.rs`, `actions.rs`, `signal_handle.rs`, `shortcut/`, `clipboard.rs`, `input.rs`.
- Excluded opt-in-network files: `llm_client.rs`, `managers/model.rs`, `commands/models.rs`, `lib.rs` (updater wiring).
- `app/src-tauri/src/replace/replacements.rs`: `pub struct Replacement { pub from: String, pub to: String }` and `pub fn apply_replacements(text: &str, rules: &[Replacement]) -> String`.
- `app/src-tauri/src/format/spoken_commands.rs`: `pub fn apply_spoken_commands(text: &str, config: &SpokenCommandsConfig) -> String`.
- Defaults: `default_update_checks_enabled() -> true`, `default_post_process_enabled() -> false`.
- Existing offline gate: `app/src-tauri/tests/zero_network.rs`, gated on env `MINDFLOW_TEST_MODEL`, fixture `tests/fixtures/jfk.wav`.

---

### Task 1: Zero-network code audit doc

**Files:**
- Create: `docs/superpowers/audits/2026-06-27-m6-zero-network-audit.md`

**Interfaces:**
- Produces: the authoritative list of hot-path modules and the audited network sites that Tasks 2 and 4 reference. Keep the hot-path module list identical to Task 2's `HOT_PATH` array.

- [ ] **Step 1: Enumerate every network call site.** From `app/src-tauri/`, run and record the output:

```bash
cd app/src-tauri
grep -rnE "reqwest|hyper|ureq|std::net|TcpStream|TcpListener|UdpSocket|\.connect\(" src --include=*.rs | grep -vE "^\s*//"
```

Classify each hit into: (a) model download, (b) app updater, (c) post-processing LLM. Confirm there are no other categories.

- [ ] **Step 2: Write the audit doc** with these exact sections:
  1. **Guarantee** — "Core dictation (capture → VAD → STT → format → inject → output) makes zero network calls. Verified by `tests/no_network_in_hot_path.rs` and this audit."
  2. **Network surface table** — the three sites with `file:line`, trigger, and default state (`update_checks_enabled=true`, `post_process_enabled=false`), each marked allowed/opt-in.
  3. **Hot-path walk** — list each hot-path module (`audio_toolkit/`, `format/`, `replace/`, `managers/transcription.rs`, `managers/audio.rs`, `transcription_coordinator.rs`, `actions.rs`, `signal_handle.rs`, `shortcut/`, `clipboard.rs`, `input.rs`) and state "no network symbols" for each (cite the grep result).
  4. **Excluded files** — `llm_client.rs`, `managers/model.rs`, `commands/models.rs`, `lib.rs` (updater) — why each is a legitimate opt-in.
  5. **Post-processing v1 note** — off by default; deferred to v2 Tier-2 LLM spec.
  6. **No telemetry/analytics, fonts bundled (no CDN)** — state explicitly, with the grep that shows no analytics SDK.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/audits/2026-06-27-m6-zero-network-audit.md
git commit -m "docs(m6): zero-network code audit"
```

---

### Task 2: Network-symbol guard test

**Files:**
- Create: `app/src-tauri/tests/no_network_in_hot_path.rs`

**Interfaces:**
- Consumes: the hot-path module list from Task 1 (must match exactly).
- Produces: a CI test `dictation_hot_path_has_no_network_symbols` that fails if a network symbol enters the hot-path.

- [ ] **Step 1: Write the test file** exactly:

```rust
//! Guard: the dictation hot-path must never make network calls.
//!
//! This is the always-on regression gate behind MindFlow's "fully local,
//! zero-network dictation" guarantee. It scans the source of every hot-path
//! module and fails if a network symbol (reqwest/hyper/ureq/std::net/sockets)
//! appears. Opt-in network features (model download, app update check,
//! post-processing LLM) live OUTSIDE this set and are intentionally excluded.
//!
//! If you legitimately add a network feature, it must NOT be in the dictation
//! path — put it in its own module and leave it out of `HOT_PATH`.

use std::fs;
use std::path::{Path, PathBuf};

/// Hot-path modules/files, relative to `src-tauri/src/`. MUST match the
/// hot-path list in docs/superpowers/audits/2026-06-27-m6-zero-network-audit.md.
const HOT_PATH: &[&str] = &[
    "audio_toolkit",
    "format",
    "replace",
    "managers/transcription.rs",
    "managers/audio.rs",
    "transcription_coordinator.rs",
    "actions.rs",
    "signal_handle.rs",
    "shortcut",
    "clipboard.rs",
    "input.rs",
];

/// Network symbols that must never appear in the hot-path. Crate/std-level
/// signals (an actual network call needs one of these), not raw URLs — URLs
/// show up in doc comments and would be noisy false positives.
const FORBIDDEN: &[&str] = &[
    "reqwest",
    "hyper::",
    "ureq",
    "std::net",
    "TcpStream",
    "TcpListener",
    "UdpSocket",
];

fn collect_rs(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().map_or(false, |e| e == "rs") {
            out.push(path.to_path_buf());
        }
        return;
    }
    for entry in fs::read_dir(path).expect("read_dir hot-path") {
        collect_rs(&entry.expect("dir entry").path(), out);
    }
}

/// Strip line comments so a `// see https://… reqwest docs` comment does not
/// trip the guard. Conservative: only removes a trailing `//…` and skips
/// full-line `//` / block-comment `*` lines.
fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with('*') || t.starts_with("/*")
}

#[test]
fn dictation_hot_path_has_no_network_symbols() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    for entry in HOT_PATH {
        let p = src.join(entry);
        assert!(
            p.exists(),
            "hot-path entry missing — update HOT_PATH and the audit doc: {}",
            p.display()
        );
        collect_rs(&p, &mut files);
    }
    assert!(!files.is_empty(), "no hot-path source files collected");

    let mut violations = Vec::new();
    for f in &files {
        let content = fs::read_to_string(f).expect("read hot-path file");
        for (i, line) in content.lines().enumerate() {
            if is_comment(line) {
                continue;
            }
            for sym in FORBIDDEN {
                if line.contains(sym) {
                    violations.push(format!(
                        "{}:{}: forbidden network symbol {:?}\n      {}",
                        f.display(),
                        i + 1,
                        sym,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Network symbols found in the dictation hot-path — the 'fully local' \
         guarantee is broken. Move network code out of the hot-path:\n{}",
        violations.join("\n")
    );
}
```

- [ ] **Step 2: Run it — expect PASS** (the real hot-path has no network symbols today):

```bash
cd app/src-tauri
cargo test --test no_network_in_hot_path -- --nocapture
```
Expected: `test dictation_hot_path_has_no_network_symbols ... ok`.

- [ ] **Step 3: Prove it CATCHES a violation.** Temporarily add a line `let _ = reqwest::Client::new();` inside any function in `app/src-tauri/src/actions.rs`, then:

```bash
cargo test --test no_network_in_hot_path
```
Expected: FAIL listing `actions.rs:<line>: forbidden network symbol "reqwest"`.

- [ ] **Step 4: Revert the planted line** and re-run; expect PASS again. Confirm `git diff app/src-tauri/src/actions.rs` is empty.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/tests/no_network_in_hot_path.rs
git commit -m "test(m6): guard dictation hot-path against network symbols"
```

---

### Task 3: Extend the offline gate with formatting + dictionary

**Files:**
- Modify: `app/src-tauri/tests/zero_network.rs` (after the transcription assertions, before the final `eprintln!`)

**Interfaces:**
- Consumes: `crate`-external public fns — `mindflow`'s lib is not imported by integration tests, so use the functions via the crate. NOTE: integration tests compile against the crate `handy_app_lib`. Import `handy_app_lib::replace::replacements::{apply_replacements, Replacement}`. If `replace` or `replacements` is not `pub` at crate root, expose it minimally (see Step 1).

- [ ] **Step 1: Ensure the replacement API is reachable from the integration test.** Check visibility:

```bash
cd app/src-tauri
grep -nE "pub mod replace|mod replace" src/lib.rs
grep -nE "pub mod replacements|pub fn apply_replacements|pub struct Replacement" src/replace/mod.rs src/replace/replacements.rs
```
If `src/lib.rs` declares `mod replace;` (private), change it to `pub mod replace;`. If `src/replace/mod.rs` declares `mod replacements;`, change it to `pub mod replacements;`. Make the minimum change so `handy_app_lib::replace::replacements::{apply_replacements, Replacement}` resolves. (Adding `pub` is non-breaking.)

- [ ] **Step 2: Add the offline format/dictionary assertions** to `transcribes_offline_on_cpu`, immediately after the existing `assert!(found, …)` block and before the final `eprintln!("PASS …")`:

```rust
    // ── DoD: dictionary/replacement applies offline ──────────────────────────
    // Prove the personalization layer (M4) runs with no network: feed the real
    // transcript through apply_replacements and assert the substitution lands.
    use handy_app_lib::replace::replacements::{apply_replacements, Replacement};

    // Pick a word we just asserted is present so the rule is guaranteed to fire.
    let rule_from = if lower.contains("country") {
        "country"
    } else if lower.contains("americans") {
        "americans"
    } else {
        "ask"
    };
    let rules = vec![Replacement {
        from: rule_from.to_string(),
        to: "MINDFLOW_OK".to_string(),
    }];
    let replaced = apply_replacements(&text, &rules);
    eprintln!("After replacement ({rule_from} -> MINDFLOW_OK): {replaced}");
    assert!(
        replaced.contains("MINDFLOW_OK"),
        "offline dictionary/replacement must apply, got: {replaced:?}"
    );
```

- [ ] **Step 3: Update the test's module doc** (top of `zero_network.rs`) — add a line under "# What it checks": "Also runs the offline dictionary/replacement pass on the transcript and asserts the substitution applies (no network)."

- [ ] **Step 4: Run with the model present** (if available locally) and without:

```bash
cd app/src-tauri
# without model -> skips cleanly:
cargo test --test zero_network -- --nocapture
# with model (if you have it) -> full gate incl. replacement:
# MINDFLOW_TEST_MODEL=/path/to/moonshine-base cargo test --test zero_network -- --nocapture
```
Expected without model: prints the skip message, test passes. Expected with model: prints the transcript + "After replacement …" and passes.

- [ ] **Step 5: Confirm the crate still builds** (the `pub` visibility change):

```bash
cargo test --no-run 2>&1 | tail -5
```
Expected: compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/tests/zero_network.rs app/src-tauri/src/lib.rs app/src-tauri/src/replace/mod.rs
git commit -m "test(m6): extend offline gate to cover dictionary/replacement"
```

---

### Task 4: DoD self-check + checklist doc

**Files:**
- Create: `docs/superpowers/checklists/m6-definition-of-done.md`

**Interfaces:**
- Consumes: the DoD (master design §4), the tests from Tasks 2–3, the runbook from Task 5 (reference by filename).

- [ ] **Step 1: Verify dictionary + snippet unit coverage exists.** Run:

```bash
cd app/src-tauri
cargo test replacements 2>&1 | tail -15
grep -rnE "#\[test\]" src/replace/replacements.rs src/format/spoken_commands.rs | head
```
Record which DoD behaviors already have unit tests. If "dictionary word takes effect" or "snippet/replacement takes effect" has NO unit test, add a minimal one to `src/replace/replacements.rs`'s `#[cfg(test)] mod tests` asserting `apply_replacements("hello world", &[Replacement{from:"world".into(),to:"there".into()}]) == "hello there"`, run `cargo test replacements`, and note it in the checklist.

- [ ] **Step 2: Write the checklist** mapping each DoD item to its verification, as a table with columns `DoD item | How verified | Evidence (test / runbook step)`:
  1. Install + onboard + model auto-pick ≥ real-time → runbook §"Install & onboard" + `recommended_tier_cmd` (code ref `commands`/`stt_tier.rs`).
  2. Hotkey → punctuated/capitalized/filler-free, offline, ~1–2s → `tests/zero_network.rs` (offline transcription) + runbook "dictate in airplane mode".
  3. Dictionary word + snippet take effect → unit tests in `replace/replacements.rs` (named) + `tests/zero_network.rs` replacement assertion + runbook step.
  4. Zero network after download → `tests/no_network_in_hot_path.rs` + audit doc + runbook "airplane mode dictation".

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/checklists/m6-definition-of-done.md app/src-tauri/src/replace/replacements.rs
git commit -m "docs(m6): definition-of-done checklist + any missing unit coverage"
```

---

### Task 5: 3-OS install runbook doc

**Files:**
- Create: `docs/superpowers/checklists/m6-3os-install-runbook.md`

- [ ] **Step 1: Write the runbook** — a per-OS (Windows / macOS / Linux) numbered procedure, each step a `- [ ]` checkbox, covering:
  1. Build/install the bundle (`bun run tauri build`; note the artifact path per OS).
  2. First launch → onboarding completes; a model is auto-recommended and downloads.
  3. **Cut the network** (airplane mode / disable adapter).
  4. Dictate into a real third-party app (e.g., a browser address bar or notes app); confirm punctuated, capitalized output appears within ~1–2s.
  5. Add a dictionary word + a snippet in Advanced settings; dictate to confirm both apply.
  6. Exercise hands-free mode (set Recording mode = Hands-free; tap hotkey, speak, press Enter → text + transcribe).
  7. Confirm the tray icon renders (gold brain) and the recording overlay shows the gold "Flow Mark island".
  8. Re-enable network only to confirm the optional update check works (and that turning `update_checks_enabled` off stops it).
  - Each OS section ends with a pass/fail signoff line and a notes field.

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/checklists/m6-3os-install-runbook.md
git commit -m "docs(m6): 3-OS manual install/airplane-mode runbook"
```

---

### Task 6: v1 release-gate checklist doc

**Files:**
- Create: `docs/superpowers/checklists/m6-v1-release-gate.md`

- [ ] **Step 1: Write the release-gate checklist** as ordered `- [ ]` items:
  1. `cd app && bun run build` + `bun run lint` green.
  2. `cd app/src-tauri && cargo test` green (incl. `no_network_in_hot_path`).
  3. `tests/zero_network.rs` run once with a real `MINDFLOW_TEST_MODEL` — passes (maintainer).
  4. 3-OS install runbook (`m6-3os-install-runbook.md`) fully signed off.
  5. Zero-network audit (`2026-06-27-m6-zero-network-audit.md`) reviewed, no open items.
  6. DoD checklist (`m6-definition-of-done.md`) all items checked.
  7. Versions bumped + consistent across `app/src-tauri/tauri.conf.json`, `app/src-tauri/Cargo.toml`, `app/package.json`.
  8. Updater `pubkey` present and `endpoints` point at the MindFlow releases (sanity, not changed).
  9. Per-OS build artifacts produced and launch-tested (from runbook).
  10. Tag `v1.0.0` and publish.

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/checklists/m6-v1-release-gate.md
git commit -m "docs(m6): v1.0 release-gate checklist"
```

---

## Self-Review

**Spec coverage:** §4.1 audit → Task 1; §4.2 guard test → Task 2; §4.3 offline-gate extension → Task 3; §4.4 DoD checklist → Task 4; §4.5 3-OS runbook → Task 5; §4.6 release checklist → Task 6. §6 self-verification ("guard test module list == audit module list") is enforced by Task 1's Interfaces note and Task 2's `HOT_PATH` comment. No gaps.

**Placeholder scan:** No `TBD`/`TODO`; every code step shows full code; every doc task enumerates its exact sections. Clean.

**Type consistency:** `apply_replacements(text: &str, rules: &[Replacement]) -> String` and `Replacement { from, to }` used identically in Tasks 3 and 4. `HOT_PATH` list in Task 2 matches the hot-path module list named in Tasks 1 and 4. `MINDFLOW_TEST_MODEL` env var consistent with the existing test.
