use crate::app::{AppState, ProfileViewMode, TargetRuntime};
use crate::metrics::MetricKind;
use crate::metrics_aggregate::ProfileKey;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Chart, Clear, Dataset, GraphType};

use super::super::format::{color_for_index, format_y_axis_labels, update_bounds};

struct SeriesSpec {
    name: String,
    color: Color,
    points: Vec<(f64, f64)>,
}

pub(super) fn draw_chart(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    target: &TargetRuntime,
) {
    frame.render_widget(Clear, area);

    let window_seconds = app.window.duration().as_secs_f64();
    let mut series_specs: Vec<SeriesSpec> = Vec::new();
    let mut timeout_events: Vec<f64> = Vec::new();
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut y_axis_unit = "";

    match target.view_mode {
        ProfileViewMode::Compare => {
            y_axis_unit = app.selected_metric.unit();
            for (idx, profile) in target.profiles.iter().enumerate() {
                let points = app.metrics.timeseries(
                    ProfileKey {
                        target_id: target.config.id,
                        profile_id: profile.config.id,
                    },
                    app.window,
                    app.selected_metric,
                    app.global.link_capacity_mbps,
                );
                update_bounds(&points, &mut min_y, &mut max_y);
                series_specs.push(SeriesSpec {
                    name: profile.config.name.clone(),
                    color: color_for_index(idx),
                    points,
                });
                timeout_events.extend(app.metrics.timeout_events(
                    ProfileKey {
                        target_id: target.config.id,
                        profile_id: profile.config.id,
                    },
                    app.window,
                ));
            }
        }
        ProfileViewMode::Single => {
            let profile = match target.profiles.get(target.selected_profile) {
                Some(profile) => profile,
                None => return,
            };
            let selected: Vec<MetricKind> = app.selected_metrics.iter().copied().collect();
            if let Some(metric) = selected.first() {
                y_axis_unit = metric.unit();
            }

            for (idx, metric) in selected.iter().enumerate() {
                let points = app.metrics.timeseries(
                    ProfileKey {
                        target_id: target.config.id,
                        profile_id: profile.config.id,
                    },
                    app.window,
                    *metric,
                    app.global.link_capacity_mbps,
                );
                update_bounds(&points, &mut min_y, &mut max_y);
                series_specs.push(SeriesSpec {
                    name: metric.label().to_string(),
                    color: color_for_index(idx),
                    points,
                });
            }
            timeout_events.extend(app.metrics.timeout_events(
                ProfileKey {
                    target_id: target.config.id,
                    profile_id: profile.config.id,
                },
                app.window,
            ));
        }
    }

    if min_y == f64::INFINITY || max_y == f64::NEG_INFINITY {
        min_y = 0.0;
        max_y = 1.0;
    }

    // Add 10% padding to y-axis
    let y_range = max_y - min_y;
    let y_padding = if y_range > 0.0 { y_range * 0.1 } else { 0.1 };
    min_y = (min_y - y_padding).max(0.0);
    max_y += y_padding;

    let timeout_y = if max_y > min_y {
        max_y - (max_y - min_y) * 0.05
    } else {
        max_y
    };
    let timeout_points: Vec<(f64, f64)> = timeout_events.iter().map(|x| (*x, timeout_y)).collect();

    let datasets: Vec<Dataset> = series_specs
        .iter()
        .map(|spec| {
            Dataset::default()
                .name(spec.name.clone())
                .graph_type(GraphType::Line)
                .style(Style::default().fg(spec.color))
                .data(&spec.points)
        })
        .collect();

    // Build color-coded legend
    let mut legend_spans: Vec<Span> = series_specs
        .iter()
        .enumerate()
        .flat_map(|(i, spec)| {
            let mut spans = vec![
                Span::styled("■ ", Style::default().fg(spec.color)),
                Span::styled(&spec.name, Style::default().fg(spec.color)),
            ];
            if i < series_specs.len() - 1 {
                spans.push(Span::styled("  ", Style::default()));
            }
            spans
        })
        .collect();

    let mut datasets = datasets;
    if !timeout_points.is_empty() {
        if !legend_spans.is_empty() {
            legend_spans.push(Span::styled("  ", Style::default()));
        }
        legend_spans.push(Span::styled("● ", Style::default().fg(Color::Red)));
        legend_spans.push(Span::styled("Timeout", Style::default().fg(Color::Red)));
        datasets.push(
            Dataset::default()
                .name("Timeout".to_string())
                .graph_type(GraphType::Scatter)
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Red))
                .data(&timeout_points),
        );
    }

    let chart_title = if target.view_mode == ProfileViewMode::Compare {
        format!(
            " Chart ({}) [{}] ",
            app.selected_metric.label(),
            app.window.label()
        )
    } else {
        format!(" Chart [{}] ", app.window.label())
    };

    let y_labels = format_y_axis_labels(min_y, max_y, y_axis_unit);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(chart_title)
                .title_bottom(Line::from(legend_spans).alignment(Alignment::Center))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().bg(Color::Black))
        .x_axis(
            ratatui::widgets::Axis::default()
                .title("Time (ago)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, window_seconds])
                .labels(vec![
                    Span::styled("now", Style::default().fg(Color::Green)),
                    Span::raw(format!("-{}s", window_seconds as u64 / 2)),
                    Span::raw(format!("-{}s", window_seconds as u64)),
                ]),
        )
        .y_axis(
            ratatui::widgets::Axis::default()
                .title(y_axis_unit)
                .style(Style::default().fg(Color::Gray))
                .bounds([min_y, max_y])
                .labels(y_labels),
        );
    frame.render_widget(chart, area);
}
