use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}

/// Byte-offset spans of maximal alphanumeric runs ("words") in `text`.
fn word_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if c.is_alphanumeric() {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            spans.push((s, i));
        }
    }
    if let Some(s) = start {
        spans.push((s, text.len()));
    }
    spans
}

/// Replace every boundary-aligned, whitespace-flexible occurrence of the word
/// sequence `from_words` (already lowercased) with `to`.
fn replace_phrase(text: &str, from_words: &[String], to: &str) -> String {
    let spans = word_spans(text);
    let n = from_words.len();
    if n == 0 {
        return text.to_string();
    }
    let mut result = String::new();
    let mut copied_to = 0usize;
    let mut i = 0usize;
    while i + n <= spans.len() {
        let mut matches = true;
        for k in 0..n {
            let (s, e) = spans[i + k];
            if text[s..e].to_lowercase() != from_words[k] {
                matches = false;
                break;
            }
            if k > 0 {
                let prev_end = spans[i + k - 1].1;
                if !text[prev_end..s].chars().all(|c| c.is_whitespace()) {
                    matches = false;
                    break;
                }
            }
        }
        if matches {
            let match_start = spans[i].0;
            let match_end = spans[i + n - 1].1;
            result.push_str(&text[copied_to..match_start]);
            result.push_str(to);
            copied_to = match_end;
            i += n;
        } else {
            i += 1;
        }
    }
    result.push_str(&text[copied_to..]);
    result
}

pub fn apply_replacements(text: &str, rules: &[Replacement]) -> String {
    // Prepare: drop empty `from`; precompute lowercased word lists.
    let mut prepared: Vec<(Vec<String>, &str)> = rules
        .iter()
        .filter_map(|r| {
            let words: Vec<String> =
                r.from.split_whitespace().map(|w| w.to_lowercase()).collect();
            if words.is_empty() {
                None
            } else {
                Some((words, r.to.as_str()))
            }
        })
        .collect();
    // Longest phrase first (by word count, then by character length) so a
    // multi-word rule wins over a shorter rule it contains.
    prepared.sort_by(|a, b| {
        b.0.len()
            .cmp(&a.0.len())
            .then_with(|| b.0.join(" ").len().cmp(&a.0.join(" ").len()))
    });

    let mut current = text.to_string();
    for (from_words, to) in &prepared {
        current = replace_phrase(&current, from_words, to);
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(from: &str, to: &str) -> Replacement {
        Replacement { from: from.into(), to: to.into() }
    }

    #[test]
    fn case_insensitive_verbatim_replacement() {
        let rules = vec![rule("poor nachem darao", "Purna Chandra Rao")];
        assert_eq!(
            apply_replacements("i am poor nachem darao", &rules),
            "i am Purna Chandra Rao"
        );
        assert_eq!(
            apply_replacements("I AM POOR NACHEM DARAO", &rules),
            "I AM Purna Chandra Rao"
        );
    }

    #[test]
    fn does_not_match_inside_larger_word() {
        let rules = vec![rule("cat", "dog")];
        assert_eq!(apply_replacements("category cat", &rules), "category dog");
    }

    #[test]
    fn longest_from_wins() {
        let rules = vec![rule("york", "Y"), rule("new york", "NYC")];
        assert_eq!(apply_replacements("new york", &rules), "NYC");
    }

    #[test]
    fn whitespace_between_words_is_flexible() {
        let rules = vec![rule("my email", "x@y.com")];
        assert_eq!(apply_replacements("my   email", &rules), "x@y.com");
    }

    #[test]
    fn replaces_all_occurrences() {
        let rules = vec![rule("cat", "dog")];
        assert_eq!(apply_replacements("cat and cat", &rules), "dog and dog");
    }

    #[test]
    fn non_whitespace_separator_does_not_match_phrase() {
        let rules = vec![rule("my email", "X")];
        assert_eq!(apply_replacements("my-email", &rules), "my-email");
        assert_eq!(apply_replacements("my, email", &rules), "my, email");
    }

    #[test]
    fn empty_from_is_ignored() {
        let rules = vec![rule("   ", "x")];
        assert_eq!(apply_replacements("hello", &rules), "hello");
    }

    #[test]
    fn empty_to_deletes_the_match() {
        let rules = vec![rule("um", "")];
        assert_eq!(apply_replacements("a um b", &rules), "a  b");
    }
}
