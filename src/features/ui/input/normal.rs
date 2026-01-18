use crate::app::{AppState, ProfileViewMode};
use crate::metrics::MetricKind;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::state::{InputMode, SettingsState};

pub(in crate::features::ui) fn handle_normal_key(
    key: KeyEvent,
    app: &mut AppState,
    input_mode: &mut InputMode,
    input_buffer: &mut String,
    settings_state: &mut SettingsState,
    glossary_page: &mut usize,
) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }
    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('?') => {
            *input_mode = InputMode::Help;
        }
        KeyCode::Char('G') => {
            *input_mode = InputMode::Glossary;
            *glossary_page = 0;
        }
        KeyCode::Char('S') => {
            *input_mode = InputMode::Settings;
            settings_state.selected = 0;
            settings_state.clear_notice();
        }
        KeyCode::Char('a') => {
            *input_mode = InputMode::AddTarget;
            input_buffer.clear();
        }
        KeyCode::Char('e') => {
            *input_mode = InputMode::Settings;
            settings_state.selected = 0;
            settings_state.clear_notice();
        }
        KeyCode::Char('d') => {
            if !app.targets.is_empty() {
                *input_mode = InputMode::ConfirmDelete;
            }
        }
        KeyCode::Char('p') => {
            app.toggle_pause(app.selected_target);
        }
        KeyCode::Char('c') => {
            if let Some(target) = app.selected_target_mut() {
                target.view_mode = match target.view_mode {
                    ProfileViewMode::Single => ProfileViewMode::Compare,
                    ProfileViewMode::Compare => ProfileViewMode::Single,
                };
            }
        }
        KeyCode::Char('g') => app.cycle_pane_mode(app.selected_target),
        KeyCode::Char('w') => app.cycle_window(),
        KeyCode::Down | KeyCode::Char('j') => {
            if app.selected_target + 1 < app.targets.len() {
                app.selected_target += 1;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected_target = app.selected_target.saturating_sub(1);
        }
        KeyCode::Tab => {
            if let Some(target) = app.selected_target_mut()
                && !target.profiles.is_empty()
            {
                target.selected_profile = (target.selected_profile + 1) % target.profiles.len();
            }
        }
        KeyCode::Char('1') => app.toggle_metric(MetricKind::Total),
        KeyCode::Char('2') => app.toggle_metric(MetricKind::Dns),
        KeyCode::Char('3') => app.toggle_metric(MetricKind::Connect),
        KeyCode::Char('4') => app.toggle_metric(MetricKind::Tls),
        KeyCode::Char('5') => app.toggle_metric(MetricKind::Ttfb),
        KeyCode::Char('6') => app.toggle_metric(MetricKind::Download),
        KeyCode::Char('7') => app.toggle_metric(MetricKind::Rtt),
        KeyCode::Char('8') => app.toggle_metric(MetricKind::Retrans),
        // Metrics category navigation
        KeyCode::Char(']') => {
            if let Some(target) = app.selected_target_mut() {
                target.metrics_category = target.metrics_category.next();
            }
        }
        KeyCode::Char('[') => {
            if let Some(target) = app.selected_target_mut() {
                target.metrics_category = target.metrics_category.prev();
            }
        }
        _ => {}
    }
    false
}
