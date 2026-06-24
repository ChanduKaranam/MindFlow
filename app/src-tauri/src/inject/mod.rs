pub mod strategy;
pub mod deliver;
pub mod adapters;

use deliver::{deliver_text, Delivered};
use strategy::{detect_session, select_strategy};

/// Resolve the strategy for this session, then deliver `text`, borrowing
/// Handy's managed Enigo for the paste step.
pub fn deliver_now(app: &tauri::AppHandle, text: &str) -> Result<Delivered, String> {
    use tauri::Manager;
    let strategy = select_strategy(&detect_session());
    let enigo_state = app
        .try_state::<crate::input::EnigoState>()
        .ok_or_else(|| "Enigo state unavailable".to_string())?;
    let mut enigo = enigo_state.0.lock().map_err(|e| e.to_string())?;
    let mut clip = adapters::ArboardClipboard::new()?;
    let mut paster = adapters::EnigoPaster { enigo: &mut enigo };
    deliver_text(text, strategy, &mut clip, &mut paster)
}
