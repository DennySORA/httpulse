use crate::app::{AppState, MetricsCategory, ProfileViewMode, TargetRuntime};
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use super::super::super::format::{color_for_index, format_stat_triplet, metrics_for_category};

pub(in crate::features::ui) fn draw_metrics_table(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    target: &TargetRuntime,
) {
    let profiles: Vec<_> = match target.view_mode {
        ProfileViewMode::Single => target
            .profiles
            .get(target.selected_profile)
            .into_iter()
            .collect(),
        ProfileViewMode::Compare => target.profiles.iter().collect(),
    };

    // Build category tabs
    let tab_spans: Vec<Span> = MetricsCategory::ALL
        .iter()
        .enumerate()
        .flat_map(|(i, cat)| {
            let is_selected = *cat == target.metrics_category;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let mut spans = vec![Span::styled(format!(" {} ", cat.label()), style)];
            if i < MetricsCategory::ALL.len() - 1 {
                spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
            }
            spans
        })
        .collect();
    let tabs_line = Line::from(tab_spans);

    // Build header row
    let mut header_cells: Vec<Line> = vec![Line::from(Span::styled(
        "Metric",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))];
    for (idx, profile) in profiles.iter().enumerate() {
        let color = color_for_index(idx);
        header_cells.push(Line::from(vec![
            Span::styled("■ ", Style::default().fg(color)),
            Span::styled(profile.config.name.clone(), Style::default().fg(color)),
        ]));
    }
    let header = Row::new(header_cells).style(Style::default().add_modifier(Modifier::BOLD));

    // Build metric rows for selected category
    let metrics = metrics_for_category(target.metrics_category);
    let rows: Vec<Row> = metrics
        .iter()
        .map(|&metric| {
            let is_selected = app.selected_metrics.contains(&metric);
            let metric_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let unit = metric.unit();
            let label_with_unit = if unit.is_empty() {
                metric.label().to_string()
            } else {
                format!("{} ({})", metric.label(), unit)
            };

            let mut cells: Vec<Cell> = Vec::new();
            cells.push(Cell::from(Span::styled(label_with_unit, metric_style)));
            for profile in &profiles {
                let aggregate = app.target_aggregate(target, profile);
                let stats = aggregate.by_metric.get(&metric);
                cells.push(Cell::from(format_stat_triplet(metric, stats)));
            }
            Row::new(cells)
        })
        .collect();

    let widths: Vec<Constraint> = std::iter::once(Constraint::Length(18))
        .chain(profiles.iter().map(|_| Constraint::Length(18)))
        .collect();

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .title(" Metrics (P50/P99/Mean) ")
            .title_bottom(tabs_line.alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(table, area);
}
