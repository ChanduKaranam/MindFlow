pub mod engine;
pub mod manager;
pub mod prompt;
pub use engine::LlmEngine;
pub use manager::CleanupManager;
pub use prompt::{
    build_chat_prompt, build_system_prompt, is_sane_output, strip_think, CleanupFlags,
};

use crate::managers::model::ModelTier;

/// Default cleanup model for a CPU tier (mirrors stt_tier's recommendation).
pub fn default_cleanup_model_id(tier: ModelTier) -> &'static str {
    match tier {
        ModelTier::Turbo => "qwen3-0.6b-q4",
        ModelTier::Balanced => "qwen3-1.7b-q4",
        ModelTier::Max => "qwen3-4b-q4",
    }
}
