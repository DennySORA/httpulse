use crate::app::{AppState, ProfileViewMode, TargetPaneMode, TargetRuntime};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use super::super::format::{list_state, truncate_string};
use super::chart::draw_chart;
use super::panes::{draw_error_bar, draw_metrics_table, draw_network_info_pane, draw_summary_pane};

pub(in crate::features::ui) fn draw_main(frame: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(10)])
        .split(area);

    draw_target_list(frame, chunks[0], app);
    draw_target_panes(frame, chunks[1], app);
}

fn draw_target_list(frame: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let items: Vec<ListItem> = app
        .targets
        .iter()
        .enumerate()
        .map(|(idx, target)| {
            // Check if any profile has an error
            let has_error = target.profiles.iter().any(|p| p.last_error.is_some());

            let (status, status_style) = if target.paused {
                ("⏸", Style::default().fg(Color::Yellow))
            } else if has_error {
                ("⚠", Style::default().fg(Color::Red))
            } else {
                ("▶", Style::default().fg(Color::Green))
            };

            let is_selected = idx == app.selected_target;
            let line = Line::from(vec![
                Span::styled(format!(" {} ", status), status_style),
                Span::styled(
                    truncate_string(target.config.url.host_str().unwrap_or("?"), 24),
                    if is_selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else if has_error {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Targets ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("│");
    let mut state = list_state(app.selected_target);
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_target_panes(frame: &mut ratatui::Frame, area: Rect, app: &AppState) {
    if app.targets.is_empty() {
        let empty_lines = vec![
            Line::from(""),
            Line::styled(
                "  No targets configured  ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Press "),
                Span::styled(
                    " a ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" to add a target"),
            ]),
            Line::from(""),
            Line::styled(
                "  Example: https://google.com h2+tls13+warm",
                Style::default().fg(Color::DarkGray),
            ),
        ];
        let empty = Paragraph::new(empty_lines).block(
            Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );
        frame.render_widget(empty, area);
        return;
    }

    if let Some(target) = app.selected_target() {
        draw_target_pane(frame, area, app, target);
    }
}

fn draw_target_pane(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    target: &TargetRuntime,
) {
    // Check for errors
    let errors: Vec<_> = target
        .profiles
        .iter()
        .filter_map(|p| p.last_error.as_ref().map(|e| (&p.config.name, e)))
        .collect();
    let has_error = !errors.is_empty();
    let pane_mode = target.pane_mode;

    let status_indicator = if target.paused {
        "⏸ PAUSED"
    } else if has_error {
        "⚠ ERROR"
    } else {
        "▶ RUNNING"
    };
    let status_color = if target.paused {
        Color::Yellow
    } else if has_error {
        Color::Red
    } else {
        Color::Green
    };

    let view_mode_str = match target.view_mode {
        ProfileViewMode::Single => "Single",
        ProfileViewMode::Compare => "Compare",
    };

    let title = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            truncate_string(target.config.url.as_str(), 40),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(" │ "),
        Span::styled(status_indicator, Style::default().fg(status_color)),
        Span::raw(" │ "),
        Span::styled(view_mode_str, Style::default().fg(Color::Magenta)),
        Span::raw(" │ "),
        Span::styled(pane_mode.label(), Style::default().fg(Color::Yellow)),
        Span::raw(" "),
    ]);

    let border_color = if has_error { Color::Red } else { Color::Blue };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Draw based on pane mode
    match pane_mode {
        TargetPaneMode::Split => {
            // Split mode: [Summary+Stats | Metrics | Network Info] on top, Chart below
            // Calculate dynamic top row height based on available space
            let available_height = inner.height;
            let error_height: u16 = if has_error { 2 } else { 0 };
            let min_chart_height: u16 = 8;

            // Metrics table needs: header(1) + metrics(17) + categories(5) + separators(4) + borders(2) = 29
            // But we can show partial metrics with scrolling
            let ideal_metrics_height: u16 = 29;
            let remaining = available_height.saturating_sub(error_height + min_chart_height);

            // Use percentage-based or capped height for top row
            // Give 40-60% to top row depending on terminal height
            let top_row_height = if available_height >= 50 {
                // Large terminal: show more metrics
                remaining.min(ideal_metrics_height).max(12)
            } else if available_height >= 35 {
                // Medium terminal: balanced split
                (available_height * 45 / 100).max(12)
            } else {
                // Small terminal: minimum viable
                12
            };

            let mut v_constraints = vec![
                Constraint::Length(top_row_height),
                Constraint::Min(min_chart_height),
            ];
            if has_error {
                v_constraints.push(Constraint::Length(2));
            }
            let v_sections = Layout::default()
                .direction(Direction::Vertical)
                .constraints(v_constraints)
                .split(inner);

            // Top row: Summary | Metrics | Network Info
            let top_row = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(26), // Summary (with stats)
                    Constraint::Min(40),    // Metrics (flex)
                    Constraint::Length(28), // Network Info (combined)
                ])
                .split(v_sections[0]);

            draw_summary_pane(frame, top_row[0], app, target);
            draw_metrics_table(frame, top_row[1], app, target);
            draw_network_info_pane(frame, top_row[2], app, target);

            // Bottom: Chart
            draw_chart(frame, v_sections[1], app, target);

            // Error bar if needed
            if has_error {
                draw_error_bar(frame, v_sections[2], &errors);
            }
        }
        TargetPaneMode::Chart => {
            let mut constraints = vec![Constraint::Min(10)];
            if has_error {
                constraints.push(Constraint::Length(2));
            }
            let sections = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(inner);

            draw_chart(frame, sections[0], app, target);
            if has_error {
                draw_error_bar(frame, sections[1], &errors);
            }
        }
        TargetPaneMode::Metrics => {
            let mut constraints = vec![Constraint::Min(10)];
            if has_error {
                constraints.push(Constraint::Length(2));
            }
            let sections = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(inner);

            draw_metrics_table(frame, sections[0], app, target);
            if has_error {
                draw_error_bar(frame, sections[1], &errors);
            }
        }
        TargetPaneMode::Summary => {
            let mut constraints = vec![Constraint::Min(10)];
            if has_error {
                constraints.push(Constraint::Length(2));
            }
            let sections = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(inner);

            draw_summary_pane(frame, sections[0], app, target);
            if has_error {
                draw_error_bar(frame, sections[1], &errors);
            }
        }
    }
}
