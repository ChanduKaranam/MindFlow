use super::strategy::InjectionStrategy;

pub trait Clipboard { fn set_text(&mut self, text: &str) -> Result<(), String>; }
pub trait Paster { fn paste(&mut self) -> Result<(), String>; }

#[derive(Debug, PartialEq, Eq)]
pub enum Delivered { Pasted, ClipboardOnly }

/// Always put the text on the clipboard first (so it is never lost), then —
/// unless we're clipboard-only — attempt a paste. If the paste fails, the text
/// is still on the clipboard, so we report ClipboardOnly instead of erroring.
pub fn deliver_text(
    text: &str,
    strategy: InjectionStrategy,
    clip: &mut dyn Clipboard,
    paster: &mut dyn Paster,
) -> Result<Delivered, String> {
    clip.set_text(text)?; // clipboard failing is the one true error: nothing was delivered
    match strategy {
        InjectionStrategy::ClipboardOnly => Ok(Delivered::ClipboardOnly),
        InjectionStrategy::DirectPaste => match paster.paste() {
            Ok(()) => Ok(Delivered::Pasted),
            Err(_) => Ok(Delivered::ClipboardOnly),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SpyClipboard { last: Option<String>, fail: bool }
    impl Clipboard for SpyClipboard {
        fn set_text(&mut self, text: &str) -> Result<(), String> {
            if self.fail { return Err("clip fail".into()); }
            self.last = Some(text.to_string());
            Ok(())
        }
    }
    struct SpyPaster { called: bool, fail: bool }
    impl Paster for SpyPaster {
        fn paste(&mut self) -> Result<(), String> {
            self.called = true;
            if self.fail { return Err("paste fail".into()); }
            Ok(())
        }
    }

    #[test]
    fn direct_paste_sets_clipboard_then_pastes() {
        let mut clip = SpyClipboard { last: None, fail: false };
        let mut paster = SpyPaster { called: false, fail: false };
        let r = deliver_text("hello", InjectionStrategy::DirectPaste, &mut clip, &mut paster).unwrap();
        assert_eq!(r, Delivered::Pasted);
        assert_eq!(clip.last.as_deref(), Some("hello"));
        assert!(paster.called);
    }

    #[test]
    fn clipboard_only_never_pastes() {
        let mut clip = SpyClipboard { last: None, fail: false };
        let mut paster = SpyPaster { called: false, fail: false };
        let r = deliver_text("hi", InjectionStrategy::ClipboardOnly, &mut clip, &mut paster).unwrap();
        assert_eq!(r, Delivered::ClipboardOnly);
        assert_eq!(clip.last.as_deref(), Some("hi"));
        assert!(!paster.called);
    }

    #[test]
    fn paste_failure_degrades_to_clipboard_only() {
        let mut clip = SpyClipboard { last: None, fail: false };
        let mut paster = SpyPaster { called: false, fail: true };
        let r = deliver_text("x", InjectionStrategy::DirectPaste, &mut clip, &mut paster).unwrap();
        assert_eq!(r, Delivered::ClipboardOnly);
        assert_eq!(clip.last.as_deref(), Some("x"));
    }

    #[test]
    fn clipboard_failure_is_an_error() {
        let mut clip = SpyClipboard { last: None, fail: true };
        let mut paster = SpyPaster { called: false, fail: false };
        assert!(deliver_text("x", InjectionStrategy::DirectPaste, &mut clip, &mut paster).is_err());
    }
}
