use crate::app::{AppState, parse_profile_specs, parse_target_url};
use crate::probe::ProbeSample;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use url::Url;

use super::super::state::InputMode;

pub(in crate::features::ui) fn handle_input_key(
    key: KeyEvent,
    app: &mut AppState,
    input_mode: &mut InputMode,
    input_buffer: &mut String,
    sample_tx: &crossbeam_channel::Sender<ProbeSample>,
) {
    match key.code {
        KeyCode::Esc => {
            *input_mode = InputMode::Normal;
            input_buffer.clear();
        }
        KeyCode::Enter => {
            match *input_mode {
                InputMode::AddTarget => {
                    if let Some((url, profiles)) = parse_add_command(input_buffer) {
                        app.add_target(url, profiles, sample_tx.clone());
                    }
                }
                InputMode::Normal
                | InputMode::Help
                | InputMode::Glossary
                | InputMode::Settings
                | InputMode::SettingsEdit(_)
                | InputMode::ConfirmDelete => {}
            }
            *input_mode = InputMode::Normal;
            input_buffer.clear();
        }
        KeyCode::Backspace => {
            input_buffer.pop();
        }
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return;
            }
            input_buffer.push(ch);
        }
        _ => {}
    }
}

fn parse_add_command(input: &str) -> Option<(Url, Option<Vec<crate::config::ProfileConfig>>)> {
    let mut parts = input.split_whitespace();
    let url_text = parts.next()?;
    let url = parse_target_url(url_text)?;
    let rest = parts.collect::<Vec<_>>().join(" ");
    if rest.is_empty() {
        Some((url, None))
    } else {
        Some((url, Some(parse_profile_specs(&rest))))
    }
}
