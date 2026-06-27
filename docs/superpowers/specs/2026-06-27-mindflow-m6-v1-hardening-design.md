# MindFlow M6 — v1 Hardening / Release Gate — Design Spec

**Milestone:** M6 — the final v1 milestone (release gate).
**Date:** 2026-06-27
**Status:** approved (design), pending spec review.

## 1. Goal

Prove and *lock in* MindFlow's core promise — **fully local, zero-network, CPU-only dictation** — and produce the checklist that gates a v1.0 release. M6 builds almost no new product surface; it is verification, a regression test that keeps the promise true over time, and release documentation.

## 2. Context

M0–M5 (+ hands-free, denoise, VAD, brand) are merged to `main`. The Definition of Done from the master design (`docs/superpowers/specs/2026-06-24-mindflow-design.md` §4) is:

> On a clean Win/macOS/Linux laptop with no GPU, a user can: (1) install, onboard, and have a model auto-selected that runs ≥ real-time; (2) hold the hotkey in any app, speak, release → punctuated, capitalized, filler-free text within ~1–2s, fully offline; (3) add a dictionary word and a snippet, both take effect; (4) all of the above with **zero network traffic** after model download.

An offline transcription gate already exists at `app/src-tauri/tests/zero_network.rs` (loads a local model, transcribes the JFK clip, asserts output; skips in CI when `MINDFLOW_TEST_MODEL` is unset). M6 extends it and adds an always-on regression guard.

### Network surface (audited)

The codebase has exactly three outbound-network sites:

| Site | Module | Trigger | v1 disposition |
|---|---|---|---|
| Model download | `managers/model.rs`, `commands/models.rs` | Explicit user action / first-run model pick | Expected, one-time, before offline use |
| App update check | tauri updater (`tauri.conf.json` endpoint) | Optional; gated by `update_checks_enabled` | Allowed opt-in; off-able |
| Post-processing LLM | `llm_client.rs` | `post_process_enabled` (off by default) → user-configured cloud provider | **Opt-in cloud, off by default; deferred to v2 (Tier-2 LLM spec).** M6 verifies + flags only. |

**Core dictation** (capture → VAD → STT → format → inject → output) touches **none** of these. That is the property M6 proves and guards.

## 3. Decisions

1. **Scope = automate what's automatable + a manual 3-OS runbook for the maintainer.** The CI environment is Linux/headless and cannot install/run the bundled app on real macOS/Windows; those steps are a human runbook.
2. **Cloud post-processing is a v1 opt-in (off by default), not removed.** M6 documents it as the sole intentional cloud touchpoint and verifies it is off by default; the full design lands in the v2 Tier-2 LLM spec. No UI removal in M6.
3. **The "zero-network" guarantee is scoped to core dictation**, not to opt-in features (model download, optional update check, opt-in post-processing).
4. **Streaming live partials and Tier-2 LLM remain v2** (separate specs), out of M6.

## 4. Components

Each is independently reviewable.

### 4.1 Zero-network code audit *(doc)*
`docs/superpowers/audits/2026-06-27-m6-zero-network-audit.md`. Enumerates every outbound-network call site (the table above, expanded with file:line and trigger), and walks the dictation hot-path module-by-module asserting no network dependency. Concludes with the explicit guarantee statement and the two opt-in exceptions. This is the human-readable proof.

### 4.2 Network-symbol guard test *(new code — the regression gate)*
`app/src-tauri/tests/no_network_in_hot_path.rs`. A source-level test that reads the hot-path module sources and asserts they contain **no** references to network crates/APIs (`reqwest`, `hyper`, `ureq`, `TcpStream`, `reqwest::Client`, raw `http://`/`https://` request URLs). 

- **Hot-path modules covered:** `audio_toolkit/`, `managers/transcription.rs`, `managers/audio.rs`, `format/`, `inject/` (or `signal_handle.rs`/`actions.rs` paste path), `shortcut/`, `transcription_coordinator.rs`.
- **Explicitly excluded (known opt-in network):** `llm_client.rs`, `managers/model.rs`, `commands/models.rs`, updater config.
- Runs in CI with no model and no network; **fails the build** if a future change introduces a network call into the local path. This is the durable guarantee.
- Implementation: read each module's `.rs` files from `CARGO_MANIFEST_DIR`, strip comments/strings minimally, scan for the forbidden symbols, assert none. Deterministic, fast, no deps.

### 4.3 Extend the offline gate *(code)*
Broaden `zero_network.rs` (when `MINDFLOW_TEST_MODEL` is set) so it also runs the **formatting + dictionary/replacement** pass on the transcript offline — asserting the DoD's "punctuated/filler-free + dictionary applied" claim, not just raw STT. Uses the existing `format`/`replace` functions directly on the transcript with a tiny in-test dictionary/replacement, asserting the substitution takes effect. Still gated/skips without the model.

### 4.4 DoD self-check + checklist *(doc)*
`docs/superpowers/checklists/m6-definition-of-done.md`. Each of the 4 DoD items broken into concrete checks, each mapped to its verification: an automated test (named), a code reference, or a runbook step (§4.5). Any gap found during the audit (e.g., a DoD claim with no test) is fixed as part of M6 and the checklist updated. Includes the dictionary + snippet "take effect" checks (verify existing unit coverage; add if missing).

### 4.5 3-OS install runbook *(doc for the maintainer)*
`docs/superpowers/checklists/m6-3os-install-runbook.md`. Per-OS (Win/macOS/Linux) numbered procedure with pass/fail checkboxes: build/install the bundle, launch, complete onboarding, download a model, **enable airplane mode / pull the network**, dictate into a real third-party app, confirm punctuated output, add a dictionary word + snippet and confirm both apply, exercise hands-free (Enter-to-stop), confirm the tray icon + gold overlay render, confirm model auto-pick ran ≥ real-time. The maintainer executes this on real hardware.

### 4.6 Release checklist *(doc)*
`docs/superpowers/checklists/m6-v1-release-gate.md`. The final gate before tagging v1.0: all `cargo test` + `bun run build`/`lint` green; guard test green; offline gate run once with a real model (maintainer); 3-OS runbook signed off; audit clean; version bumped (`tauri.conf.json`, `Cargo.toml`, `package.json`) and consistent; updater pubkey/endpoint sane; build artifacts produced per OS. Checkbox gate.

## 5. Out of scope (v1 / M6)
- Running the 3-OS manual install tests (maintainer does, on real hardware).
- Full cloud post-processing / Tier-2 LLM design → **v2 Tier-2 LLM spec**.
- Streaming sub-400ms live partials → **v2 streaming spec**.
- Command Mode, Power Mode/context-awareness → **v2 specs**.
- Any new product feature. M6 is hardening only.

## 6. Testing / acceptance
- New `no_network_in_hot_path` guard test passes in CI (no model/network needed) and fails if a network symbol is introduced into a hot-path module (verify by a temporary local edit during development, then revert).
- Extended `zero_network.rs` passes with `MINDFLOW_TEST_MODEL` set (formatting + dictionary applied offline) and still skips cleanly without it.
- `cargo test`, `bun run build`, `bun run lint` all green.
- The four docs (audit, DoD checklist, 3-OS runbook, release checklist) are complete — no placeholders.
- Self-verification: the guard test's hot-path module list matches the audit's hot-path module list (single source of truth, no drift).

## 7. Decomposition (for the plan)
1. Zero-network code audit doc (§4.1) — read each network site + hot-path module, write the audit.
2. Network-symbol guard test (§4.2).
3. Extend offline gate with formatting + dictionary (§4.3).
4. DoD self-check + checklist; fix any gap found (§4.4).
5. 3-OS install runbook (§4.5).
6. v1 release-gate checklist (§4.6).

Order: audit (1) first — it discovers the exact hot-path module set and any gaps that 2–4 depend on. Then guard test (2) + offline-gate extension (3) (code). Then the three checklists (4–6) which consume the audit's findings.
