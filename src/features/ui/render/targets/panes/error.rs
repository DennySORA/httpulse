use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::super::super::format::truncate_string;

pub(in crate::features::ui) fn draw_error_bar(
    frame: &mut ratatui::Frame,
    area: Rect,
    errors: &[(&String, &crate::probe::ProbeErrorKind)],
) {
    let error_msg: String = errors
        .iter()
        .map(|(name, err)| format!("{}: {}", name, err.short_label()))
        .collect::<Vec<_>>()
        .join(" | ");
    let error_line = Line::from(vec![
        Span::styled(" ⚠ ", Style::default().fg(Color::Red)),
        Span::styled(
            truncate_string(&error_msg, 60),
            Style::default().fg(Color::Red),
        ),
    ]);
    let error_para = Paragraph::new(error_line).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(error_para, area);
}
