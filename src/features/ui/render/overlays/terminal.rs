use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::super::super::state::{MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH};

/// Draw a warning when terminal is too small
pub(in crate::features::ui) fn draw_terminal_too_small(frame: &mut ratatui::Frame, area: Rect) {
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::from(""),
        Line::styled(
            "Terminal Too Small",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        Line::from(vec![
            Span::raw("Current: "),
            Span::styled(
                format!("{}x{}", area.width, area.height),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Minimum: "),
            Span::styled(
                format!("{}x{}", MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(""),
        Line::styled(
            "Please resize your terminal",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(" httpulse"),
    );

    frame.render_widget(paragraph, area);
}
