use crate::app::AppState;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Padding, Paragraph, Row, Table, TableState};

use super::super::state::{InputMode, SettingsField, SettingsRow, SettingsState};
use super::format::{centered_rect, truncate_string};

pub(in crate::features::ui) fn draw_settings_popup(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    settings_state: &SettingsState,
    input_mode: InputMode,
    input_buffer: &str,
) {
    let popup_area = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup_area);

    let rows = settings_rows(app);
    let mut table_state = TableState::default();
    let mut selected = settings_state.selected;
    if rows.is_empty() {
        table_state.select(None);
    } else {
        if selected >= rows.len() {
            selected = rows.len().saturating_sub(1);
        }
        table_state.select(Some(selected));
    }

    let title = app
        .selected_target()
        .map(|target| {
            format!(
                " Settings - {} ",
                truncate_string(target.config.url.as_str(), 30)
            )
        })
        .unwrap_or_else(|| " Settings ".to_string());

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut constraints = vec![Constraint::Min(6), Constraint::Length(2)];
    if matches!(input_mode, InputMode::SettingsEdit(_)) {
        constraints.push(Constraint::Length(3));
    }
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let header = Row::new(vec![
        Cell::from("Scope"),
        Cell::from("Setting"),
        Cell::from("Value"),
        Cell::from("Action"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let table_rows = rows.iter().map(|row| {
        Row::new(vec![
            Cell::from(row.scope),
            Cell::from(row.label),
            Cell::from(row.value.clone()),
            Cell::from(row.action),
        ])
    });

    let widths = [
        Constraint::Length(8),
        Constraint::Length(18),
        Constraint::Length(18),
        Constraint::Min(12),
    ];

    let table = Table::new(table_rows, widths)
        .header(header)
        .column_spacing(1)
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_widget(Clear, sections[0]);
    frame.render_stateful_widget(table, sections[0], &mut table_state);

    let mut help_lines = vec![Line::from(vec![
        Span::styled("  ↑↓ ", Style::default().fg(Color::Green)),
        Span::raw("Select  "),
        Span::styled("Enter ", Style::default().fg(Color::Green)),
        Span::raw("Edit/Toggle  "),
        Span::styled("Esc ", Style::default().fg(Color::Green)),
        Span::raw("Close"),
    ])];
    if let Some(notice) = &settings_state.notice {
        help_lines.push(Line::styled(
            format!("  {notice}"),
            Style::default().fg(Color::Red),
        ));
    }

    let help = Paragraph::new(help_lines).style(Style::default().bg(Color::Black));
    frame.render_widget(help, sections[1]);

    if let InputMode::SettingsEdit(field) = input_mode {
        let prompt = settings_edit_prompt(field);
        let input_line = Line::from(vec![
            Span::styled(format!("  {prompt}"), Style::default().fg(Color::Yellow)),
            Span::raw(input_buffer),
            Span::styled("█", Style::default().fg(Color::Gray)),
        ]);
        let input = Paragraph::new(input_line).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(input, sections[2]);
    }
}

pub(in crate::features::ui) fn settings_rows(app: &AppState) -> Vec<SettingsRow> {
    let mut rows = Vec::new();
    rows.push(SettingsRow {
        field: SettingsField::UiRefreshHz,
        scope: "Global",
        label: "UI refresh",
        value: format!("{} Hz", app.global.ui_refresh_hz),
        action: "Enter to edit",
    });
    rows.push(SettingsRow {
        field: SettingsField::LinkCapacityMbps,
        scope: "Global",
        label: "Link capacity",
        value: app
            .global
            .link_capacity_mbps
            .map(|value| format!("{value:.1} Mbps"))
            .unwrap_or_else(|| "Off".to_string()),
        action: "Enter to edit",
    });

    if let Some(target) = app.selected_target() {
        rows.push(SettingsRow {
            field: SettingsField::TargetInterval,
            scope: "Target",
            label: "Interval",
            value: format!("{}s", target.config.interval.as_secs()),
            action: "Enter to edit",
        });
        rows.push(SettingsRow {
            field: SettingsField::TargetTimeout,
            scope: "Target",
            label: "Timeout",
            value: format!("{}s", target.config.timeout_total.as_secs()),
            action: "Enter to edit",
        });
        rows.push(SettingsRow {
            field: SettingsField::TargetDnsEnabled,
            scope: "Target",
            label: "DNS",
            value: if target.config.dns_enabled {
                "On".to_string()
            } else {
                "Off".to_string()
            },
            action: "Enter to toggle",
        });
        rows.push(SettingsRow {
            field: SettingsField::TargetPane,
            scope: "Target",
            label: "Pane",
            value: target.pane_mode.label().to_string(),
            action: "Enter to cycle",
        });
        rows.push(SettingsRow {
            field: SettingsField::TargetPaused,
            scope: "Target",
            label: "Status",
            value: if target.paused {
                "Paused".to_string()
            } else {
                "Running".to_string()
            },
            action: "Enter to toggle",
        });
    }

    rows
}

pub(super) fn settings_edit_prompt(field: SettingsField) -> &'static str {
    match field {
        SettingsField::UiRefreshHz => "Set UI refresh (Hz): ",
        SettingsField::LinkCapacityMbps => "Set link capacity Mbps (blank=off): ",
        SettingsField::TargetInterval => "Set probe interval (e.g. 5s): ",
        SettingsField::TargetTimeout => "Set timeout (e.g. 10s): ",
        SettingsField::TargetDnsEnabled
        | SettingsField::TargetPane
        | SettingsField::TargetPaused => "Press Enter to toggle: ",
    }
}

pub(in crate::features::ui) fn seed_settings_input(app: &AppState, field: SettingsField) -> String {
    match field {
        SettingsField::UiRefreshHz => app.global.ui_refresh_hz.to_string(),
        SettingsField::LinkCapacityMbps => app
            .global
            .link_capacity_mbps
            .map(|value| format!("{value:.1}"))
            .unwrap_or_default(),
        SettingsField::TargetInterval => app
            .selected_target()
            .map(|target| format!("{}s", target.config.interval.as_secs()))
            .unwrap_or_default(),
        SettingsField::TargetTimeout => app
            .selected_target()
            .map(|target| format!("{}s", target.config.timeout_total.as_secs()))
            .unwrap_or_default(),
        SettingsField::TargetDnsEnabled
        | SettingsField::TargetPane
        | SettingsField::TargetPaused => String::new(),
    }
}
