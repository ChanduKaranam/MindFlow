//! Auto-learn: word-level diff between a transcript and the user's edit of it,
//! extracting corrected proper nouns for the custom-words dictionary.

use crate::audio_toolkit::text::COMMON_WORDS;
use strsim::levenshtein;

fn norm(w: &str) -> String {
    w.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// Punctuation-trimmed but case-preserving form, used for the LCS match itself.
/// Case-*insensitive* matching here would treat "rao" (before) and "Rao" (after)
/// as the same word and silently drop it from the diff — even though it's part
/// of the corrected name. Keeping case makes a pure case-fix still count as a
/// changed word, so it stays attached to its phrase; the later filter step
/// (which does compare case-insensitively) is what discards pure case-only edits.
fn norm_cased(w: &str) -> String {
    w.trim_matches(|c: char| !c.is_alphanumeric()).to_string()
}

/// Substitution phrases (≤3 words) in `after` that look like name corrections
/// of something in `before`: textually close (a respelling, not a rewrite) and
/// capitalized (name-like). Conservative by design — false negatives are fine,
/// false dictionary entries are not.
pub fn learned_phrases(before: &str, after: &str) -> Vec<String> {
    let b: Vec<&str> = before.split_whitespace().collect();
    let a: Vec<&str> = after.split_whitespace().collect();
    let bn: Vec<String> = b.iter().map(|w| norm_cased(w)).collect();
    let an: Vec<String> = a.iter().map(|w| norm_cased(w)).collect();

    // LCS table
    let mut dp = vec![vec![0u32; an.len() + 1]; bn.len() + 1];
    for i in (0..bn.len()).rev() {
        for j in (0..an.len()).rev() {
            dp[i][j] = if bn[i] == an[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    // Walk the table collecting (removed-from-before, inserted-into-after) blocks.
    let mut subs: Vec<(Vec<&str>, Vec<&str>)> = Vec::new();
    let mut cur: Option<(Vec<&str>, Vec<&str>)> = None;
    let (mut i, mut j) = (0usize, 0usize);
    while i < bn.len() || j < an.len() {
        if i < bn.len() && j < an.len() && bn[i] == an[j] {
            if let Some(s) = cur.take() {
                subs.push(s);
            }
            i += 1;
            j += 1;
        } else if j < an.len() && (i >= bn.len() || dp[i][j + 1] >= dp[i + 1][j]) {
            cur.get_or_insert_with(|| (vec![], vec![])).1.push(a[j]);
            j += 1;
        } else {
            cur.get_or_insert_with(|| (vec![], vec![])).0.push(b[i]);
            i += 1;
        }
    }
    if let Some(s) = cur.take() {
        subs.push(s);
    }

    subs.into_iter()
        .filter_map(|(mut from, mut to)| {
            // The case-sensitive walk keeps case-only fixes in the diff, so an
            // ordinary word that was just capitalized next to a real respelling
            // ("monday"→"Monday" beside "shandra"→"Chandra") merges into the
            // same block and would be learned as part of the name. Pairwise-trim
            // such common-word case fixes off both ends of equal-length blocks;
            // unequal blocks are left as-is (conservative).
            if from.len() == to.len() {
                let common_case_fix = |f: &str, t: &str| {
                    let fl = norm(f);
                    fl == norm(t) && COMMON_WORDS.contains(fl.as_str())
                };
                while from
                    .first()
                    .zip(to.first())
                    .is_some_and(|(f, t)| common_case_fix(f, t))
                {
                    from.remove(0);
                    to.remove(0);
                }
                while from
                    .last()
                    .zip(to.last())
                    .is_some_and(|(f, t)| common_case_fix(f, t))
                {
                    from.pop();
                    to.pop();
                }
            }
            if from.is_empty() || to.is_empty() || from.len() > 3 || to.len() > 3 {
                return None; // pure insert/delete or too long to be a name fix
            }
            let f = from.iter().map(|w| norm(w)).collect::<Vec<_>>().join(" ");
            let t_norm = to.iter().map(|w| norm(w)).collect::<Vec<_>>().join(" ");
            if f == t_norm {
                return None; // case/punctuation-only edit
            }
            let dist = levenshtein(&f, &t_norm) as f64
                / f.chars().count().max(t_norm.chars().count()).max(1) as f64;
            if dist > 0.6 {
                return None; // rewrite, not a respelling
            }
            let name_like = to.iter().any(|w| {
                let stripped = w.trim_matches(|c: char| !c.is_alphanumeric());
                stripped.chars().next().is_some_and(|c| c.is_uppercase())
            });
            if !name_like {
                return None;
            }
            let phrase = to
                .iter()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
                .collect::<Vec<_>>()
                .join(" ");
            (!phrase.is_empty()).then_some(phrase)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learns_corrected_multiword_name() {
        let learned = learned_phrases(
            "i met poorna shandra rao at the office",
            "i met Purna Chandra Rao at the office",
        );
        assert_eq!(learned, vec!["Purna Chandra Rao".to_string()]);
    }

    #[test]
    fn learns_single_corrected_name() {
        let learned = learned_phrases("ask krishna moorthy", "ask Krishna Murthy");
        assert_eq!(learned, vec!["Krishna Murthy".to_string()]);
    }

    #[test]
    fn ignores_pure_grammar_rewrites() {
        // Unrelated rewording is not a mishearing fix.
        let learned = learned_phrases("we should go there tomorrow", "we could visit the site");
        assert!(learned.is_empty());
    }

    #[test]
    fn trims_common_word_case_fix_adjacent_to_respelling() {
        let learned = learned_phrases(
            "i called shandra monday we met",
            "I called Chandra Monday we met",
        );
        assert_eq!(learned, vec!["Chandra".to_string()]);
    }

    #[test]
    fn trims_common_word_case_fix_across_punctuation() {
        let learned = learned_phrases(
            "i called shandra. monday we met",
            "I called Chandra. Monday we met",
        );
        assert_eq!(learned, vec!["Chandra".to_string()]);
    }

    #[test]
    fn ignores_case_only_and_lowercase_edits() {
        assert!(learned_phrases("i think so", "I think so").is_empty());
        assert!(learned_phrases("use the servor", "use the server").is_empty());
        // no capital → not name-like
    }
}
