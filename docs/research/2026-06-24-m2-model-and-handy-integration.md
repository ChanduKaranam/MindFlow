# M2 Research — Best Small CPU STT Model + Handy's Existing Model Integration

*Compiled 2026-06-24. Two findings: (A) the model choice, (B) the big one — Handy already has the whole STT pipeline.*

## A) Model choice (smallest + least resources + best results)

Sorted by the size↔resource↔accuracy frontier (all CPU; Open ASR RTFx is GPU, used for WER only):

| Model | Disk (int8) | ~RAM | CPU RTF | WER | Lang | Format/Runtime | License |
|---|---|---|---|---|---|---|---|
| Moonshine tiny en | ~125 MB (Handy pkg 31 MB streaming-tiny) | ~250-400 MB | ~5× faster than whisper-tiny | ~12% | en | ONNX / transcribe-rs, sherpa | MIT |
| Whisper base.en GGML q5 | ~60 MB | ~150 MB | 0.2-0.5 | ~10% class | en | GGML / whisper-rs | MIT |
| Moonshine base | ~55-190 MB | ~300-500 MB | ~5× faster than whisper-base | ~10% | en | ONNX | MIT |
| SenseVoice-Small | ~152-229 MB | ~400-700 MB | RTF ~0.007 (claim) | strong CJK+en | zh/en/yue/ja/ko | ONNX | verify weights |
| **Parakeet-TDT-0.6B-v2 (DEFAULT)** | **~451-631 MB** | ~1.5-2.5 GB | **~0.03-0.05 (20-30× RT)** | **6.05% (SOTA-class)** | **en** | ONNX / transcribe-rs, sherpa | CC-BY-4.0 |
| Parakeet-TDT-0.6B-v3 | ~456-640 MB | ~1.5-2.5 GB | ~same | 6.34% | 25 EU langs | ONNX | CC-BY-4.0 |

**Recommendation (confirmed):**
- **Default = Parakeet-TDT-0.6B-v2 (English)** — SOTA accuracy (6.05% WER) *and* 20-30× real-time on CPU int8. The frontier is discontinuous: nothing between ~190 MB and ~630 MB matches it. Needs ≥8 GB RAM.
- **Tiny fallback = Moonshine tiny en (~125 MB, MIT)** — for 4 GB / weak CPUs; stays in the same ONNX runtime.
- **Multilingual option = Parakeet v3** (25 EU langs).
- Licenses all commercial-safe & ungated (CC-BY-4.0 / MIT). Download-on-demand, not bundled.

## B) THE BIG FINDING — Handy already implements the entire STT pipeline

Exploring `app/src-tauri` revealed Handy is a **complete, working local dictation app**, not just a shell. It already has everything M2-M5 of our plan intended to build:

- **Model catalog (16 models)** in `managers/model.rs` (lines 126-611): whisper small/medium/turbo/large, **parakeet-tdt-0.6b-v2 (451 MB) & v3 (456 MB, already `is_recommended: true` DEFAULT)**, moonshine tiny-streaming (31 MB)/base/small/medium, SenseVoice, GigaAM, Canary 180m/1b, Cohere. All with download URLs (`blob.handy.computer`), sizes, SHA256.
- **Download/storage/verify flow** (`managers/model.rs` 987-1325): resumable download, SHA256 verify, tar.gz extract, app-data-dir storage, auto-select first downloaded.
- **transcribe-rs engine dispatch** (`managers/transcription.rs` 253-733): loads Whisper/Parakeet/Moonshine/SenseVoice/etc by `EngineType`, transcribes with per-engine params, post-processes (custom words, filler removal).
- **Model selection UI** (`src/components/model-selector/`) + settings (`selected_model`, `settings.rs`).
- **Accelerator settings** (`whisper_accelerator`, `ort_accelerator`, `whisper_gpu_device`) defaulting to **Auto** — `commands/...` + `shortcut/mod.rs` 1108-1165.
- **Full pipeline already wired**: hotkey → audio → VAD → transcribe → paste.

### What this means for M2 (re-scope)
M2 is **NOT** "build the STT pipeline" — Handy already has it. M2 becomes **configuration + curation + CPU-only enforcement + verification**:
1. **Force CPU accelerators**: `settings.rs` change `WhisperAcceleratorSetting`/`OrtAcceleratorSetting` defaults Auto→Cpu + `get_default_settings()` (lines ~285-306, 715-816). Critical now that we're CPU-only.
2. **Set MindFlow's default model**: research says **Parakeet v2 (English)**; Handy currently defaults to v3. One-line `is_recommended`/default change in `model.rs`.
3. **Curate tiers** from existing models: ⚡Turbo = moonshine-tiny-streaming (31 MB); ⭐Balanced = parakeet-v2; 🎯Max = parakeet-v3 or whisper-large-turbo. Either add a `tier` field to `ModelInfo` or trim/group the catalog + onboarding.
4. **CPU-based auto-pick** on first run (optional): recommend tier by detected RAM/cores.
5. **Verify** CPU transcription works end-to-end offline (manual on a real desktop; the vendored pipeline + our CPU-only build).

### Files to touch (from the exploration)
- `app/src-tauri/src/settings.rs` — accelerator defaults → Cpu.
- `app/src-tauri/src/managers/model.rs` — default model, optional `tier` field / catalog curation.
- `app/src/components/onboarding/Onboarding.tsx` + `model-selector/` — tier display, auto-download default.
- `app/src-tauri/src/managers/transcription.rs` — `apply_accelerator_settings` already correct (no change).

### Honest implication
Handy-as-vendored is already ~80% of our v1. MindFlow's *real* differentiation (vs just running Handy) is the **AI-formatting layer (Whispering-style) and dictionary/Power-mode (VoiceInk-style)** — which are later milestones. Our M1 injection module also partly duplicated Handy's existing paste pipeline. M2 should therefore be a small config/curation milestone, and we should re-scope M3+ around the genuinely-new formatting/personalization work.
