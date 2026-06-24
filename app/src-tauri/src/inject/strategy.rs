#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os { Windows, MacOs, Linux }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionEnv { pub os: Os, pub is_wayland: bool }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionStrategy { DirectPaste, ClipboardOnly }

/// Choose how to deliver text. Wayland blocks synthetic key events into other
/// apps reliably, so we degrade to leaving text on the clipboard there.
pub fn select_strategy(env: &SessionEnv) -> InjectionStrategy {
    match env {
        SessionEnv { os: Os::Linux, is_wayland: true } => InjectionStrategy::ClipboardOnly,
        _ => InjectionStrategy::DirectPaste,
    }
}

/// Detect the running session. `is_wayland` is only meaningful on Linux.
#[allow(dead_code)]
pub fn detect_session() -> SessionEnv {
    #[cfg(target_os = "windows")]
    let os = Os::Windows;
    #[cfg(target_os = "macos")]
    let os = Os::MacOs;
    #[cfg(target_os = "linux")]
    let os = Os::Linux;

    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").map(|v| v == "wayland").unwrap_or(false);

    SessionEnv { os, is_wayland }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_uses_direct_paste() {
        let env = SessionEnv { os: Os::Windows, is_wayland: false };
        assert_eq!(select_strategy(&env), InjectionStrategy::DirectPaste);
    }

    #[test]
    fn macos_uses_direct_paste() {
        let env = SessionEnv { os: Os::MacOs, is_wayland: false };
        assert_eq!(select_strategy(&env), InjectionStrategy::DirectPaste);
    }

    #[test]
    fn linux_x11_uses_direct_paste() {
        let env = SessionEnv { os: Os::Linux, is_wayland: false };
        assert_eq!(select_strategy(&env), InjectionStrategy::DirectPaste);
    }

    #[test]
    fn linux_wayland_falls_back_to_clipboard_only() {
        let env = SessionEnv { os: Os::Linux, is_wayland: true };
        assert_eq!(select_strategy(&env), InjectionStrategy::ClipboardOnly);
    }
}
