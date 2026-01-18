use crate::app::{AppState, TargetRuntime};
use crate::metrics::MetricKind;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use super::super::super::format::{
    format_count, format_goodput, format_latency, style_for_latency, style_for_success_rate,
    style_for_timeout_count,
};

pub(in crate::features::ui) fn draw_summary_pane(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    target: &TargetRuntime,
) {
    let summary = app.target_summary(target);

    let success_rate = if summary.requests == 0 {
        0.0
    } else {
        summary.successes as f64 / summary.requests as f64 * 100.0
    };

    let mut rows = vec![
        Row::new(vec![
            Cell::from("Requests"),
            Cell::from(format_count(summary.requests)),
        ]),
        Row::new(vec![
            Cell::from("Success"),
            Cell::from(format!("{success_rate:.1}%")).style(style_for_success_rate(success_rate)),
        ]),
        Row::new(vec![
            Cell::from("Timeouts"),
            Cell::from(format_count(summary.timeouts))
                .style(style_for_timeout_count(summary.timeouts)),
        ]),
    ];

    // Add latency stats
    if let Some(profile) = target.profiles.get(target.selected_profile) {
        let aggregate = app.target_aggregate(target, profile);
        if let Some(stats) = aggregate.by_metric.get(&MetricKind::Total) {
            if let Some(p50) = stats.p50 {
                rows.push(Row::new(vec![
                    Cell::from("Latency P50"),
                    Cell::from(format_latency(p50)).style(style_for_latency(p50)),
                ]));
            }
            if let Some(p99) = stats.p99 {
                rows.push(Row::new(vec![
                    Cell::from("Latency P99"),
                    Cell::from(format_latency(p99)).style(style_for_latency(p99)),
                ]));
            }
        }
    }

    // Add goodput stats
    if let Some(profile) = target.profiles.get(target.selected_profile) {
        let aggregate = app.target_aggregate(target, profile);
        let goodput_stats = aggregate.by_metric.get(&MetricKind::GoodputBps).cloned();
        if let Some(stats) = &goodput_stats
            && let Some(mean) = stats.mean
        {
            rows.push(Row::new(vec![
                Cell::from("Goodput"),
                Cell::from(format_goodput(mean)),
            ]));
        }
    }

    // Add error breakdown (compact)
    let total_errors: u64 = summary.errors.values().sum();
    if total_errors > 0 {
        let error_summary: String = summary
            .errors
            .iter()
            .take(2)
            .map(|(e, c)| format!("{}:{}", e.short_label(), c))
            .collect::<Vec<_>>()
            .join(" ");
        rows.push(Row::new(vec![
            Cell::from("Errors").style(Style::default().fg(Color::Red)),
            Cell::from(error_summary).style(Style::default().fg(Color::Red)),
        ]));
    }

    let widths = [
        ratatui::layout::Constraint::Length(12),
        ratatui::layout::Constraint::Min(12),
    ];

    let table = Table::new(rows, widths).column_spacing(1).block(
        Block::default()
            .title(format!(" Summary [{}] ", app.window.label()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(table, area);
}
