pub struct SpokenCommandsConfig {
    pub enabled: bool,
    pub number_conversion: bool,
}

/// Newline triggers, longest phrase first so "new paragraph" wins over "new".
const NEWLINE_TRIGGERS: &[(&str, &str)] = &[
    ("new paragraph", "\n\n"),
    ("next paragraph", "\n\n"),
    ("new line", "\n"),
    ("newline", "\n"),
];

pub fn apply_spoken_commands(text: &str, config: &SpokenCommandsConfig) -> String {
    if !config.enabled {
        return text.to_string();
    }
    // Later tasks insert punctuation/capitalization/number passes here, BEFORE
    // newlines (newlines run last because they introduce '\n').
    apply_newlines(text)
}

/// Split into sentences, each chunk keeping its terminator and trailing whitespace.
fn split_sentences_with_trailing_ws(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        cur.push(c);
        if c == '.' || c == '!' || c == '?' {
            while let Some(&n) = chars.peek() {
                if n == ' ' || n == '\t' || n == '\n' {
                    cur.push(n);
                    chars.next();
                } else {
                    break;
                }
            }
            chunks.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    chunks
}

/// Lowercase, trim, drop a trailing sentence terminator, collapse inner whitespace.
fn normalize_phrase(s: &str) -> String {
    let t = s.trim().trim_end_matches(['.', '!', '?']).trim();
    t.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Replace any sentence whose entire content is a newline trigger with the break,
/// trimming the trailing space the previous sentence left behind.
fn apply_newlines(text: &str) -> String {
    let mut result = String::new();
    for chunk in split_sentences_with_trailing_ws(text) {
        let norm = normalize_phrase(chunk.trim_end());
        if let Some((_, repl)) = NEWLINE_TRIGGERS.iter().find(|(trig, _)| *trig == norm) {
            while result.ends_with(' ') || result.ends_with('\t') {
                result.pop();
            }
            result.push_str(repl);
        } else {
            result.push_str(&chunk);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(enabled: bool, number_conversion: bool) -> SpokenCommandsConfig {
        SpokenCommandsConfig { enabled, number_conversion }
    }

    #[test]
    fn disabled_is_identity() {
        let input = "New paragraph.";
        assert_eq!(apply_spoken_commands(input, &cfg(false, false)), input);
    }

    #[test]
    fn standalone_new_paragraph_becomes_double_newline() {
        let out = apply_spoken_commands("Hello there. New paragraph. Goodbye.", &cfg(true, false));
        assert_eq!(out, "Hello there.\n\nGoodbye.");
    }

    #[test]
    fn standalone_new_line_becomes_single_newline() {
        let out = apply_spoken_commands("First. New line. Second.", &cfg(true, false));
        assert_eq!(out, "First.\nSecond.");
    }

    #[test]
    fn trailing_new_paragraph_without_terminator() {
        let out = apply_spoken_commands("Done. New paragraph", &cfg(true, false));
        assert_eq!(out, "Done.\n\n");
    }

    #[test]
    fn literal_new_line_in_prose_is_untouched() {
        let input = "I started a new line of work today.";
        assert_eq!(apply_spoken_commands(input, &cfg(true, false)), input);
    }
}
