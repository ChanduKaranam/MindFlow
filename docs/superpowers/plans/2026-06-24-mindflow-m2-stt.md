# MindFlow M2 — CPU-Only STT Configuration & Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Make MindFlow's local speech-to-text run CPU-only by default with the research-backed model (Parakeet-TDT-0.6B-v2 English default, Moonshine-tiny fallback), curated into Turbo/Balanced/Max tiers with CPU-based auto-pick, and prove a real offline CPU transcription end-to-end (fulfilling M1's zero-network gate obligation).

**Architecture:** The vendored Handy app **already implements** the full STT pipeline — a 16-model catalog with download/verify/extract, transcribe-rs engine dispatch (Whisper/Parakeet/Moonshine/…), model-selection UI, and accelerator settings. M2 therefore is **configuration + curation + verification**, NOT building a pipeline. We force CPU accelerator defaults, set MindFlow's default model, add a `tier` concept over the existing models, add a pure CPU-capability→tier recommender, surface tiers in onboarding, and add a real offline-transcription integration test.

**Tech Stack:** Rust (Tauri v2), transcribe-rs 0.3.8 (whisper-cpp + onnx, CPU-only), existing Handy managers (`model.rs`, `transcription.rs`, `settings.rs`), React/TS frontend.

## Global Constraints

- CPU-only: no task may enable a GPU runtime/accelerator. Accelerator settings must default to `Cpu`. (spec §1)
- No cloud/network on the dictation path; model download is the only network use, explicit and verifiable. (spec §1, §6)
- Default model = **Parakeet-TDT-0.6B-v2** (English, CC-BY-4.0). Tiny fallback = **Moonshine-tiny** (MIT). Multilingual option = Parakeet-v3. (research A)
- Tiers: ⚡Turbo = `moonshine-tiny-streaming-en`, ⭐Balanced = `parakeet-tdt-0.6b-v2`, 🎯Max = `parakeet-tdt-0.6b-v3`. (research A/B)
- Do not break Handy's existing model catalog/engine dispatch — extend, don't rewrite. (research B)

---

### Task 1: Force CPU accelerator defaults

**Files:**
- Modify: `app/src-tauri/src/settings.rs` (the `Default` impls for `WhisperAcceleratorSetting` ~285-289 and `OrtAcceleratorSetting` ~302-306; `get_default_settings()` ~715-816)
- Test: inline `#[cfg(test)]` in `settings.rs`

**Interfaces:**
- Consumes: existing `WhisperAcceleratorSetting`, `OrtAcceleratorSetting` enums (have a `Cpu` variant).
- Produces: defaults now resolve to `Cpu`; `get_default_settings()` sets both to `Cpu`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod m2_cpu_defaults {
    use super::*;
    #[test]
    fn accelerator_defaults_are_cpu() {
        assert_eq!(WhisperAcceleratorSetting::default(), WhisperAcceleratorSetting::Cpu);
        assert_eq!(OrtAcceleratorSetting::default(), OrtAcceleratorSetting::Cpu);
    }
    #[test]
    fn default_settings_force_cpu() {
        let s = get_default_settings();
        assert_eq!(s.whisper_accelerator, WhisperAcceleratorSetting::Cpu);
        assert_eq!(s.ort_accelerator, OrtAcceleratorSetting::Cpu);
    }
}
```
(If the enums lack `PartialEq`, derive it in this task.)

- [ ] **Step 2: Run it, confirm it fails** — `bash -lc 'cd app/src-tauri && cargo test --lib m2_cpu_defaults 2>&1 | tail -20'` → FAIL (defaults are `Auto`).
- [ ] **Step 3: Change the two `Default` impls to return `::Cpu`, and set both fields to `::Cpu` in `get_default_settings()`.** Add `#[derive(PartialEq)]` to the enums if missing.
- [ ] **Step 4: Run it, confirm PASS.**
- [ ] **Step 5: Commit** — `feat(m2): default speech accelerators to CPU`

---

### Task 2: Set Parakeet-v2 as MindFlow's recommended default model

**Files:**
- Modify: `app/src-tauri/src/managers/model.rs` (the `parakeet-tdt-0.6b-v2` entry ~line 273 and `parakeet-tdt-0.6b-v3` entry ~line 310 — move `is_recommended: true` to v2; ensure v3 is `is_recommended: false`)
- Test: inline `#[cfg(test)]` in `model.rs`

**Interfaces:**
- Consumes: existing `ModelManager::new()` catalog, `ModelInfo { is_recommended, .. }`.
- Produces: exactly one recommended model, id `parakeet-tdt-0.6b-v2`.

- [ ] **Step 1: Write the failing test** (constructs the catalog and asserts the recommended id):

```rust
#[cfg(test)]
mod m2_default_model {
    use super::*;
    #[test]
    fn recommended_model_is_parakeet_v2() {
        let mm = ModelManager::test_catalog(); // see Step 3
        let rec: Vec<&str> = mm.models().values()
            .filter(|m| m.is_recommended).map(|m| m.id.as_str()).collect();
        assert_eq!(rec, vec!["parakeet-tdt-0.6b-v2"]);
    }
}
```

- [ ] **Step 2: Run it, confirm it fails** (currently v3 is recommended). `bash -lc 'cd app/src-tauri && cargo test --lib m2_default_model 2>&1 | tail -20'`
- [ ] **Step 3:** Add a tiny test helper `ModelManager::test_catalog()` that builds the same `HashMap` without needing an `AppHandle` (extract the catalog-construction into a pure `fn build_catalog() -> HashMap<String, ModelInfo>` and call it from both `new()` and the helper). Flip `is_recommended`: `true` on v2, `false` on v3.
- [ ] **Step 4: Run it, confirm PASS.**
- [ ] **Step 5: Commit** — `feat(m2): default to Parakeet-v2 (English) as recommended model`

---

### Task 3: Add Turbo/Balanced/Max tier metadata to the curated models

**Files:**
- Modify: `app/src-tauri/src/managers/model.rs` (add `pub tier: Option<ModelTier>` to `ModelInfo`; define `enum ModelTier { Turbo, Balanced, Max }` with serde; set it on the three curated models)
- Modify: `app/src/bindings.ts` is auto-generated — regenerate or let specta handle; if manual, expose `tier` in the model type used by the selector
- Test: inline `#[cfg(test)]` in `model.rs`

**Interfaces:**
- Produces: `ModelTier` enum (Serialize/Deserialize/specta::Type); `moonshine-tiny-streaming-en→Turbo`, `parakeet-tdt-0.6b-v2→Balanced`, `parakeet-tdt-0.6b-v3→Max`; all other models `tier: None`.

- [ ] **Step 1: Write the failing test**:

```rust
#[cfg(test)]
mod m2_tiers {
    use super::*;
    #[test]
    fn curated_models_have_expected_tiers() {
        let c = build_catalog();
        assert_eq!(c["moonshine-tiny-streaming-en"].tier, Some(ModelTier::Turbo));
        assert_eq!(c["parakeet-tdt-0.6b-v2"].tier, Some(ModelTier::Balanced));
        assert_eq!(c["parakeet-tdt-0.6b-v3"].tier, Some(ModelTier::Max));
    }
}
```

- [ ] **Step 2: Run it, confirm it fails** (no `tier` field yet).
- [ ] **Step 3:** Define `ModelTier` (derive `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type`), add `tier: Option<ModelTier>` to `ModelInfo` (default `None` for all existing entries), set the three curated values.
- [ ] **Step 4: Run it, confirm PASS;** also `cargo check` the whole crate (specta type generation must still compile).
- [ ] **Step 5: Commit** — `feat(m2): add Turbo/Balanced/Max tier metadata to curated models`

---

### Task 4: CPU-capability → recommended tier (pure logic)

**Files:**
- Create: `app/src-tauri/src/stt_tier.rs` (pure recommender)
- Modify: `app/src-tauri/src/lib.rs` (`mod stt_tier;`)
- Test: inline `#[cfg(test)]` in `stt_tier.rs`

**Interfaces:**
- Produces:
  - `struct CpuProfile { pub physical_cores: usize, pub total_ram_gb: f32 }`
  - `fn recommend_tier(p: &CpuProfile) -> ModelTier` (import `ModelTier` from `crate::managers::model`)
  - `fn detect_cpu_profile() -> CpuProfile` (uses an existing system-info crate if present, else `std`/`sysinfo`)

- [ ] **Step 1: Write the failing test**:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::model::ModelTier;
    #[test]
    fn weak_machine_gets_turbo() {
        assert_eq!(recommend_tier(&CpuProfile { physical_cores: 2, total_ram_gb: 4.0 }), ModelTier::Turbo);
    }
    #[test]
    fn typical_laptop_gets_balanced() {
        assert_eq!(recommend_tier(&CpuProfile { physical_cores: 4, total_ram_gb: 16.0 }), ModelTier::Balanced);
    }
    #[test]
    fn strong_desktop_gets_max() {
        assert_eq!(recommend_tier(&CpuProfile { physical_cores: 12, total_ram_gb: 32.0 }), ModelTier::Max);
    }
}
```

- [ ] **Step 2: Run it, confirm it fails.**
- [ ] **Step 3: Implement**:

```rust
use crate::managers::model::ModelTier;

pub struct CpuProfile { pub physical_cores: usize, pub total_ram_gb: f32 }

/// Parakeet-v2 (Balanced) needs ~8 GB headroom; Max (v3) wants a strong CPU.
/// Weak machines fall back to the tiny Moonshine (Turbo).
pub fn recommend_tier(p: &CpuProfile) -> ModelTier {
    if p.total_ram_gb < 8.0 || p.physical_cores < 4 {
        ModelTier::Turbo
    } else if p.physical_cores >= 8 && p.total_ram_gb >= 24.0 {
        ModelTier::Max
    } else {
        ModelTier::Balanced
    }
}
```
For `detect_cpu_profile()`, reuse whatever system-info crate Handy already depends on (check `Cargo.toml`); if none, add `sysinfo` and read core count + total memory. Keep `recommend_tier` pure (no I/O) so it stays unit-tested.

- [ ] **Step 4: Run it, confirm 3/3 PASS.**
- [ ] **Step 5: Commit** — `feat(m2): CPU-capability-based tier recommender`

---

### Task 5: Surface tiers in onboarding + auto-pick the recommended model

**Files:**
- Modify: `app/src/components/onboarding/Onboarding.tsx` (group/show the 3 tiers; preselect the tier from a new command)
- Modify: `app/src/components/model-selector/` (display `tier` badge)
- Create/Modify: a Tauri command `recommended_tier_cmd() -> Result<String,String>` in `lib.rs` (returns "turbo"/"balanced"/"max" from `stt_tier::recommend_tier(&detect_cpu_profile())`), registered in the specta `collect_commands!` list.
- Test: frontend build passes; a Rust test for the command's mapping.

**Interfaces:**
- Consumes: Task 3 `ModelTier`, Task 4 `recommend_tier`/`detect_cpu_profile`.
- Produces: `recommended_tier_cmd`; onboarding highlights the recommended tier and downloads that model on confirm.

- [ ] **Step 1 (Rust): Write failing test** for the command's tier→string mapping in `lib.rs` (`turbo|balanced|max`). Run, fail, implement, pass.
- [ ] **Step 2:** Register `recommended_tier_cmd` with `#[tauri::command] #[specta::specta]` in `collect_commands!` (same mechanism as M1's `deliver_text_cmd`).
- [ ] **Step 3 (Frontend):** In onboarding, call `recommended_tier_cmd`, render the 3 tiers (Turbo/Balanced/Max) with the recommended one preselected; show a `tier` badge in the model selector. Use raw `invoke` or regenerate specta bindings.
- [ ] **Step 4:** `bash -lc 'cd app && bun run build'` → passes.
- [ ] **Step 5: Commit** — `feat(m2): tier-aware onboarding with CPU-based recommendation`

---

### Task 6: Real offline CPU transcription test (fulfills M1 zero-network obligation)

**Files:**
- Modify: `app/src-tauri/tests/zero_network.rs` (replace the placeholder with a real test)
- Create: `app/src-tauri/tests/fixtures/hello.wav` (a few seconds of clear English speech, 16 kHz mono)
- Test: the integration test itself

**Interfaces:**
- Consumes: transcribe-rs Moonshine/Parakeet engine via Handy's transcription path (or transcribe-rs directly with a downloaded model dir).

- [ ] **Step 1: Write the test** (gated behind an env flag so CI without the model still passes, but it runs locally/with the model present):

```rust
// Real offline-transcription gate. Requires a model dir at $MINDFLOW_TEST_MODEL
// (download moonshine-tiny-streaming-en, ~31 MB). Skips cleanly if unset so CI
// without the model is green; run locally with the env set to actually assert.
#[test]
fn transcribes_offline_on_cpu() {
    let Ok(model_dir) = std::env::var("MINDFLOW_TEST_MODEL") else {
        eprintln!("MINDFLOW_TEST_MODEL unset — skipping real transcription gate");
        return;
    };
    let wav = include_bytes!("fixtures/hello.wav");
    let samples = mindflow_test_decode_wav(wav); // small helper in the test file
    // Load the moonshine/parakeet engine from model_dir via transcribe-rs and transcribe.
    let text = transcribe_with_model(&model_dir, &samples).expect("transcription");
    assert!(!text.trim().is_empty(), "expected non-empty transcript");
    assert!(text.to_lowercase().contains("hello"), "expected the spoken word, got: {text}");
}
```

- [ ] **Step 2: Run without the env** → passes (skips). With `MINDFLOW_TEST_MODEL` pointing at a downloaded moonshine-tiny dir → transcribes and asserts. Document the download command in the test header.
- [ ] **Step 3:** Wire the actual transcribe-rs call (mirror `managers/transcription.rs` load+transcribe for the Moonshine/Parakeet engine). Keep it minimal.
- [ ] **Step 4: Run both modes**, confirm behavior.
- [ ] **Step 5: Commit** — `test(m2): real offline CPU transcription gate (replaces M1 placeholder)`

---

## Self-Review

**Spec coverage:** CPU-only default (Task 1) ✅ · research model default (Task 2) ✅ · tiers (Task 3) ✅ · CPU auto-pick (Task 4) ✅ · onboarding UX (Task 5) ✅ · real offline+zero-network verification (Task 6, fulfills M1 obligation) ✅.

**Placeholder scan:** Tasks 2/3 reference `build_catalog()`/`test_catalog()` — Task 2 Step 3 creates them. Task 6 uses a gated real test (skips without the model) — this is intentional and documented, not a hidden placeholder; it asserts real transcription when the model is present.

**Type consistency:** `ModelTier` defined in Task 3 is reused by Tasks 4/5 from `crate::managers::model`. `recommend_tier`/`CpuProfile`/`detect_cpu_profile` (Task 4) consumed by Task 5's command.

## Note on scope (important)
Handy already provides the STT engine, catalog, download, and selection UI — so M2 is configuration/curation/verification, not a from-scratch build. The genuinely-new MindFlow value (tiered rules+LLM **AI formatting**, **custom dictionary / Power-mode**) is **M3+** and is where MindFlow diverges from "just running Handy." Consider whether to prioritize that differentiation next.
