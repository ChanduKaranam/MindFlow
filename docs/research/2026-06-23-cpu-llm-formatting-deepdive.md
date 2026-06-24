# CPU Local LLMs for Transcript Formatting Deep Dive (2026)

*Scope: the AI-formatting layer — remove fillers, fix punctuation/caps, format lists, light tone, optional instruction rewrite. CPU-only. CPU vs GPU flagged.*

## Methodology warning
Most "small LLM benchmark" tok/s online are **GPU/Metal mislabeled as CPU** (AscentCore, llmcheck.net, official Qwen benchmarks are all GPU). CPU generation is memory-bandwidth-bound. Trustworthy pure-CPU data is scarce; Qwen/Gemma CPU rows are extrapolated.

## CPU tok/s anchor (Llama 3.2 3B, Q4, CPU-only, geerlingguy/ai-benchmarks)
88 (Ryzen AI Max+395) · 23.8 (Framework 13 Ryzen AI 5) · 13.1 (older Framework) · 9.1 (Intel N150) · 4.9 (Pi 5).
**Rule of thumb on a no-GPU laptop:** 3B ≈ 10-25 tok/s, 1-2B ≈ 20-50 tok/s, 7-8B ≈ 8-14 tok/s. Prompt processing ~10-20× faster than generation.

## Key models

| Model | Params | Q4_K_M size | License | CPU tok/s (mid laptop) | IFEval | Context | GGUF |
|---|---|---|---|---|---|---|---|
| Qwen2.5-0.5B / Qwen3-0.6B | 0.5-0.6B | ~490-520 MB | Apache-2.0 ✅ | ~40-55 (est) | weak/unreliable | 32K | Yes |
| **Llama 3.2 1B** | 1B | 808 MB | Llama 3.2 ⚠️ gated | 20-50 | 53-59 | 128K | Yes |
| **SmolLM2-1.7B** | 1.7B | 1.06 GB | Apache-2.0 ✅ | ~15-30 | **56.7** | **8K only** ⚠️ | Yes |
| **HyprLLM-sm** (Qwen3-1.7B FT) | 1.7B | Q4_K_M | base Apache; FT unstated ⚠️ | ~18-30 | purpose-tuned for cleanup | 32K | Yes |
| **Llama 3.2 3B** | 3B | 2.02 GB | Llama 3.2 ⚠️ gated | **10-24** (best-attested) | **68-77** | 128K | Yes |
| Qwen2.5-3B | 3B | 2.1 GB | **NON-COMMERCIAL ⛔** | ~10-20 | 58 | 32K | Yes |
| **Phi-4-mini** | 3.8B | 2.5 GB | **MIT** ✅ | **12** (i7-12700) | **70.1** | 128K | Yes |
| **Qwen3-4B-Instruct-2507** | 4B | 2.5 GB | Apache-2.0 ✅ | ~8-18 | **83.4** (best) | 32K | Yes |
| Gemma 2 2B / Gemma 3 4B | 2.6-4B | 1.7-3.3 GB | Gemma ⚠️ gated | ~8-25 | 80-90 (Google metric) | 8K/128K | Yes |

## Latency reality check (~80-token cleanup on CPU)
**No general LLM reliably hits a <2s feel for 80 output tokens on a mainstream CPU.** Sub-1B approach it but follow edit instructions inconsistently (Qwen3-0.6B drops −61.8% reliability@10). 1.7B ≈ 2.7-4.5s, 3B ≈ 3.6-8s, 4B ≈ 5-10s.

## Dedicated punctuation model beats LLM for the narrow task
A small BERT-style model does punctuation + truecasing + filler removal in **~10-75ms CPU** and matches/beats GPT-4-class on it:
- **`1-800-BAD-CODE/punctuation_fullstop_truecase_english`** — 6-layer ONNX, punct+case+sentence-boundary in one, F1 ~97 punct / 99.5 case, CPU-friendly. **Recommended.**
- Disfluency removal (Google small-BERT): 1.3-12 MiB, 9-15ms, F1 88-90.
- `oliverguhr/fullstop` (XLM-R, MIT, multilingual punct, no casing).
- Fine-tuned BERT beat GPT-4 Turbo on SponSpeech punctuation restoration.
- Can't do: tone/rewrite/formatting — those need an LLM.

## Recommendations
- **Default LLM (Tier 2 rewrite):** **Llama 3.2 3B Q4_K_M** (~10-24 tok/s, IFEval 68-77) — or **Phi-4-mini Q4_K_M (MIT, clean license, IFEval 70.1)** for a permissive default.
- **Light fallback (weak CPU):** Llama 3.2 1B Q4 (~20-50 tok/s) or **SmolLM2-1.7B (Apache-2.0)**.
- **High quality (strong CPU):** **Qwen3-4B-Instruct-2507 Q4 (Apache-2.0, IFEval 83.4)**.
- **Purpose-built to A/B test:** HyprLLM-sm (trained on transcript cleanup; verify license).

## Architecture recommendation — TIERED, not pure-LLM
- **Tier 1 (always-on, ~10-80ms CPU):** regex/rules (whitespace, "i"→"I", strip exact fillers) + small ONNX punctuation/truecasing model + optional tiny disfluency BERT. Handles ~90% of dictation (clean/punctuate/capitalize) with a true real-time feel, no GPU.
- **Tier 2 (on-demand LLM, only on explicit user request):** "make concise", "bulletize", "make formal", translate. Accept 2-8s latency because it's occasional. Default Llama 3.2 3B / Phi-4-mini; high-quality Qwen3-4B; fallback 1B/SmolLM2.

This is what the best competitor (Hyprnote) does in spirit — purpose-trained small model + constrained generic LLM. It sidesteps sub-1B unreliability on deterministic edits.
