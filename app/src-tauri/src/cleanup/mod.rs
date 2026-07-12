pub mod engine;
pub mod prompt;
#[allow(unused_imports)] // consumed by Task 7 (cleanup pipeline)
pub use engine::LlmEngine;
pub use prompt::{build_chat_prompt, build_system_prompt, strip_think, CleanupFlags};
