use super::deliver::{Clipboard, Paster};

pub struct ArboardClipboard {
    inner: arboard::Clipboard,
}
impl ArboardClipboard {
    pub fn new() -> Result<Self, String> {
        Ok(Self { inner: arboard::Clipboard::new().map_err(|e| e.to_string())? })
    }
}
impl Clipboard for ArboardClipboard {
    fn set_text(&mut self, text: &str) -> Result<(), String> {
        self.inner.set_text(text.to_string()).map_err(|e| e.to_string())
    }
}

pub struct EnigoPaster<'a> { pub enigo: &'a mut enigo::Enigo }
impl<'a> Paster for EnigoPaster<'a> {
    fn paste(&mut self) -> Result<(), String> {
        crate::input::send_paste_ctrl_v(self.enigo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arboard_clipboard_constructs_or_reports_error() {
        // On headless CI there may be no clipboard; either Ok or a String error is acceptable.
        match ArboardClipboard::new() {
            Ok(_) => {}
            Err(e) => assert!(!e.is_empty()),
        }
    }
}
