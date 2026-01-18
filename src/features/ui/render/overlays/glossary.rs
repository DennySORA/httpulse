use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap};

use super::super::super::state::GLOSSARY_PAGE_COUNT;
use super::super::format::centered_rect;

pub(in crate::features::ui) fn draw_glossary_popup(
    frame: &mut ratatui::Frame,
    area: Rect,
    page: usize,
) {
    let popup_area = centered_rect(75, 85, area);
    frame.render_widget(Clear, popup_area);

    let page_titles = [
        "Latency Metrics",
        "Quality & Reliability",
        "Throughput & TCP",
    ];
    let page_title = page_titles.get(page).unwrap_or(&"Glossary");

    let glossary_text = match page {
        0 => vec![
            Line::styled(
                "─── Latency Metrics ───",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  DNS        ", Style::default().fg(Color::Cyan)),
                Span::raw("Time to resolve domain name to IP address."),
            ]),
            Line::styled(
                "               Includes recursive resolver lookup time.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Connect    ", Style::default().fg(Color::Cyan)),
                Span::raw("TCP three-way handshake duration (SYN→SYN-ACK→ACK)."),
            ]),
            Line::styled(
                "               Reflects network latency to server.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  TLS        ", Style::default().fg(Color::Cyan)),
                Span::raw("TLS/SSL handshake time after TCP connection."),
            ]),
            Line::styled(
                "               Includes certificate verification and key exchange.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  TTFB       ", Style::default().fg(Color::Cyan)),
                Span::raw("Time To First Byte - server processing time."),
            ]),
            Line::styled(
                "               From request sent to first response byte received.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Download   ", Style::default().fg(Color::Cyan)),
                Span::raw("Time to download response body."),
            ]),
            Line::styled(
                "               Affected by bandwidth, content size, and server speed.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Total      ", Style::default().fg(Color::Cyan)),
                Span::raw("Complete request lifecycle time."),
            ]),
            Line::styled(
                "               DNS + Connect + TLS + TTFB + Download.",
                Style::default().fg(Color::DarkGray),
            ),
        ],
        1 => vec![
            Line::styled(
                "─── Quality & Reliability ───",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  RTT        ", Style::default().fg(Color::Cyan)),
                Span::raw("Round-trip time from TCP_INFO (kernel-level)."),
            ]),
            Line::styled(
                "               Measures network latency without application overhead.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  RTTVar     ", Style::default().fg(Color::Cyan)),
                Span::raw("RTT variance - network stability indicator."),
            ]),
            Line::styled(
                "               Lower values indicate more stable network.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Jitter     ", Style::default().fg(Color::Cyan)),
                Span::raw("Variation in total latency between probes."),
            ]),
            Line::styled(
                "               Computed as abs(diff) between consecutive samples.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Retrans    ", Style::default().fg(Color::Cyan)),
                Span::raw("TCP packet retransmissions count."),
            ]),
            Line::styled(
                "               Indicates packet loss or network congestion.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Reordering ", Style::default().fg(Color::Cyan)),
                Span::raw("Out-of-order packet delivery events."),
            ]),
            Line::styled(
                "               Higher values suggest network path issues.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Loss Rate  ", Style::default().fg(Color::Cyan)),
                Span::raw("% of probes that failed (timeouts/errors)."),
            ]),
            Line::styled(
                "               Application-level reliability metric.",
                Style::default().fg(Color::DarkGray),
            ),
        ],
        _ => vec![
            Line::styled(
                "─── Throughput & TCP ───",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Goodput    ", Style::default().fg(Color::Cyan)),
                Span::raw("Application-layer throughput (successful bytes)."),
            ]),
            Line::styled(
                "               Excludes protocol overhead and retransmissions.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Utilization", Style::default().fg(Color::Cyan)),
                Span::raw("Goodput / configured link capacity."),
            ]),
            Line::styled(
                "               Requires setting Link Capacity in Settings.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  cwnd       ", Style::default().fg(Color::Cyan)),
                Span::raw("TCP congestion window size (packets)."),
            ]),
            Line::styled(
                "               Controls how much data can be in flight.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ssthresh   ", Style::default().fg(Color::Cyan)),
                Span::raw("Slow-start threshold value."),
            ]),
            Line::styled(
                "               Boundary between slow start and congestion avoidance.",
                Style::default().fg(Color::DarkGray),
            ),
        ],
    };

    let page_indicator: Vec<Span> = (0..GLOSSARY_PAGE_COUNT)
        .flat_map(|i| {
            let is_active = i == page;
            let style = if is_active {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let mut spans = vec![Span::styled(format!(" {} ", i + 1), style)];
            if i < GLOSSARY_PAGE_COUNT - 1 {
                spans.push(Span::styled("·", Style::default().fg(Color::DarkGray)));
            }
            spans
        })
        .collect();

    let glossary = Paragraph::new(glossary_text)
        .block(
            Block::default()
                .title(format!(" Glossary - {} ", page_title))
                .title_alignment(Alignment::Center)
                .title_bottom(Line::from(page_indicator).alignment(Alignment::Center))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .padding(Padding::horizontal(1)),
        )
        .style(Style::default().bg(Color::Black))
        .wrap(Wrap { trim: false });

    frame.render_widget(glossary, popup_area);
}
