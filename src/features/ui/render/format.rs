use crate::app::MetricsCategory;
use crate::metrics::{MetricKind, MetricStats};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Span;

pub(super) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub(super) fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

pub(super) fn format_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1000 {
        format!("{:.1}K", count as f64 / 1000.0)
    } else {
        count.to_string()
    }
}

pub(super) fn format_latency(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.2}s", ms / 1000.0)
    } else if ms >= 100.0 {
        format!("{:.0}ms", ms)
    } else if ms >= 10.0 {
        format!("{:.1}ms", ms)
    } else {
        format!("{:.2}ms", ms)
    }
}

pub(super) fn format_goodput(bps: f64) -> String {
    if bps >= 1_000_000_000.0 {
        format!("{:.1} Gbps", bps / 1_000_000_000.0)
    } else if bps >= 1_000_000.0 {
        format!("{:.1} Mbps", bps / 1_000_000.0)
    } else if bps >= 1000.0 {
        format!("{:.1} Kbps", bps / 1000.0)
    } else {
        format!("{:.0} bps", bps)
    }
}

pub(super) fn style_for_success_rate(rate: f64) -> Style {
    if rate >= 99.0 {
        Style::default().fg(Color::Green)
    } else if rate >= 95.0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Red)
    }
}

pub(super) fn style_for_latency(ms: f64) -> Style {
    if ms <= 100.0 {
        Style::default().fg(Color::Green)
    } else if ms <= 500.0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Red)
    }
}

pub(super) fn style_for_timeout_count(count: u64) -> Style {
    if count == 0 {
        Style::default().fg(Color::Green)
    } else if count <= 3 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Red)
    }
}

/// Get metrics for a specific category
pub(super) fn metrics_for_category(category: MetricsCategory) -> &'static [MetricKind] {
    match category {
        MetricsCategory::Latency => &[
            MetricKind::Dns,
            MetricKind::Connect,
            MetricKind::Tls,
            MetricKind::Ttfb,
            MetricKind::Download,
            MetricKind::Total,
        ],
        MetricsCategory::Quality => &[MetricKind::Rtt, MetricKind::RttVar, MetricKind::Jitter],
        MetricsCategory::Reliability => &[
            MetricKind::Retrans,
            MetricKind::Reordering,
            MetricKind::TransportLoss,
            MetricKind::ProbeLossRate,
        ],
        MetricsCategory::Throughput => &[MetricKind::GoodputBps, MetricKind::BandwidthUtilization],
        MetricsCategory::Tcp => &[MetricKind::Cwnd, MetricKind::Ssthresh],
    }
}

pub(super) fn format_y_axis_labels(min_y: f64, max_y: f64, unit: &str) -> Vec<Span<'static>> {
    let mid_y = (min_y + max_y) / 2.0;

    let format_value = |v: f64| -> String {
        match unit {
            "ms" => {
                if v >= 1000.0 {
                    format!("{:.1}s", v / 1000.0)
                } else if v >= 10.0 {
                    format!("{:.0}ms", v)
                } else {
                    format!("{:.1}ms", v)
                }
            }
            "%" => format!("{:.0}%", v * 100.0),
            "Mbps" => {
                if v >= 1_000_000.0 {
                    format!("{:.1}Mb", v / 1_000_000.0)
                } else if v >= 1000.0 {
                    format!("{:.0}Kb", v / 1000.0)
                } else {
                    format!("{:.0}b", v)
                }
            }
            "" => {
                if v >= 1000.0 {
                    format!("{:.1}K", v / 1000.0)
                } else {
                    format!("{:.0}", v)
                }
            }
            _ => format!("{:.0}{}", v, unit),
        }
    };

    vec![
        Span::raw(format_value(min_y)),
        Span::raw(format_value(mid_y)),
        Span::raw(format_value(max_y)),
    ]
}

pub(super) fn format_stat_triplet(metric: MetricKind, stats: Option<&MetricStats>) -> String {
    let p50 = format_metric_value(metric, stats.and_then(|stats| stats.p50));
    let p99 = format_metric_value(metric, stats.and_then(|stats| stats.p99));
    let mean = format_metric_value(metric, stats.and_then(|stats| stats.mean));
    format!("{p50}/{p99}/{mean}")
}

pub(super) fn format_metric_value(metric: MetricKind, value: Option<f64>) -> String {
    let value = match value {
        Some(value) => value,
        None => return "—".to_string(),
    };

    match metric {
        MetricKind::Dns
        | MetricKind::Connect
        | MetricKind::Tls
        | MetricKind::Ttfb
        | MetricKind::Download
        | MetricKind::Total
        | MetricKind::Rtt
        | MetricKind::RttVar
        | MetricKind::Jitter => {
            if value < 1.0 {
                format!("{:.1}", value)
            } else if value < 1000.0 {
                format!("{:.0}", value)
            } else {
                format!("{:.1}s", value / 1000.0)
            }
        }
        MetricKind::GoodputBps => {
            let mbps = value / 1_000_000.0;
            if mbps < 1.0 {
                format!("{:.0}K", value / 1000.0)
            } else {
                format!("{:.1}M", mbps)
            }
        }
        MetricKind::BandwidthUtilization | MetricKind::ProbeLossRate => {
            format!("{:.1}%", value * 100.0)
        }
        _ => {
            if value < 1000.0 {
                format!("{:.0}", value)
            } else {
                format!("{:.1}K", value / 1000.0)
            }
        }
    }
}

pub(super) fn list_state(selected: usize) -> ratatui::widgets::ListState {
    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(selected));
    state
}

pub(super) fn color_for_index(idx: usize) -> Color {
    const COLORS: [Color; 6] = [
        Color::Cyan,
        Color::Yellow,
        Color::Green,
        Color::Magenta,
        Color::Blue,
        Color::Red,
    ];
    COLORS[idx % COLORS.len()]
}

pub(super) fn update_bounds(points: &[(f64, f64)], min_y: &mut f64, max_y: &mut f64) {
    if points.is_empty() {
        return;
    }

    for (_, y) in points {
        *min_y = min_y.min(*y);
        *max_y = max_y.max(*y);
    }
}
