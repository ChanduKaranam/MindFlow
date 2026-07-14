//! M9 privacy-safe context: optional, default-OFF sources (window title,
//! selection, clipboard) captured at recording start and injected into the
//! cleanup prompt as tagged data. Content never leaves the machine and is
//! never transcribed; history/audit records only which sources were used.

use crate::settings::AppSettings;
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Each source is truncated to this many chars — enough to resolve names and
/// terms, small enough to never dominate the prompt.
const MAX_SOURCE_CHARS: usize = 400;

fn truncate(s: &str) -> String {
    s.chars().take(MAX_SOURCE_CHARS).collect()
}

/// Context captured for one dictation, consumed by the cleanup pass.
pub struct CapturedContext {
    pub window_title: Option<String>,
    pub selection: Option<String>,
    pub clipboard: Option<String>,
}

impl CapturedContext {
    /// Which sources are present — for the "context-used" audit event.
    pub fn sources(&self) -> Vec<&'static str> {
        let mut s = Vec::new();
        if self.window_title.is_some() {
            s.push("window_title");
        }
        if self.selection.is_some() {
            s.push("selection");
        }
        if self.clipboard.is_some() {
            s.push("clipboard");
        }
        s
    }

    pub fn is_empty(&self) -> bool {
        self.window_title.is_none() && self.selection.is_none() && self.clipboard.is_none()
    }
}

/// Managed state: the context captured at recording start, taken by the
/// transcription pipeline when the dictation completes.
pub struct DictationContext(pub std::sync::Mutex<Option<CapturedContext>>);

/// Capture the enabled context sources. MUST run on the main thread —
/// selection capture simulates Ctrl+C via enigo.
pub fn capture(app: &tauri::AppHandle, settings: &AppSettings) -> CapturedContext {
    let window_title = if settings.context_window_title {
        active_win_pos_rs::get_active_window()
            .ok()
            .map(|w| w.title)
            .filter(|t| !t.trim().is_empty())
            .map(|t| truncate(&t))
    } else {
        None
    };
    // Ctrl+C into a terminal is SIGINT — the Code category covers terminals,
    // so selection capture is skipped there.
    let selection = if settings.context_selection
        && crate::context::detect_active_app_category() != crate::context::AppCategory::Code
    {
        crate::clipboard::capture_selection(app).map(|s| truncate(&s))
    } else {
        None
    };
    let clipboard = if settings.context_clipboard {
        app.clipboard()
            .read_text()
            .ok()
            .filter(|c| !c.trim().is_empty())
            .map(|c| truncate(&c))
    } else {
        None
    };
    CapturedContext {
        window_title,
        selection,
        clipboard,
    }
}

/// The prompt section appended to the cleanup system prompt for a non-empty
/// context. Pure for testability.
pub fn context_prompt_section(ctx: &CapturedContext) -> String {
    let mut s = String::from(
        "Context (from the user's screen — do NOT transcribe or repeat it, use it only to resolve names, spellings, and terms):\n",
    );
    if let Some(t) = &ctx.window_title {
        s.push_str(&format!("Window: {t}\n"));
    }
    if let Some(sel) = &ctx.selection {
        s.push_str(&format!("Selected text: {sel}\n"));
    }
    if let Some(c) = &ctx.clipboard {
        s.push_str(&format!("Clipboard: {c}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(w: Option<&str>, s: Option<&str>, c: Option<&str>) -> CapturedContext {
        CapturedContext {
            window_title: w.map(String::from),
            selection: s.map(String::from),
            clipboard: c.map(String::from),
        }
    }

    #[test]
    fn sources_and_is_empty() {
        let empty = ctx(None, None, None);
        assert!(empty.is_empty());
        assert!(empty.sources().is_empty());

        let full = ctx(Some("Inbox"), Some("hello"), Some("clip"));
        assert!(!full.is_empty());
        assert_eq!(
            full.sources(),
            vec!["window_title", "selection", "clipboard"]
        );

        let partial = ctx(None, Some("hello"), None);
        assert_eq!(partial.sources(), vec!["selection"]);
    }

    #[test]
    fn truncation_is_char_boundary_safe() {
        // Multi-byte chars: 500 'é' must truncate to 400 chars, not panic.
        let long: String = "é".repeat(500);
        let out = truncate(&long);
        assert_eq!(out.chars().count(), 400);
        let short = truncate("hello");
        assert_eq!(short, "hello");
    }

    #[test]
    fn prompt_section_lists_present_fields_only() {
        let c = ctx(Some("Inbox — Outlook"), None, Some("meeting notes"));
        let s = context_prompt_section(&c);
        assert!(s.starts_with("Context (from the user's screen"));
        assert!(s.contains("Window: Inbox — Outlook\n"));
        assert!(s.contains("Clipboard: meeting notes\n"));
        assert!(!s.contains("Selected text:"));
    }
}
