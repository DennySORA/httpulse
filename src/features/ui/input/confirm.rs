use crate::app::AppState;
use crossterm::event::{KeyCode, KeyEvent};

use super::super::state::InputMode;

pub(in crate::features::ui) fn handle_confirm_delete_key(
    key: KeyEvent,
    app: &mut AppState,
    input_mode: &mut InputMode,
) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.remove_target(app.selected_target);
            *input_mode = InputMode::Normal;
        }
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            *input_mode = InputMode::Normal;
        }
        _ => {}
    }
}
