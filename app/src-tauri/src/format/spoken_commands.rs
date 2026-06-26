pub struct SpokenCommandsConfig {
    pub enabled: bool,
    pub number_conversion: bool,
}

/// Punctuation triggers, two-word phrases listed so they are matched before one-word.
const PUNCT_TRIGGERS: &[(&str, &str)] = &[
    ("full stop", "."),
    ("question mark", "?"),
    ("exclamation mark", "!"),
    ("exclamation point", "!"),
    ("open paren", "("),
    ("open parenthesis", "("),
    ("close paren", ")"),
    ("close parenthesis", ")"),
    ("period", "."),
    ("comma", ","),
    ("colon", ":"),
    ("semicolon", ";"),
    ("hyphen", "-"),
    ("dash", "-"),
];

/// Symbols that attach to the previous token with no leading space.
const ATTACH_LEFT: &[&str] = &[".", ",", "?", "!", ":", ";", ")"];

fn strip_edges(token: &str) -> &str {
    token.trim_matches(|c: char| ".,?!:;()".contains(c))
}

fn lookup_punct(phrase: &str) -> Option<&'static str> {
    PUNCT_TRIGGERS
        .iter()
        .find(|(trig, _)| *trig == phrase)
        .map(|(_, sym)| *sym)
}

fn apply_punctuation(text: &str) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let two = if i + 1 < tokens.len() {
            Some(format!(
                "{} {}",
                strip_edges(tokens[i]).to_lowercase(),
                strip_edges(tokens[i + 1]).to_lowercase()
            ))
        } else {
            None
        };
        let one = strip_edges(tokens[i]).to_lowercase();

        if let Some(sym) = two.as_deref().and_then(lookup_punct) {
            push_symbol(&mut out, sym);
            i += 2;
        } else if let Some(sym) = lookup_punct(&one) {
            push_symbol(&mut out, sym);
            i += 1;
        } else {
            out.push(tokens[i].to_string());
            i += 1;
        }
    }
    join_with_smart_spacing(&out)
}

/// Append a punctuation symbol, collapsing an immediately-preceding identical
/// symbol (idempotency: "done." + "period" must not yield "done..").
fn push_symbol(out: &mut Vec<String>, sym: &str) {
    if let Some(last) = out.last() {
        if last.ends_with(sym) && ATTACH_LEFT.contains(&sym) {
            return;
        }
    }
    out.push(sym.to_string());
}

fn join_with_smart_spacing(tokens: &[String]) -> String {
    let mut result = String::new();
    let mut prev_open_paren = false;
    for tok in tokens {
        let attach_left = ATTACH_LEFT.contains(&tok.as_str());
        if result.is_empty() || attach_left || prev_open_paren {
            result.push_str(tok);
        } else {
            result.push(' ');
            result.push_str(tok);
        }
        prev_open_paren = tok == "(";
    }
    result
}

/// Newline triggers, longest phrase first so "new paragraph" wins over "new".
const NEWLINE_TRIGGERS: &[(&str, &str)] = &[
    ("new paragraph", "\n\n"),
    ("next paragraph", "\n\n"),
    ("new line", "\n"),
    ("newline", "\n"),
];

fn apply_capitalization(text: &str) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut out: Vec<String> = Vec::new();
    let mut caps_region = false;
    let mut uppercase_next = false;
    let mut i = 0;
    while i < tokens.len() {
        let low = strip_edges(tokens[i]).to_lowercase();
        let low_next = tokens.get(i + 1).map(|t| strip_edges(t).to_lowercase());

        if low == "all" && low_next.as_deref() == Some("caps") {
            uppercase_next = true;
            i += 2;
            continue;
        }
        if low == "caps" && low_next.as_deref() == Some("on") {
            caps_region = true;
            i += 2;
            continue;
        }
        if low == "caps" && low_next.as_deref() == Some("off") {
            caps_region = false;
            i += 2;
            continue;
        }

        if uppercase_next {
            out.push(tokens[i].to_uppercase());
            uppercase_next = false;
        } else if caps_region {
            out.push(tokens[i].to_uppercase());
        } else {
            out.push(tokens[i].to_string());
        }
        i += 1;
    }
    join_with_smart_spacing(&out)
}

pub fn apply_spoken_commands(text: &str, config: &SpokenCommandsConfig) -> String {
    if !config.enabled {
        return text.to_string();
    }
    let mut out = apply_punctuation(text);
    out = apply_capitalization(&out);
    // (Task 8) if config.number_conversion { out = apply_numbers(&out); }
    out = apply_newlines(&out);
    out
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

    // --- Task 6: punctuation spoken-commands ---

    #[test]
    fn comma_word_becomes_symbol_attached() {
        let out = apply_spoken_commands("hello comma world", &cfg(true, false));
        assert_eq!(out, "hello, world");
    }

    #[test]
    fn two_word_question_mark() {
        let out = apply_spoken_commands("really question mark", &cfg(true, false));
        assert_eq!(out, "really?");
    }

    #[test]
    fn open_paren_attaches_to_next_word() {
        let out = apply_spoken_commands("see open paren note close paren", &cfg(true, false));
        assert_eq!(out, "see (note)");
    }

    #[test]
    fn punctuation_is_idempotent_no_double() {
        // STT already attached the period; saying "period" must not double it.
        let out = apply_spoken_commands("done. period", &cfg(true, false));
        assert_eq!(out, "done.");
    }

    // --- Task 7: capitalization spoken-commands ---

    #[test]
    fn all_caps_uppercases_next_word_only() {
        let out = apply_spoken_commands("the all caps api is ready", &cfg(true, false));
        assert_eq!(out, "the API is ready");
    }

    #[test]
    fn caps_on_off_uppercases_region() {
        let out = apply_spoken_commands("say caps on hello world caps off now", &cfg(true, false));
        assert_eq!(out, "say HELLO WORLD now");
    }
}
