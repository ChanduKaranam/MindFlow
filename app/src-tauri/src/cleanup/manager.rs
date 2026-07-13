use crate::cleanup::{
    build_chat_prompt, build_system_prompt, is_sane_output, strip_think, CleanupFlags, LlmEngine,
};
use crate::managers::model::{EngineType, ModelManager};
use crate::settings::AppSettings;
use anyhow::{anyhow, Result};
use log::{error, warn};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const CLEANUP_TIMEOUT: Duration = Duration::from_secs(10);

/// The cheap guard checks `cleanup()` runs before touching the model manager
/// or spawning any work — pulled out as a pure function so the fallback
/// behavior is unit-testable without a live `ModelManager` (which needs a
/// real `AppHandle`, unavailable in a plain `cargo test`).
/// Preferred model if it's among the downloaded text-LLMs, else the largest
/// downloaded one — running cleanup on a smaller model always beats silently
/// skipping it when the configured model was never downloaded.
fn pick_cleanup_model(preferred: &str, downloaded_llms: &[(String, u64)]) -> Option<String> {
    if downloaded_llms.iter().any(|(id, _)| id == preferred) {
        return Some(preferred.to_string());
    }
    downloaded_llms
        .iter()
        .max_by_key(|(_, size_mb)| *size_mb)
        .map(|(id, _)| id.clone())
}

fn should_attempt_cleanup(text: &str, settings: &AppSettings) -> bool {
    if !settings.ai_cleanup_enabled || text.trim().is_empty() {
        return false;
    }
    settings.cleanup_smart
        || settings.cleanup_self_correction
        || settings.cleanup_preserve_technical
}

pub struct CleanupManager {
    model_manager: Arc<ModelManager>,
    /// (model_id, engine) — reloaded when the configured model changes.
    engine: Arc<Mutex<Option<(String, LlmEngine)>>>,
}

impl CleanupManager {
    pub fn new(model_manager: Arc<ModelManager>) -> Self {
        Self {
            model_manager,
            engine: Arc::new(Mutex::new(None)),
        }
    }

    fn resolve_model_id(&self, settings: &AppSettings) -> Option<String> {
        let preferred = settings.cleanup_model_id.clone().unwrap_or_else(|| {
            crate::cleanup::default_cleanup_model_id(crate::stt_tier::recommend_tier(
                &crate::stt_tier::detect_cpu_profile(),
            ))
            .to_string()
        });
        let downloaded_llms: Vec<(String, u64)> = self
            .model_manager
            .get_available_models()
            .into_iter()
            .filter(|m| m.is_downloaded && matches!(m.engine_type, EngineType::TextLlm))
            .map(|m| (m.id, m.size_mb))
            .collect();
        pick_cleanup_model(&preferred, &downloaded_llms)
    }

    /// Run the LLM cleanup pass. `None` means "use the rules-only text" — the
    /// caller must treat every failure as a silent fallback, never an error.
    pub async fn cleanup(&self, text: &str, settings: &AppSettings) -> Option<String> {
        if !should_attempt_cleanup(text, settings) {
            return None;
        }
        let flags = CleanupFlags {
            smart: settings.cleanup_smart,
            self_correction: settings.cleanup_self_correction,
            preserve_technical: settings.cleanup_preserve_technical,
        };
        let model_id = match self.resolve_model_id(settings) {
            Some(id) => id,
            None => {
                warn!("AI cleanup: no downloaded text-LLM model configured, falling back");
                return None;
            }
        };
        let path = match self.model_manager.get_model_path(&model_id) {
            Ok(path) => path,
            Err(e) => {
                warn!("AI cleanup: model path unavailable for {model_id}: {e}, falling back");
                return None;
            }
        };
        let prompt = build_chat_prompt(&build_system_prompt(&flags, &settings.custom_words), text);
        let max_tokens = (text.split_whitespace().count() * 3).clamp(64, 2048);

        let engine_slot = Arc::clone(&self.engine);
        let task = tauri::async_runtime::spawn_blocking(move || -> Result<String> {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut guard = engine_slot.lock().unwrap_or_else(|e| e.into_inner());
                let needs_load = !matches!(&*guard, Some((id, _)) if *id == model_id);
                if needs_load {
                    *guard = Some((model_id.clone(), LlmEngine::load(&path)?));
                }
                let (_, engine) = guard.as_ref().expect("just loaded");
                engine.generate(&prompt, max_tokens)
            }));
            match outcome {
                Ok(r) => r,
                Err(_) => {
                    // Engine state unknown after a panic — drop it so it reloads next time.
                    *engine_slot.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    Err(anyhow!("cleanup engine panicked"))
                }
            }
        });

        // ponytail: on timeout the blocking task is abandoned, not killed; a wedged
        // generate() serializes later cleanups on the engine mutex — upgrade to a
        // dedicated worker thread with a kill switch if this shows up in practice.
        let raw = match tokio::time::timeout(CLEANUP_TIMEOUT, task).await {
            Ok(Ok(Ok(text))) => text,
            Ok(Ok(Err(e))) => {
                error!("AI cleanup failed, falling back to rules-only text: {e}");
                return None;
            }
            Ok(Err(join_err)) => {
                error!("AI cleanup task join error: {join_err}");
                return None;
            }
            Err(_) => {
                warn!("AI cleanup timed out after {CLEANUP_TIMEOUT:?}, falling back");
                return None;
            }
        };

        let cleaned = strip_think(&raw);
        if !is_sane_output(&cleaned, text) {
            warn!("AI cleanup output rejected (empty, bad length ratio, or unclosed think block), falling back");
            return None;
        }
        Some(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::get_default_settings;

    // `CleanupManager::cleanup()` needs a live `ModelManager`, which needs a
    // real `AppHandle` (no generic `Runtime` param, and this codebase has no
    // mock-runtime test harness) — unavailable in a plain `cargo test`. Its
    // fallback-to-`None` guards run before any of that is touched, so we test
    // them directly via `should_attempt_cleanup`, the exact predicate
    // `cleanup()` checks first. The real-model `generate()` path is covered
    // by `engine::tests::llm_engine_smoke` (`#[ignore]`d, run manually with a
    // real GGUF file).

    fn llms(entries: &[(&str, u64)]) -> Vec<(String, u64)> {
        entries.iter().map(|(id, s)| (id.to_string(), *s)).collect()
    }

    #[test]
    fn preferred_model_wins_when_downloaded() {
        let downloaded = llms(&[("qwen3-0.6b-q4", 484), ("qwen3-4b-q4", 2382)]);
        assert_eq!(
            pick_cleanup_model("qwen3-0.6b-q4", &downloaded).as_deref(),
            Some("qwen3-0.6b-q4")
        );
    }

    #[test]
    fn falls_back_to_largest_downloaded_llm() {
        // The user's real bug: 4B selected ("recommended"), only 1.7B downloaded —
        // cleanup must use the 1.7B instead of silently doing nothing.
        let downloaded = llms(&[("qwen3-0.6b-q4", 484), ("qwen3-1.7b-q4", 1056)]);
        assert_eq!(
            pick_cleanup_model("qwen3-4b-q4", &downloaded).as_deref(),
            Some("qwen3-1.7b-q4")
        );
    }

    #[test]
    fn no_downloaded_llms_means_no_cleanup() {
        assert_eq!(pick_cleanup_model("qwen3-4b-q4", &[]), None);
    }

    #[test]
    fn disabled_setting_blocks_cleanup() {
        let mut settings = get_default_settings();
        settings.ai_cleanup_enabled = false;
        assert!(!should_attempt_cleanup("um hello there", &settings));
    }

    #[test]
    fn empty_text_blocks_cleanup() {
        let settings = get_default_settings();
        assert!(!should_attempt_cleanup("   ", &settings));
    }

    #[test]
    fn all_flags_off_blocks_cleanup() {
        let mut settings = get_default_settings();
        settings.cleanup_smart = false;
        settings.cleanup_self_correction = false;
        settings.cleanup_preserve_technical = false;
        assert!(!should_attempt_cleanup("um hello there", &settings));
    }

    #[test]
    fn default_settings_attempt_cleanup() {
        let settings = get_default_settings();
        assert!(should_attempt_cleanup("um hello there", &settings));
    }
}
