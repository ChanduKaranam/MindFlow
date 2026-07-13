//! In-process GGUF LLM inference via llama.cpp (CPU-only). One fresh context
//! per generate() call keeps the KV cache clean between dictations.
//!
//! `LlamaBackend::init()` returns `Err(BackendAlreadyInitialized)` if a backend
//! handle is already alive anywhere in the process (llama-cpp-2 tracks this with
//! a process-wide atomic flag, cleared again on `Drop`). Owning one `LlamaBackend`
//! per `LlmEngine` would make a second `load()` call fail while the first engine
//! is still alive (e.g. swapping models behind a `Mutex`). Instead we init the
//! backend once into a process-wide `OnceCell` and hand out `&'static` references
//! to it — every `LlmEngine` shares the same handle, so `load()` is safe to call
//! any number of times, sequentially or with multiple engines alive at once.

use anyhow::{anyhow, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use once_cell::sync::OnceCell;
use std::num::NonZeroU32;
use std::path::Path;

static BACKEND: OnceCell<LlamaBackend> = OnceCell::new();

fn backend() -> Result<&'static LlamaBackend> {
    BACKEND.get_or_try_init(|| {
        LlamaBackend::init().map_err(|e| anyhow!("failed to init llama backend: {e}"))
    })
}

pub struct LlmEngine {
    model: LlamaModel,
    n_threads: i32,
}

impl LlmEngine {
    pub fn load(model_path: &Path) -> Result<Self> {
        let backend = backend()?;
        // CPU-only: explicit n_gpu_layers(0) overrides llama.cpp's default of -1
        // ("offload everything"), matching the no-GPU-features convention even
        // though no GPU backend is compiled in to act on it.
        let params = LlamaModelParams::default().with_n_gpu_layers(0);
        let model = LlamaModel::load_from_file(backend, model_path, &params)
            .map_err(|e| anyhow!("failed to load LLM model: {e}"))?;
        let n_threads = crate::stt_tier::detect_cpu_profile()
            .physical_cores
            .saturating_sub(1)
            .max(1) as i32;
        Ok(Self { model, n_threads })
    }

    pub fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let backend = backend()?;
        let tokens = self.model.str_to_token(prompt, AddBos::Never)?;
        // Clamp to the model's trained context so an unusually long prompt can't
        // ask llama.cpp for a context size it doesn't support.
        let n_ctx = ((tokens.len() + max_tokens + 16) as u32)
            .max(1024)
            .min(self.model.n_ctx_train());
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(n_ctx))
            .with_n_threads(self.n_threads)
            .with_n_threads_batch(self.n_threads);
        let mut ctx = self.model.new_context(backend, ctx_params)?;

        let mut batch = LlamaBatch::new(tokens.len().max(512), 1);
        let last_idx = tokens.len() - 1;
        for (i, token) in tokens.iter().enumerate() {
            batch.add(*token, i as i32, &[0], i == last_idx)?;
        }
        ctx.decode(&mut batch)?;

        // Qwen3's official non-thinking sampling (temp 0.7, top-p 0.8, top-k 20);
        // the model card warns near-greedy decoding degrades output and loops.
        // Fixed dist seed keeps runs reproducible.
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(20),
            LlamaSampler::top_p(0.8, 1),
            LlamaSampler::temp(0.7),
            LlamaSampler::dist(42),
        ]);

        let mut out = String::new();
        // One decoder for the whole generation: UTF-8 chars split across token
        // boundaries decode correctly instead of turning into replacement chars.
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        for n_cur in (tokens.len() as i32..).take(max_tokens) {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            if self.model.is_eog_token(token) {
                break;
            }
            out.push_str(&self.model.token_to_piece(token, &mut decoder, true, None)?);
            batch.clear();
            batch.add(token, n_cur, &[0], true)?;
            ctx.decode(&mut batch)?;
        }
        Ok(out)
    }
}

// `LlmEngine` must be `Send`: it lives behind a `Mutex` and runs inside
// `spawn_blocking` (Task 7). `LlamaModel` is `unsafe impl Send` in llama-cpp-2;
// `n_threads` is a plain `i32`. No `LlamaBackend` field here (see module docs),
// so this falls out automatically — no unsafe impl needed on our side. This is
// a compile-time-only check: it never runs, so it fails the build (not a test)
// if `LlmEngine` ever stops being `Send`.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<LlmEngine>();
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Manual: MINDFLOW_LLM_TEST_MODEL=/path/to/Qwen3-0.6B-Q4_K_M.gguf \
    ///   cargo test llm_engine_smoke -- --ignored --nocapture
    #[test]
    #[ignore]
    fn llm_engine_smoke() {
        let path = std::env::var("MINDFLOW_LLM_TEST_MODEL").expect("set MINDFLOW_LLM_TEST_MODEL");
        let engine = LlmEngine::load(std::path::Path::new(&path)).unwrap();
        let prompt = crate::cleanup::build_chat_prompt(
            &crate::cleanup::build_system_prompt(
                &crate::cleanup::CleanupFlags {
                    smart: true,
                    self_correction: true,
                    preserve_technical: true,
                },
                &[],
            ),
            "um so I think we should uh ship it on friday",
        );
        let out = crate::cleanup::strip_think(&engine.generate(&prompt, 256).unwrap());
        assert!(!out.is_empty());
        assert!(!out.to_lowercase().contains("um "), "got: {out}");
    }
}
