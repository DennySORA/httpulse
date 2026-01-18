use crate::app::{AppState, apply_edit_command};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::render::{seed_settings_input, settings_rows};
use super::super::state::{InputMode, SettingsField, SettingsState, parse_link_capacity_mbps};

pub(in crate::features::ui) fn handle_settings_key(
    key: KeyEvent,
    app: &mut AppState,
    input_mode: &mut InputMode,
    input_buffer: &mut String,
    settings_state: &mut SettingsState,
) {
    let rows = settings_rows(app);
    settings_state.clamp(rows.len());

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('S') => {
            *input_mode = InputMode::Normal;
            settings_state.clear_notice();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            settings_state.select_next(rows.len());
            settings_state.clear_notice();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            settings_state.select_prev(rows.len());
            settings_state.clear_notice();
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            settings_state.clear_notice();
            if let Some(row) = rows.get(settings_state.selected) {
                match row.field {
                    SettingsField::TargetDnsEnabled => {
                        if let Some(target) = app.selected_target() {
                            let mut updated = target.config.clone();
                            updated.dns_enabled = !updated.dns_enabled;
                            app.update_target_config(app.selected_target, updated);
                        }
                    }
                    SettingsField::TargetPane => {
                        app.cycle_pane_mode(app.selected_target);
                    }
                    SettingsField::TargetPaused => {
                        app.toggle_pause(app.selected_target);
                    }
                    SettingsField::UiRefreshHz
                    | SettingsField::LinkCapacityMbps
                    | SettingsField::TargetInterval
                    | SettingsField::TargetTimeout => {
                        *input_mode = InputMode::SettingsEdit(row.field);
                        input_buffer.clear();
                        input_buffer.push_str(&seed_settings_input(app, row.field));
                    }
                }
            }
        }
        _ => {}
    }
}

pub(in crate::features::ui) fn handle_settings_edit_key(
    key: KeyEvent,
    app: &mut AppState,
    input_mode: &mut InputMode,
    input_buffer: &mut String,
    field: SettingsField,
    settings_state: &mut SettingsState,
) {
    match key.code {
        KeyCode::Esc => {
            *input_mode = InputMode::Settings;
            input_buffer.clear();
            settings_state.clear_notice();
        }
        KeyCode::Enter => {
            let trimmed = input_buffer.trim();
            let mut applied = false;
            settings_state.clear_notice();
            match field {
                SettingsField::UiRefreshHz => {
                    if let Ok(value) = trimmed.parse::<u16>() {
                        if value > 0 {
                            app.global.ui_refresh_hz = value;
                            applied = true;
                        } else {
                            settings_state.notice = Some("Refresh must be > 0".to_string());
                        }
                    } else {
                        settings_state.notice = Some("Invalid refresh value".to_string());
                    }
                }
                SettingsField::LinkCapacityMbps => match parse_link_capacity_mbps(trimmed) {
                    Ok(value) => {
                        app.global.link_capacity_mbps = value;
                        applied = true;
                    }
                    Err(message) => {
                        settings_state.notice = Some(message.to_string());
                    }
                },
                SettingsField::TargetInterval => {
                    if let Some(target) = app.selected_target() {
                        let command = format!("interval={trimmed}");
                        if let Some(updated) = apply_edit_command(target, &command) {
                            app.update_target_config(app.selected_target, updated);
                            applied = true;
                        } else {
                            settings_state.notice = Some("Invalid interval value".to_string());
                        }
                    }
                }
                SettingsField::TargetTimeout => {
                    if let Some(target) = app.selected_target() {
                        let command = format!("timeout={trimmed}");
                        if let Some(updated) = apply_edit_command(target, &command) {
                            app.update_target_config(app.selected_target, updated);
                            applied = true;
                        } else {
                            settings_state.notice = Some("Invalid timeout value".to_string());
                        }
                    }
                }
                SettingsField::TargetDnsEnabled
                | SettingsField::TargetPane
                | SettingsField::TargetPaused => {}
            }

            if applied {
                *input_mode = InputMode::Settings;
                input_buffer.clear();
            }
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
