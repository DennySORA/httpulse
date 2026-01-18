use crate::app::{AppState, TargetRuntime};
use crate::metrics::MetricKind;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::super::super::format::format_latency;

/// Combined network info pane showing Profile, Connection, and TCP stats
pub(in crate::features::ui) fn draw_network_info_pane(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    target: &TargetRuntime,
) {
    let profile = match target.profiles.get(target.selected_profile) {
        Some(p) => p,
        None => return,
    };

    let aggregate = app.target_aggregate(target, profile);
    let mut lines: Vec<Line> = Vec::new();

    // Section: Profile
    lines.push(Line::styled(
        "─ Profile ─",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));

    let http_tls = format!("{:?}/{:?}", profile.config.http, profile.config.tls);
    lines.push(Line::from(vec![
        Span::styled(" Proto ", Style::default().fg(Color::DarkGray)),
        Span::styled(http_tls, Style::default().fg(Color::Yellow)),
    ]));

    let reuse = format!("{:?}", profile.config.conn_reuse);
    lines.push(Line::from(vec![
        Span::styled(" Reuse ", Style::default().fg(Color::DarkGray)),
        Span::styled(reuse, Style::default().fg(Color::Cyan)),
    ]));

    // Section: Connection
    lines.push(Line::styled(
        "─ Connection ─",
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    ));

    if let Some(last_sample) = &profile.last_sample {
        if let Some(remote) = &last_sample.remote {
            lines.push(Line::from(vec![
                Span::styled(" Addr  ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{}:{}", remote.ip(), remote.port())),
            ]));
        }
        let alpn = last_sample.negotiated.alpn.as_deref().unwrap_or("—");
        let tls_ver = last_sample.negotiated.tls_version.as_deref().unwrap_or("—");
        lines.push(Line::from(vec![
            Span::styled(" ALPN  ", Style::default().fg(Color::DarkGray)),
            Span::styled(alpn, Style::default().fg(Color::Green)),
            Span::styled(" TLS ", Style::default().fg(Color::DarkGray)),
            Span::raw(tls_ver),
        ]));
    } else {
        lines.push(Line::styled(
            " (no data)",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Section: TCP State (from TCP_INFO)
    lines.push(Line::styled(
        "─ TCP State ─",
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    ));

    // Get TCP stats
    let cwnd_stats = aggregate.by_metric.get(&MetricKind::Cwnd);
    let ssthresh_stats = aggregate.by_metric.get(&MetricKind::Ssthresh);
    let rtt_stats = aggregate.by_metric.get(&MetricKind::Rtt);
    let rttvar_stats = aggregate.by_metric.get(&MetricKind::RttVar);

    // Display TCP metrics
    if let Some(stats) = &rtt_stats
        && let Some(mean) = stats.mean
    {
        lines.push(Line::from(vec![
            Span::styled(" RTT   ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_latency(mean), Style::default().fg(Color::Green)),
        ]));
    }
    if let Some(stats) = &rttvar_stats
        && let Some(mean) = stats.mean
    {
        lines.push(Line::from(vec![
            Span::styled(" RTTV  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_latency(mean), Style::default().fg(Color::Yellow)),
        ]));
    }
    if let Some(stats) = &cwnd_stats
        && let Some(mean) = stats.mean
    {
        lines.push(Line::from(vec![
            Span::styled(" cwnd  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{mean:.0}"), Style::default().fg(Color::Cyan)),
        ]));
    }
    if let Some(stats) = &ssthresh_stats
        && let Some(mean) = stats.mean
    {
        lines.push(Line::from(vec![
            Span::styled(" ssth  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{mean:.0}"), Style::default().fg(Color::Magenta)),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(" Network Info ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(paragraph, area);
}
