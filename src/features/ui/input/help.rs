use crossterm::event::{KeyCode, KeyEvent};

use super::super::state::{GLOSSARY_PAGE_COUNT, InputMode};

pub(in crate::features::ui) fn handle_help_key(key: KeyEvent, input_mode: &mut InputMode) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            *input_mode = InputMode::Normal;
        }
        _ => {}
    }
}

pub(in crate::features::ui) fn handle_glossary_key(
    key: KeyEvent,
    input_mode: &mut InputMode,
    glossary_page: &mut usize,
) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('G') => {
            *input_mode = InputMode::Normal;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            *glossary_page = glossary_page.saturating_sub(1);
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if *glossary_page + 1 < GLOSSARY_PAGE_COUNT {
                *glossary_page += 1;
            }
        }
        KeyCode::Char('1') => *glossary_page = 0,
        KeyCode::Char('2') => *glossary_page = 1,
        KeyCode::Char('3') => *glossary_page = 2,
        _ => {}
    }
}
