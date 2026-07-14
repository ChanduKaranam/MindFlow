pub mod engine;
pub mod manager;
pub mod prompt;
pub use engine::LlmEngine;
pub use manager::CleanupManager;
pub use prompt::{
    build_chat_prompt, build_command_prompt, build_system_prompt, is_sane_command_output,
    is_sane_output, strip_think, CleanupFlags,
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

/// Offline end-to-end eval harness (M7/M9): runs a real recording through the
/// actual pipeline — Parakeet STT → rule stages → chunked LLM cleanup — so
/// prompt/pipeline changes can be validated against a reference script.
/// Ignored: needs local model files. Run with:
///   MINDFLOW_EVAL_WAV=... MINDFLOW_EVAL_PARAKEET=... MINDFLOW_LLM_TEST_MODEL=... \
///   cargo test --lib recording_eval -- --ignored --nocapture
/// The Parakeet pass is cached in /tmp/eval_stt.txt (delete to redo STT).
#[cfg(test)]
mod recording_eval {
    use super::*;
    use std::time::Instant;
    use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams, TimestampGranularity};
    use transcribe_rs::onnx::Quantization;
    use transcribe_rs::SpeechModel;

    const CUSTOM_WORDS: &[&str] = &[
        "Karanam Purna Chandra Rao",
        "Venkata Sai Krishna",
        "Sai Charan",
        "Naga Venkata Lakshmi Prasanna",
        "Sree Harsha",
        "Vichinth Reddy",
        "Mrinal Singh",
        "Aishwarya Krishnamurthy",
        "Mohammed Abdul Rahman",
        "Shubham Choudhary",
        "Thiruvengadam Subramanian",
        "Sayi",
        "Sri Sai",
        "Rahul Varma",
        "Rahul V. R. Varma",
        "factorlab.in",
        "Vishakhapatnam",
        "Ollama",
        "Llama 3.2",
        "YOLO V8",
        "PostgreSQL",
        "FastAPI",
    ];

    #[test]
    #[ignore]
    fn eval_recording() {
        let wav_path = std::env::var("MINDFLOW_EVAL_WAV").unwrap();
        let parakeet_path = std::env::var("MINDFLOW_EVAL_PARAKEET").unwrap();
        let llm_path = std::env::var("MINDFLOW_LLM_TEST_MODEL").unwrap();

        let raw = if let Ok(cached) = std::fs::read_to_string("/tmp/eval_stt.txt") {
            println!("PARAKEET (cached) RAW >>>\n{cached}\n<<<");
            cached
        } else {
            let mut reader = hound::WavReader::open(&wav_path).unwrap();
            assert_eq!(reader.spec().sample_rate, 16000, "wav must be 16kHz");
            let audio: Vec<f32> = reader
                .samples::<i16>()
                .map(|s| s.unwrap() as f32 / 32768.0)
                .collect();
            let mut engine =
                ParakeetModel::load(std::path::Path::new(&parakeet_path), &Quantization::Int8)
                    .unwrap();
            let t = Instant::now();
            let params = ParakeetParams {
                timestamp_granularity: Some(TimestampGranularity::Segment),
                ..Default::default()
            };
            let raw = engine
                .transcribe_with(&audio, &params)
                .unwrap()
                .text
                .trim()
                .to_string();
            println!("PARAKEET ({:?}) RAW >>>\n{raw}\n<<<", t.elapsed());
            std::fs::write("/tmp/eval_stt.txt", &raw).unwrap();
            raw
        };

        let mut settings = crate::settings::get_default_settings();
        settings.custom_words = CUSTOM_WORDS.iter().map(|s| s.to_string()).collect();
        let ruled = crate::actions::apply_rule_stages(&raw, &settings);
        let ruled = crate::audio_toolkit::text::glue_spoken_emails(&ruled);
        println!("RULES >>>\n{ruled}\n<<<");

        let llm = LlmEngine::load(std::path::Path::new(&llm_path)).unwrap();
        let flags = CleanupFlags {
            smart: true,
            self_correction: true,
            preserve_technical: true,
        };
        let system = build_system_prompt(&flags, &settings.custom_words, "", false);
        let chunks = prompt::chunk_transcript(&ruled, 70);
        println!("CHUNKS: {}", chunks.len());
        let t = Instant::now();
        let mut outs = Vec::new();
        for chunk in &chunks {
            let p = build_chat_prompt(&system, chunk);
            let max_tokens = (chunk.split_whitespace().count() * 3).clamp(64, 512);
            let raw_out = llm.generate(&p, max_tokens, None).unwrap();
            let cleaned = strip_think(&raw_out);
            let ok = is_sane_output(&cleaned, chunk);
            println!("--- chunk (sane={ok}):\nOUT: {cleaned}");
            outs.push(if ok { cleaned } else { chunk.clone() });
        }
        println!(
            "LLM total {:?} FINAL >>>\n{}\n<<<",
            t.elapsed(),
            outs.join(" ")
        );
    }
}
