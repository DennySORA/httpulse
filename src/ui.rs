use crate::app::{AppState, ProfileViewMode, StatFocus, apply_edit_command, parse_profile_specs};
use crate::metrics::MetricKind;
use crate::metrics_aggregate::ProfileKey;
use crate::probe::ProbeSample;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{QueueableCommand, execute};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Chart, Dataset, GraphType, List, ListItem, Paragraph, Row, Table,
};
use std::io::{self, Stdout, Write};
use std::time::{Duration, Instant};
use url::Url;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputMode {
    Normal,
    AddTarget,
    EditTarget,
}

pub fn run_ui(
    mut app: AppState,
    sample_rx: crossbeam_channel::Receiver<ProbeSample>,
    sample_tx: crossbeam_channel::Sender<ProbeSample>,
) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input_mode = InputMode::Normal;
    let mut input_buffer = String::new();
    let mut should_quit = false;
    let mut last_tick = Instant::now();

    while !should_quit {
        while let Ok(sample) = sample_rx.try_recv() {
            app.apply_sample(sample);
        }

        terminal.draw(|frame| {
            let size = frame.area();
            let mut main_constraints = vec![Constraint::Min(3)];
            if input_mode != InputMode::Normal {
                main_constraints.push(Constraint::Length(3));
            }
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(main_constraints)
                .split(size);

            let main_area = chunks[0];
            draw_main(frame, main_area, &app);

            if input_mode != InputMode::Normal {
                let prompt = match input_mode {
                    InputMode::AddTarget => "add> url [profile1,profile2]",
                    InputMode::EditTarget => "edit> interval=5s timeout=10s dns=on/off",
                    InputMode::Normal => "",
                };
                let input = Paragraph::new(input_buffer.clone())
                    .block(Block::default().title(prompt).borders(Borders::ALL));
                frame.render_widget(input, chunks[1]);
            }
        })?;

        let tick_rate = Duration::from_secs_f64(1.0 / app.global.ui_refresh_hz as f64);
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match input_mode {
                    InputMode::Normal => {
                        if handle_normal_key(key, &mut app, &mut input_mode, &mut input_buffer) {
                            should_quit = true;
                        }
                    }
                    _ => {
                        handle_input_key(
                            key,
                            &mut app,
                            &mut input_mode,
                            &mut input_buffer,
                            &sample_tx,
                        );
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    cleanup_terminal(&mut terminal)?;
    Ok(())
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    terminal.backend_mut().queue(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    terminal.backend_mut().flush()?;
    Ok(())
}

fn handle_normal_key(
    key: KeyEvent,
    app: &mut AppState,
    input_mode: &mut InputMode,
    input_buffer: &mut String,
) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }
    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('a') => {
            *input_mode = InputMode::AddTarget;
            input_buffer.clear();
        }
        KeyCode::Char('e') => {
            *input_mode = InputMode::EditTarget;
            input_buffer.clear();
        }
        KeyCode::Char('d') => {
            if !app.targets.is_empty() {
                app.remove_target(app.selected_target);
            }
        }
        KeyCode::Char('p') => {
            app.toggle_pause(app.selected_target);
        }
        KeyCode::Char('c') => {
            if let Some(target) = app.selected_target_mut() {
                target.view_mode = match target.view_mode {
                    ProfileViewMode::Single => ProfileViewMode::Compare,
                    ProfileViewMode::Compare => ProfileViewMode::Single,
                };
            }
        }
        KeyCode::Char('s') => app.cycle_stat_focus(),
        KeyCode::Char('w') => app.cycle_window(),
        KeyCode::Down => {
            if app.selected_target + 1 < app.targets.len() {
                app.selected_target += 1;
            }
        }
        KeyCode::Up => {
            app.selected_target = app.selected_target.saturating_sub(1);
        }
        KeyCode::Tab => {
            if let Some(target) = app.selected_target_mut() {
                if !target.profiles.is_empty() {
                    target.selected_profile =
                        (target.selected_profile + 1) % target.profiles.len();
                }
            }
        }
        KeyCode::Char('1') => app.toggle_metric(MetricKind::Total),
        KeyCode::Char('2') => app.toggle_metric(MetricKind::Dns),
        KeyCode::Char('3') => app.toggle_metric(MetricKind::Connect),
        KeyCode::Char('4') => app.toggle_metric(MetricKind::Tls),
        KeyCode::Char('5') => app.toggle_metric(MetricKind::Ttfb),
        KeyCode::Char('6') => app.toggle_metric(MetricKind::Download),
        KeyCode::Char('7') => app.toggle_metric(MetricKind::Rtt),
        KeyCode::Char('8') => app.toggle_metric(MetricKind::Retrans),
        _ => {}
    }
    false
}

fn handle_input_key(
    key: KeyEvent,
    app: &mut AppState,
    input_mode: &mut InputMode,
    input_buffer: &mut String,
    sample_tx: &crossbeam_channel::Sender<ProbeSample>,
) {
    match key.code {
        KeyCode::Esc => {
            *input_mode = InputMode::Normal;
            input_buffer.clear();
        }
        KeyCode::Enter => {
            match *input_mode {
                InputMode::AddTarget => {
                    if let Some((url, profiles)) = parse_add_command(input_buffer) {
                        app.add_target(url, profiles, sample_tx.clone());
                    }
                }
                InputMode::EditTarget => {
                    if let Some(target) = app.selected_target() {
                        if let Some(updated) = apply_edit_command(target, input_buffer) {
                            app.update_target_config(app.selected_target, updated);
                        }
                    }
                }
                InputMode::Normal => {}
            }
            *input_mode = InputMode::Normal;
            input_buffer.clear();
        }
        KeyCode::Backspace => {
            input_buffer.pop();
        }
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return;
            }
            input_buffer.push(ch);
        }
        _ => {}
    }
}

fn parse_add_command(input: &str) -> Option<(Url, Option<Vec<crate::config::ProfileConfig>>)> {
    let mut parts = input.split_whitespace();
    let url_text = parts.next()?;
    let url = Url::parse(url_text).ok()?;
    let rest = parts.collect::<Vec<_>>().join(" ");
    if rest.is_empty() {
        Some((url, None))
    } else {
        Some((url, Some(parse_profile_specs(&rest))))
    }
}

fn draw_main(frame: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(10)])
        .split(area);

    draw_target_list(frame, chunks[0], app);
    draw_target_panes(frame, chunks[1], app);
}

fn draw_target_list(frame: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let items: Vec<ListItem> = app
        .targets
        .iter()
        .map(|target| {
            let status = if target.paused { "PAUSE" } else { "RUN" };
            let line = Line::from(format!("{status} {}", target.config.url));
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Targets").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    let mut state = list_state(app.selected_target);
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_target_panes(frame: &mut ratatui::Frame, area: Rect, app: &AppState) {
    if app.targets.is_empty() {
        let empty = Paragraph::new("No targets. Press 'a' to add.")
            .block(Block::default().title("Details").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    }

    let count = app.targets.len() as u32;
    let constraints = (0..count)
        .map(|_| Constraint::Ratio(1, count))
        .collect::<Vec<_>>();
    let panes = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (pane, target) in panes.iter().zip(app.targets.iter()) {
        draw_target_pane(frame, *pane, app, target);
    }
}

fn draw_target_pane(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    target: &crate::app::TargetRuntime,
) {
    let title = format!(
        "{} [{}] {}",
        target.config.url,
        if target.paused { "PAUSE" } else { "RUN" },
        match target.view_mode {
            ProfileViewMode::Single => "single",
            ProfileViewMode::Compare => "compare",
        }
    );
    let block = Block::default().title(title).borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(6)])
        .split(inner);

    draw_metrics_table(frame, sections[0], app, target);
    draw_chart(frame, sections[1], app, target);
}

fn draw_metrics_table(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    target: &crate::app::TargetRuntime,
) {
    let metrics = [
        MetricKind::Dns,
        MetricKind::Connect,
        MetricKind::Tls,
        MetricKind::Ttfb,
        MetricKind::Download,
        MetricKind::Total,
        MetricKind::Rtt,
        MetricKind::Retrans,
    ];

    let profiles: Vec<_> = match target.view_mode {
        ProfileViewMode::Single => target
            .profiles
            .get(target.selected_profile)
            .into_iter()
            .collect(),
        ProfileViewMode::Compare => target.profiles.iter().collect(),
    };

    let mut header_cells = vec![Span::styled(
        "metric",
        Style::default().add_modifier(Modifier::BOLD),
    )];
    for profile in &profiles {
        header_cells.push(Span::raw(profile.config.name.clone()));
    }
    let header = Row::new(header_cells);

    let rows = metrics.iter().map(|metric| {
        let mut cells = Vec::new();
        cells.push(Span::raw(metric.label()));
        for profile in &profiles {
            let aggregate = app.target_aggregate(target, profile);
            let stats = aggregate.by_metric.get(metric);
            let value = stats.and_then(|stats| stat_value(stats, app.stat_focus));
            cells.push(Span::raw(format_metric(*metric, value)));
        }
        Row::new(cells)
    });

    let table = Table::new(rows, vec![Constraint::Length(12); profiles.len() + 1])
        .header(header)
        .block(Block::default().title("metrics").borders(Borders::ALL));
    frame.render_widget(table, area);
}

fn draw_chart(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    target: &crate::app::TargetRuntime,
) {
    let window_seconds = app.window.duration().as_secs_f64();
    let mut series_specs: Vec<SeriesSpec> = Vec::new();
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    match target.view_mode {
        ProfileViewMode::Compare => {
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
            }
        }
        ProfileViewMode::Single => {
            let profile = match target.profiles.get(target.selected_profile) {
                Some(profile) => profile,
                None => return,
            };
            let selected: Vec<MetricKind> = app.selected_metrics.iter().copied().collect();
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
        }
    }

    if min_y == f64::INFINITY || max_y == f64::NEG_INFINITY {
        min_y = 0.0;
        max_y = 1.0;
    }

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

    let chart = Chart::new(datasets)
        .block(Block::default().title("chart").borders(Borders::ALL))
        .x_axis(ratatui::widgets::Axis::default().bounds([0.0, window_seconds]))
        .y_axis(ratatui::widgets::Axis::default().bounds([min_y, max_y]));
    frame.render_widget(chart, area);
}

fn stat_value(stats: &crate::metrics::MetricStats, focus: StatFocus) -> Option<f64> {
    match focus {
        StatFocus::P50 => stats.p50,
        StatFocus::P99 => stats.p99,
        StatFocus::Mean => stats.mean,
    }
}

fn format_metric(metric: MetricKind, value: Option<f64>) -> String {
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
        | MetricKind::Jitter => format!("{value:.1}ms"),
        MetricKind::GoodputBps => format!("{:.2}Mbps", value / 1_000_000.0),
        MetricKind::BandwidthUtilization => format!("{:.0}%", value * 100.0),
        MetricKind::ProbeLossRate => format!("{:.0}%", value * 100.0),
        _ => format!("{value:.0}"),
    }
}

fn list_state(selected: usize) -> ratatui::widgets::ListState {
    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(selected));
    state
}

fn color_for_index(idx: usize) -> Color {
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

struct SeriesSpec {
    name: String,
    color: Color,
    points: Vec<(f64, f64)>,
}

fn update_bounds(points: &[(f64, f64)], min_y: &mut f64, max_y: &mut f64) {
    if points.is_empty() {
        return;
    }
    let local_min = points.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let local_max = points
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);
    *min_y = min_y.min(local_min);
    *max_y = max_y.max(local_max);
}
