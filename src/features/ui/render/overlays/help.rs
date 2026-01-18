use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap};

use super::super::format::centered_rect;

pub(in crate::features::ui) fn draw_help_popup(frame: &mut ratatui::Frame, area: Rect) {
    let popup_area = centered_rect(60, 80, area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(vec![Span::styled(
            "  Keyboard Shortcuts  ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::styled("─── Navigation ───", Style::default().fg(Color::Yellow)),
        Line::from(vec![
            Span::styled("  Up/Down, j/k  ", Style::default().fg(Color::Green)),
            Span::raw("Select target"),
        ]),
        Line::from(vec![
            Span::styled("  Tab       ", Style::default().fg(Color::Green)),
            Span::raw("Cycle through profiles"),
        ]),
        Line::from(""),
        Line::styled("─── Target Actions ───", Style::default().fg(Color::Yellow)),
        Line::from(vec![
            Span::styled("  a         ", Style::default().fg(Color::Green)),
            Span::raw("Add new target"),
        ]),
        Line::from(vec![
            Span::styled("  d         ", Style::default().fg(Color::Green)),
            Span::raw("Delete selected target"),
        ]),
        Line::from(vec![
            Span::styled("  e         ", Style::default().fg(Color::Green)),
            Span::raw("Edit target (Settings)"),
        ]),
        Line::from(vec![
            Span::styled("  p         ", Style::default().fg(Color::Green)),
            Span::raw("Pause/Resume probing"),
        ]),
        Line::from(""),
        Line::styled("─── View Options ───", Style::default().fg(Color::Yellow)),
        Line::from(vec![
            Span::styled("  c         ", Style::default().fg(Color::Green)),
            Span::raw("Toggle compare mode"),
        ]),
        Line::from(vec![
            Span::styled("  g         ", Style::default().fg(Color::Green)),
            Span::raw("Cycle right pane (Split/Chart/Metrics)"),
        ]),
        Line::from(vec![
            Span::styled("  w         ", Style::default().fg(Color::Green)),
            Span::raw("Cycle time window (1m/5m/15m/60m)"),
        ]),
        Line::from(vec![
            Span::styled("  1-8       ", Style::default().fg(Color::Green)),
            Span::raw("Toggle metric series on chart"),
        ]),
        Line::from(""),
        Line::styled("─── Metrics (1-8) ───", Style::default().fg(Color::Yellow)),
        Line::from(vec![
            Span::styled("  1 ", Style::default().fg(Color::Green)),
            Span::raw("Total  "),
            Span::styled("2 ", Style::default().fg(Color::Green)),
            Span::raw("DNS  "),
            Span::styled("3 ", Style::default().fg(Color::Green)),
            Span::raw("Connect  "),
            Span::styled("4 ", Style::default().fg(Color::Green)),
            Span::raw("TLS"),
        ]),
        Line::from(vec![
            Span::styled("  5 ", Style::default().fg(Color::Green)),
            Span::raw("TTFB   "),
            Span::styled("6 ", Style::default().fg(Color::Green)),
            Span::raw("Download  "),
            Span::styled("7 ", Style::default().fg(Color::Green)),
            Span::raw("RTT  "),
            Span::styled("8 ", Style::default().fg(Color::Green)),
            Span::raw("Retrans"),
        ]),
        Line::from(""),
        Line::styled("─── General ───", Style::default().fg(Color::Yellow)),
        Line::from(vec![
            Span::styled("  ?         ", Style::default().fg(Color::Green)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("  S         ", Style::default().fg(Color::Green)),
            Span::raw("Open settings"),
        ]),
        Line::from(vec![
            Span::styled("  q/Ctrl+C  ", Style::default().fg(Color::Green)),
            Span::raw("Quit application"),
        ]),
        Line::from(""),
        Line::styled(
            "  Press Esc or ? to close  ",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .padding(Padding::horizontal(1)),
        )
        .style(Style::default().bg(Color::Black))
        .wrap(Wrap { trim: false });

    frame.render_widget(help, popup_area);
}
