use crate::app::AppState;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::super::format::{centered_rect, truncate_string};

pub(in crate::features::ui) fn draw_confirm_delete_popup(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
) {
    let popup_area = centered_rect(40, 25, area);
    frame.render_widget(Clear, popup_area);

    let target_name = app
        .selected_target()
        .map(|t| t.config.url.as_str())
        .unwrap_or("Unknown");

    let lines = vec![
        Line::from(""),
        Line::styled(
            "  Delete this target?  ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                truncate_string(target_name, 30),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Press "),
            Span::styled(" y ", Style::default().fg(Color::Black).bg(Color::Red)),
            Span::raw(" to delete, "),
            Span::styled(" n ", Style::default().fg(Color::Black).bg(Color::Green)),
            Span::raw(" to cancel"),
        ]),
    ];

    let popup = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Confirm Delete ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .style(Style::default().bg(Color::Black));

    frame.render_widget(popup, popup_area);
}
