use crate::app::AppState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::super::state::InputMode;

pub(in crate::features::ui) fn draw_header(frame: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let selected_metrics: Vec<_> = app.selected_metrics.iter().map(|m| m.label()).collect();
    let metrics_str = if selected_metrics.is_empty() {
        "none".to_string()
    } else {
        selected_metrics.join(",")
    };

    let header = Line::from(vec![
        Span::styled(
            " httpulse",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("│ "),
        Span::styled("Window:", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {} ", app.window.label()),
            Style::default().fg(Color::Green),
        ),
        Span::raw("│ "),
        Span::styled("Stats:", Style::default().fg(Color::DarkGray)),
        Span::styled(" P50/P99/Mean ", Style::default().fg(Color::Yellow)),
        Span::raw("│ "),
        Span::styled("Metrics:", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {} ", metrics_str),
            Style::default().fg(Color::Magenta),
        ),
        Span::raw("│ "),
        Span::styled("Targets:", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {} ", app.targets.len()),
            Style::default().fg(Color::White),
        ),
    ]);

    let paragraph = Paragraph::new(header).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}

pub(in crate::features::ui) fn draw_footer(
    frame: &mut ratatui::Frame,
    area: Rect,
    mode: InputMode,
) {
    let hints = match mode {
        InputMode::Normal => vec![
            ("q", "Quit"),
            ("?", "Help"),
            ("G", "Glossary"),
            ("S", "Settings"),
            ("a", "Add"),
            ("d", "Delete"),
            ("p", "Pause"),
            ("c", "Compare"),
            ("g", "Pane"),
            ("w", "Window"),
            ("[ ]", "Category"),
        ],
        InputMode::AddTarget => vec![("Enter", "Confirm"), ("Esc", "Cancel")],
        InputMode::Help | InputMode::Glossary => vec![("Esc", "Close")],
        InputMode::Settings => vec![
            ("Enter", "Edit/Toggle"),
            ("↑↓", "Navigate"),
            ("Esc", "Close"),
        ],
        InputMode::SettingsEdit(_) => vec![("Enter", "Apply"), ("Esc", "Cancel")],
        InputMode::ConfirmDelete => vec![("y", "Delete"), ("n", "Cancel")],
    };

    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|(key, action)| {
            vec![
                Span::styled(format!(" {key} "), Style::default().fg(Color::Yellow)),
                Span::styled(format!("{action} "), Style::default().fg(Color::Gray)),
            ]
        })
        .collect();

    let footer = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(footer, area);
}
