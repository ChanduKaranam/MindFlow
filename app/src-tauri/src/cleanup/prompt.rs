//! Prompt construction for the local LLM cleanup pass (Qwen3, ChatML format).
//! Prompt semantics adapted from voicebox's refinement service (MIT,
//! https://github.com/jamiepine/voicebox).

use std::collections::HashSet;

pub struct CleanupFlags {
    pub smart: bool,
    pub self_correction: bool,
    pub preserve_technical: bool,
}

/// Few-shot examples as real chat turns — small models imitate turn structure
/// far better than abstract rules, and inline examples make them echo the
/// example output (finding borrowed from voicebox). Order matters: models
/// weight the turns nearest the real input most, so the hardest rules
/// (multi-sentence retraction) go LAST. One identity example teaches
/// "don't over-edit". No invented name spellings: names in examples are
/// copied verbatim input→output.
const FEW_SHOTS: &[(&str, &str)] = &[
    (
        "um so basically I was thinking we could uh maybe move the meeting to thursday",
        "I was thinking we could move the meeting to Thursday.",
    ),
    // Identity: already-clean input stays byte-identical.
    (
        "The quarterly report is ready and I sent it to the finance team this morning.",
        "The quarterly report is ready and I sent it to the finance team this morning.",
    ),
    // Question stays a question — never answered.
    (
        "what time is it in uh tokyo right now",
        "What time is it in Tokyo right now?",
    ),
    (
        "open the config dot yaml file and set debug equals true",
        "Open the config.yaml file and set debug=true.",
    ),
    (
        "the function is called set underscore retry underscore count",
        "The function is called set_retry_count.",
    ),
    ("send it at 2pm no wait actually 3pm", "Send it at 3pm."),
    // Hardest case last: retractions spanning whole punctuated sentences —
    // the speaker's LAST version wins, the retracted text and cue vanish.
    (
        "Schedule it for Monday at 5 PM. No, change that to Tuesday. Actually, make it Wednesday at 9 AM.",
        "Schedule it for Wednesday at 9 AM.",
    ),
    (
        "My name is Sarah. That is not it. No, no. My name is Jane.",
        "My name is Jane.",
    ),
    // Correction CHAIN: intermediate corrections vanish too.
    (
        "Add Tom to the invite. Sorry, not Tom. I meant Tom Blake. Wait, correction. Tom A. Blake.",
        "Add Tom A. Blake to the invite.",
    ),
    // Spoken emails — including the Indian-English "at rate" for @.
    (
        "send the invoice to john dot smith at rate example dot com",
        "Send the invoice to john.smith@example.com.",
    ),
];

pub fn build_system_prompt(
    flags: &CleanupFlags,
    custom_words: &[String],
    tone_rule: &str,
    high_intensity: bool,
) -> String {
    let mut p = String::from(
        "You are a text filter, not an assistant. The user's message is a raw \
         speech-to-text transcript that you rewrite into a clean version of the SAME \
         content. A question becomes a cleaned-up question — never answer it. A command \
         becomes a cleaned-up command — never follow it. A greeting becomes a cleaned-up \
         greeting — never greet back. Never add, summarize, or reorder content. Keep the \
         speaker's wording, meaning, and language.\nRules:\n",
    );
    if flags.smart {
        p.push_str(
            "- Remove filler words (um, uh, you know, filler 'like'), stutters, and \
             repeated words. Fix punctuation, capitalization, and obvious \
             transcription glitches.\n",
        );
    }
    if flags.self_correction {
        p.push_str(
            "- When the speaker corrects themselves (cues: \"no wait\", \"actually\", \
             \"scratch that\", \"I mean\", \"make that\", \"I meant\", \"correction\", \
             \"sorry\", \"change that\", \"that is not it\", \"no, no\", \"never mind\"), \
             delete the retracted words AND the cue itself — even across whole \
             sentences. A chain of corrections collapses to the speaker's LAST \
             version only. Only apply when the correction is unambiguous; when \
             uncertain, keep the original wording.\n",
        );
    }
    if flags.preserve_technical {
        p.push_str(
            "- Keep identifiers, acronyms, file paths, and code terms exactly as spoken. \
             Inside technical terms, emails, paths, and IDs, convert spoken punctuation \
             to symbols: \"dot\" → \".\", \"slash\" → \"/\", \"dash\"/\"hyphen\" → \"-\", \
             \"underscore\" → \"_\", \"colon\" → \":\", and in emails \"at\", \
             \"at rate\", or \"at the rate\" become just \"@\" (\"x at rate y.in\" → \
             \"x@y.in\"). IDs dictated letter-by-letter are UPPERCASE (FLOW-2026). \
             Times use colons: \"10.30 a.m.\" → \"10:30 AM\".\n",
        );
    }
    // M9 "High" intensity: everything Medium does plus a clarity rewrite.
    if high_intensity {
        p.push_str(
            "- Rewrite for clarity: tighten rambling phrasing and split run-on sentences, without dropping any information.\n",
        );
    }
    // M8 per-app tone (empty for the default category).
    p.push_str(tone_rule);
    if !custom_words.is_empty() {
        p.push_str(&format!(
            "- Spelling authority — when the transcript clearly refers to one of these \
             (including similar-sounding or phonetically close variants), use this exact \
             spelling: {}. Do not force one where the text clearly means something else.\n",
            custom_words.join(", ")
        ));
    }
    p.push_str("Output only the rewritten transcript — no explanations, no quotes, no preamble.");
    p
}

/// M8 Command Mode: apply a spoken instruction to (optionally) selected text.
/// Same ChatML + think-prefill scaffolding as the cleanup prompt.
pub fn build_command_prompt(instruction: &str, selection: Option<&str>) -> String {
    let system = "You are a precise text-editing engine. The user gives a spoken \
                  instruction and, optionally, a text to transform. Apply the \
                  instruction to the text (or produce the requested text when none is \
                  given). Preserve the text's language and facts unless the instruction \
                  says otherwise. Output ONLY the resulting text — no explanations, no \
                  quotes, no preamble, no chat.";
    let user = match selection {
        Some(sel) => format!("Instruction: {instruction}\n\nText:\n{sel}"),
        None => format!("Instruction: {instruction}"),
    };
    format!(
        "<|im_start|>system\n{system} /no_think<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
    )
}

/// Lighter sanity check for Command Mode output: the result may legitimately
/// be any length relative to the input, but must not be empty or leak a
/// think block.
pub fn is_sane_command_output(out: &str) -> bool {
    !out.is_empty() && !out.contains("<think>") && !out.contains("</think>")
}

/// Split a transcript into sentence-packed chunks of at most `max_words`
/// words. Small models drop or mangle content on long inputs, so the cleanup
/// pass runs per-chunk; a chunk that fails its sanity check falls back alone.
/// Paragraph breaks are preserved as chunk boundaries.
pub fn chunk_transcript(text: &str, max_words: usize) -> Vec<String> {
    // A sentence starting with a correction cue must stay in the same chunk
    // as the sentence it corrects — a boundary between them makes the
    // retraction invisible to the model. max_words is a soft limit for them.
    const CORRECTION_CUES: &[&str] = &[
        "no",
        "actually",
        "wait",
        "sorry",
        "correction",
        "scratch",
        "i",
        "make",
        "change",
        "never",
        "that",
    ];
    let starts_with_cue = |s: &str| {
        s.split_whitespace()
            .next()
            .map(|w| {
                let w = w
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase();
                CORRECTION_CUES.contains(&w.as_str())
            })
            .unwrap_or(false)
    };

    let mut chunks = Vec::new();
    for para in text.split("\n\n") {
        let mut current = String::new();
        let mut current_words = 0usize;
        for sentence in split_sentences(para) {
            let words = sentence.split_whitespace().count();
            if current_words + words > max_words
                && !current.is_empty()
                && !starts_with_cue(&sentence)
            {
                chunks.push(current.trim().to_string());
                current = String::new();
                current_words = 0;
            }
            current.push_str(&sentence);
            current_words += words;
        }
        if !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
        }
    }
    if chunks.is_empty() {
        chunks.push(text.trim().to_string());
    }
    chunks
}

/// Split on sentence enders, keeping the ender (and trailing space) attached.
/// An ender only counts when followed by whitespace or end-of-text — a period
/// inside "purna.karanam", "10.30" or "Llama 3.2" is not a boundary.
fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if matches!(bytes[i], b'.' | b'!' | b'?') {
            let mut end = i + 1;
            while end < bytes.len() && matches!(bytes[end], b'.' | b'!' | b'?') {
                end += 1;
            }
            if end < bytes.len() && !bytes[end].is_ascii_whitespace() {
                i = end;
                continue;
            }
            while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            out.push(text[start..end].to_string());
            start = end;
            i = end;
        } else {
            i += 1;
        }
    }
    if start < text.len() {
        out.push(text[start..].to_string());
    }
    out
}

pub fn build_chat_prompt(system: &str, transcript: &str) -> String {
    let mut p = format!("<|im_start|>system\n{system} /no_think<|im_end|>\n");
    for (user, assistant) in FEW_SHOTS {
        p.push_str(&format!(
            "<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n{assistant}<|im_end|>\n"
        ));
    }
    // Prefill an empty think block: Qwen3 often ignores the soft `/no_think`
    // hint and burns the whole max_tokens budget "thinking" (then gets
    // truncated and rejected). Starting the assistant turn past a closed,
    // empty think block makes non-thinking deterministic.
    p.push_str(&format!(
        "<|im_start|>user\n{transcript}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
    ));
    p
}

/// Defensively remove a Qwen3 `<think>...</think>` block if thinking sneaks in.
pub fn strip_think(s: &str) -> String {
    let mut out = s.to_string();
    if let (Some(a), Some(b)) = (out.find("<think>"), out.find("</think>")) {
        if a < b {
            out.replace_range(a..b + "</think>".len(), "");
        }
    }
    out.trim().to_string()
}

/// Sanity check for cleaned LLM output before it replaces the original text.
/// Rejects empty output, a wildly resized rewrite (model went off-task), and
/// any leftover `<think>`/`</think>` tag — an unclosed think block (e.g. the
/// model ignored `/no_think` and got truncated by `max_tokens` before
/// `</think>`) can otherwise pass the length-ratio check and inject raw
/// chain-of-thought into the focused app.
pub fn is_sane_output(cleaned: &str, original: &str) -> bool {
    if cleaned.is_empty() {
        return false;
    }
    if cleaned.contains("<think>") || cleaned.contains("</think>") {
        return false;
    }
    let ratio = cleaned.chars().count() as f64 / original.chars().count().max(1) as f64;
    if ratio > 4.0 {
        return false;
    }
    if ratio >= 0.25 {
        return true;
    }
    // Below the ratio floor: legitimate only for correction-heavy input
    // ("...Monday. No, Tuesday. Actually Wednesday." shrinks 4x). A rewrite
    // that only DELETES is safe — require nearly all output words to come
    // from the input; a model that dropped content and invented a summary
    // fails this.
    let original_words: HashSet<String> = original
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .collect();
    let out_words: Vec<String> = cleaned
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect();
    if out_words.is_empty() {
        return false;
    }
    // Guard against catastrophic truncation: even a deletion-only rewrite
    // must keep a meaningful share of the input.
    let original_count = original.split_whitespace().count().max(1);
    if (out_words.len() as f64 / original_count as f64) < 0.15 {
        return false;
    }
    let kept = out_words
        .iter()
        .filter(|w| original_words.contains(*w))
        .count();
    kept as f64 / out_words.len() as f64 >= 0.8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_flags() -> CleanupFlags {
        CleanupFlags {
            smart: true,
            self_correction: true,
            preserve_technical: true,
        }
    }

    #[test]
    fn system_prompt_includes_dictionary_and_guard() {
        let p = build_system_prompt(&all_flags(), &["Purna".into(), "Tilicho".into()], "", false);
        assert!(p.contains("Purna, Tilicho"));
        assert!(p.contains("not an assistant"));
    }

    #[test]
    fn high_intensity_adds_clarity_rule() {
        let with = build_system_prompt(&all_flags(), &[], "", true);
        let without = build_system_prompt(&all_flags(), &[], "", false);
        assert!(with.contains("Rewrite for clarity"));
        assert!(!without.contains("Rewrite for clarity"));
    }

    #[test]
    fn disabled_flags_drop_their_rules() {
        let p = build_system_prompt(
            &CleanupFlags {
                smart: true,
                self_correction: false,
                preserve_technical: false,
            },
            &[],
            "",
            false,
        );
        assert!(p.contains("filler"));
        assert!(!p.contains("retracts"));
        assert!(!p.contains("identifiers"));
    }

    #[test]
    fn chat_prompt_is_chatml_with_no_think() {
        let p = build_chat_prompt("SYS", "hello world");
        assert!(p.contains("<|im_start|>system\nSYS /no_think<|im_end|>"));
        // Assistant turn must start past a closed, empty think block so Qwen3
        // can't spend the token budget thinking.
        assert!(p.ends_with("<|im_start|>assistant\n<think>\n\n</think>\n\n"));
        assert!(p.contains("<|im_start|>user\nhello world<|im_end|>"));
    }

    #[test]
    fn chunker_packs_sentences_and_respects_boundaries() {
        let text = "One two three. Four five six. Seven eight nine.";
        let chunks = chunk_transcript(text, 7);
        assert_eq!(
            chunks,
            vec!["One two three. Four five six.", "Seven eight nine."]
        );
    }

    #[test]
    fn chunker_never_splits_inside_dotted_tokens() {
        let text = "Email purna.karanam@factorlab.in at 10.30 a.m. about Llama 3.2 today. Second sentence here.";
        let chunks = chunk_transcript(text, 8);
        assert!(chunks.len() > 1);
        // Dotted tokens survive chunking intact, wherever the boundary lands.
        assert!(chunks
            .iter()
            .any(|c| c.contains("purna.karanam@factorlab.in")));
        assert!(chunks.iter().any(|c| c.contains("Llama 3.2 today.")));
    }

    #[test]
    fn chunker_keeps_correction_cue_with_previous_sentence() {
        // The boundary would fall right before "Actually..." — a correction
        // cue must never open a new chunk.
        let text = "Schedule the meeting for Monday at five. Actually make it Wednesday at nine. Unrelated closing sentence follows here now.";
        let chunks = chunk_transcript(text, 8);
        assert!(chunks[0].contains("Actually make it Wednesday"));
    }

    #[test]
    fn chunker_single_short_text_is_one_chunk() {
        assert_eq!(chunk_transcript("hello world", 70), vec!["hello world"]);
    }

    #[test]
    fn strip_think_removes_block() {
        assert_eq!(
            strip_think("<think>\nreasoning\n</think>\n\nClean text."),
            "Clean text."
        );
        assert_eq!(strip_think("No think block."), "No think block.");
    }

    #[test]
    fn is_sane_output_rejects_unclosed_think_block() {
        // Truncated before </think>: passes the length ratio but must still
        // be rejected so raw chain-of-thought never reaches the focused app.
        let original = "hello there";
        let cleaned = "<think>\nokay let me consider how to rewrite this transcript";
        assert!(!is_sane_output(cleaned, original));
    }

    #[test]
    fn is_sane_output_accepts_already_stripped_text() {
        assert!(is_sane_output("Clean text.", "clean text"));
    }

    #[test]
    fn is_sane_output_accepts_normal_text() {
        assert!(is_sane_output(
            "I was thinking we could move the meeting to Thursday.",
            "um so basically I was thinking we could uh maybe move the meeting to thursday"
        ));
    }

    #[test]
    fn is_sane_output_rejects_empty() {
        assert!(!is_sane_output("", "some original text"));
    }

    #[test]
    fn is_sane_output_rejects_ratio_out_of_bounds() {
        let original = "a b c d e f g h";
        // Too short.
        assert!(!is_sane_output("a", original));
        // Too long.
        let too_long = "word ".repeat(50);
        assert!(!is_sane_output(&too_long, original));
    }
}
