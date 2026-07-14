//! M8 context awareness: detect the focused application and map it to a
//! category that adjusts the cleanup LLM's tone (Wispr Flow's per-app style).
//! Detection degrades to `Default` on any failure (e.g. Wayland without a
//! supported backend) — tone is a hint, never a dependency.

use active_win_pos_rs::get_active_window;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCategory {
    Email,
    Chat,
    Code,
    Notes,
    Default,
}

/// Focused app at this instant, categorized. Cheap enough to call per dictation.
// ponytail: captured at transcription end, not recording start — if the user
// alt-tabs during a long transcription the tone follows the new app; capture
// at recording start if that shows up in practice.
pub fn detect_active_app_category() -> AppCategory {
    match get_active_window() {
        Ok(win) => categorize(&win.app_name, &win.process_name()),
        Err(_) => AppCategory::Default,
    }
}

trait ProcessName {
    fn process_name(&self) -> String;
}
impl ProcessName for active_win_pos_rs::ActiveWindow {
    fn process_name(&self) -> String {
        self.process_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    }
}

/// Built-in process/app-name → category table (matched case-insensitively,
/// substring on app name, exact-ish on process stem).
pub fn categorize(app_name: &str, process_stem: &str) -> AppCategory {
    let hay = format!(
        "{} {}",
        app_name.to_lowercase(),
        process_stem.to_lowercase()
    );
    const EMAIL: &[&str] = &[
        "outlook",
        "thunderbird",
        "mailspring",
        "apple mail",
        " mail",
    ];
    const CHAT: &[&str] = &[
        "slack", "discord", "teams", "telegram", "whatsapp", "signal", "messages", "wechat",
    ];
    const CODE: &[&str] = &[
        "code",
        "cursor",
        "zed",
        "sublime",
        "intellij",
        "pycharm",
        "webstorm",
        "goland",
        "clion",
        "rider",
        "neovim",
        "nvim",
        "vim",
        "emacs",
        "terminal",
        "iterm",
        "alacritty",
        "kitty",
        "konsole",
        "wezterm",
        "powershell",
        "windowsterminal",
        "cmd",
        "warp",
        "ghostty",
    ];
    const NOTES: &[&str] = &[
        "notion", "obsidian", "onenote", "evernote", "logseq", "joplin",
    ];

    let hit = |list: &[&str]| list.iter().any(|k| hay.contains(k));
    if hit(EMAIL) {
        AppCategory::Email
    } else if hit(CHAT) {
        AppCategory::Chat
    } else if hit(CODE) {
        AppCategory::Code
    } else if hit(NOTES) {
        AppCategory::Notes
    } else {
        AppCategory::Default
    }
}

/// The prompt fragment a category contributes (empty for Default).
pub fn tone_rule(category: AppCategory) -> &'static str {
    match category {
        AppCategory::Email => {
            "- Tone: the user is writing an email — complete sentences, professional \
             punctuation and capitalization.\n"
        }
        AppCategory::Chat => {
            "- Tone: the user is writing a casual chat message — keep it light and \
             conversational; informal phrasing is fine, don't formalize it.\n"
        }
        AppCategory::Code => {
            "- Context: a code editor or terminal — treat the text as technical input; \
             keep identifiers, commands, and symbols verbatim, never prose-ify them.\n"
        }
        AppCategory::Notes => {
            "- Tone: the user is taking notes — concise phrasing; preserve list-like \
             structure.\n"
        }
        AppCategory::Default => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorize_common_apps() {
        assert_eq!(categorize("Slack", "slack"), AppCategory::Chat);
        assert_eq!(
            categorize("Microsoft Outlook", "OUTLOOK"),
            AppCategory::Email
        );
        assert_eq!(categorize("Visual Studio Code", "Code"), AppCategory::Code);
        assert_eq!(categorize("Obsidian", "obsidian"), AppCategory::Notes);
        assert_eq!(categorize("Some Game", "game"), AppCategory::Default);
    }

    #[test]
    fn default_contributes_no_rule() {
        assert!(tone_rule(AppCategory::Default).is_empty());
        assert!(!tone_rule(AppCategory::Email).is_empty());
    }
}
