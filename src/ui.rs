use crate::app::{
    apply_edit_command, parse_profile_specs, parse_target_url, AppState, ProfileViewMode,
    TargetPaneMode,
};
use crate::metrics::MetricKind;
use crate::metrics_aggregate::ProfileKey;
use crate::probe::ProbeSample;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, QueueableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Chart, Clear, Dataset, GraphType, List, ListItem, Padding, Paragraph,
    Row, Table, TableState, Wrap,
};
use ratatui::Terminal;
use std::io::{self, Stdout, Write};
use std::time::{Duration, Instant};
use url::Url;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsField {
    UiRefreshHz,
    LinkCapacityMbps,
    TargetInterval,
    TargetTimeout,
    TargetDnsEnabled,
    TargetPane,
    TargetPaused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputMode {
    Normal,
    AddTarget,
    Help,
    Settings,
    SettingsEdit(SettingsField),
    ConfirmDelete,
}

struct SettingsRow {
    field: SettingsField,
    scope: &'static str,
    label: &'static str,
    value: String,
    action: &'static str,
}

struct SettingsState {
    selected: usize,
    notice: Option<String>,
}

impl SettingsState {
    fn new() -> Self {
        Self {
            selected: 0,
            notice: None,
        }
    }

    fn select_next(&mut self, total: usize) {
        if total == 0 {
            self.selected = 0;
            return;
        }
        self.selected = (self.selected + 1) % total;
    }

    fn select_prev(&mut self, total: usize) {
        if total == 0 {
            self.selected = 0;
            return;
        }
        if self.selected == 0 {
            self.selected = total - 1;
        } else {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    fn clamp(&mut self, total: usize) {
        if total == 0 {
            self.selected = 0;
        } else if self.selected >= total {
            self.selected = total - 1;
        }
    }

    fn clear_notice(&mut self) {
        self.notice = None;
    }
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
    let mut settings_state = SettingsState::new();
    let mut should_quit = false;
    let mut last_tick = Instant::now();

    while !should_quit {
        while let Ok(sample) = sample_rx.try_recv() {
            app.apply_sample(sample);
        }

        terminal.draw(|frame| {
            let size = frame.area();

            // Main layout: Header, Content, Input (optional), Footer
            let mut constraints = vec![
                Constraint::Length(1), // Header
                Constraint::Min(10),   // Content
            ];
            if matches!(input_mode, InputMode::AddTarget) {
                constraints.push(Constraint::Length(3)); // Input bar
            }
            constraints.push(Constraint::Length(1)); // Footer

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(size);

            // Header bar
            draw_header(frame, chunks[0], &app);

            // Main content area
            draw_main(frame, chunks[1], &app);

            // Input bar (if in input mode)
            let footer_idx = if matches!(input_mode, InputMode::AddTarget) {
                let prompt = match input_mode {
                    InputMode::AddTarget => " Add Target: <url> [profile1,profile2,...] ",
                    _ => "",
                };
                let input = Paragraph::new(Line::from(vec![
                    Span::styled(prompt, Style::default().fg(Color::Yellow)),
                    Span::raw(&input_buffer),
                    Span::styled("█", Style::default().fg(Color::Gray)),
                ]))
                .style(Style::default().bg(Color::DarkGray));
                frame.render_widget(input, chunks[2]);
                3
            } else {
                2
            };

            // Footer with keybindings
            draw_footer(frame, chunks[footer_idx], input_mode);

            // Overlay popups
            match input_mode {
                InputMode::Help => draw_help_popup(frame, size),
                InputMode::Settings | InputMode::SettingsEdit(_) => {
                    draw_settings_popup(
                        frame,
                        size,
                        &app,
                        &settings_state,
                        input_mode,
                        &input_buffer,
                    );
                }
                InputMode::ConfirmDelete => draw_confirm_delete_popup(frame, size, &app),
                _ => {}
            }
        })?;

        let tick_rate = Duration::from_secs_f64(1.0 / app.global.ui_refresh_hz as f64);
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match input_mode {
                    InputMode::Normal => {
                        if handle_normal_key(
                            key,
                            &mut app,
                            &mut input_mode,
                            &mut input_buffer,
                            &mut settings_state,
                        ) {
                            should_quit = true;
                        }
                    }
                    InputMode::Help => {
                        handle_help_key(key, &mut input_mode);
                    }
                    InputMode::Settings => {
                        handle_settings_key(
                            key,
                            &mut app,
                            &mut input_mode,
                            &mut input_buffer,
                            &mut settings_state,
                        );
                    }
                    InputMode::SettingsEdit(field) => {
                        handle_settings_edit_key(
                            key,
                            &mut app,
                            &mut input_mode,
                            &mut input_buffer,
                            field,
                            &mut settings_state,
                        );
                    }
                    InputMode::ConfirmDelete => {
                        handle_confirm_delete_key(key, &mut app, &mut input_mode);
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

fn draw_header(frame: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let selected_metrics: Vec<_> = app.selected_metrics.iter().map(|m| m.label()).collect();
    let metrics_str = if selected_metrics.is_empty() {
        "none".to_string()
    } else {
        selected_metrics.join(",")
    };

    let header = Line::from(vec![
        Span::styled(
            " Monitor Network ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("│ "),
        Span::styled("Window:", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {} ", app.window.label()),
            Style::default().fg(Color::Green),
        ),
        Span::raw("│ "),
        Span::styled("Stats:", Style::default().fg(Color::DarkGray)),
        Span::styled(" P50/P99/Mean ", Style::default().fg(Color::Yellow)),
        Span::raw("│ "),
        Span::styled("Metrics:", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {} ", metrics_str),
            Style::default().fg(Color::Magenta),
        ),
        Span::raw("│ "),
        Span::styled("Targets:", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {} ", app.targets.len()),
            Style::default().fg(Color::White),
        ),
    ]);

    let paragraph = Paragraph::new(header).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut ratatui::Frame, area: Rect, mode: InputMode) {
    let hints = match mode {
        InputMode::Normal => vec![
            ("q", "Quit"),
            ("?", "Help"),
            ("S", "Settings"),
            ("a", "Add"),
            ("d", "Del"),
            ("e", "Edit"),
            ("p", "Pause"),
            ("c", "Compare"),
            ("g", "Pane"),
            ("w", "Window"),
            ("Up/Down", "Select"),
            ("Tab", "Profile"),
            ("1-8", "Metrics"),
        ],
        InputMode::Help => vec![("Esc", "Close"), ("q", "Close")],
        InputMode::Settings => vec![
            ("Esc", "Close"),
            ("Up/Down", "Select"),
            ("Enter", "Edit"),
            ("Space", "Toggle"),
        ],
        InputMode::SettingsEdit(_) => vec![("Enter", "Apply"), ("Esc", "Cancel")],
        InputMode::AddTarget => vec![("Enter", "Confirm"), ("Esc", "Cancel")],
        InputMode::ConfirmDelete => vec![("y", "Yes, Delete"), ("n/Esc", "Cancel")],
    };

    let spans: Vec<Span> = hints
        .iter()
        .enumerate()
        .flat_map(|(i, (key, desc))| {
            let mut result = vec![
                Span::styled(
                    format!(" {} ", key),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{} ", desc), Style::default().fg(Color::White)),
            ];
            if i < hints.len() - 1 {
                result.push(Span::raw(" "));
            }
            result
        })
        .collect();

    let footer = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(footer, area);
}

fn draw_help_popup(frame: &mut ratatui::Frame, area: Rect) {
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

fn draw_settings_popup(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    settings_state: &SettingsState,
    input_mode: InputMode,
    input_buffer: &str,
) {
    let popup_area = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup_area);

    let rows = settings_rows(app);
    let mut table_state = TableState::default();
    let mut selected = settings_state.selected;
    if rows.is_empty() {
        table_state.select(None);
    } else {
        if selected >= rows.len() {
            selected = rows.len().saturating_sub(1);
        }
        table_state.select(Some(selected));
    }

    let title = app
        .selected_target()
        .map(|target| {
            format!(
                " Settings - {} ",
                truncate_string(target.config.url.as_str(), 30)
            )
        })
        .unwrap_or_else(|| " Settings ".to_string());

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut constraints = vec![Constraint::Min(6), Constraint::Length(2)];
    if matches!(input_mode, InputMode::SettingsEdit(_)) {
        constraints.push(Constraint::Length(3));
    }
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let header = Row::new(vec![
        Cell::from("Scope"),
        Cell::from("Setting"),
        Cell::from("Value"),
        Cell::from("Action"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let table_rows = rows.iter().map(|row| {
        Row::new(vec![
            Cell::from(row.scope),
            Cell::from(row.label),
            Cell::from(row.value.clone()),
            Cell::from(row.action),
        ])
    });

    let widths = [
        Constraint::Length(8),
        Constraint::Length(18),
        Constraint::Length(18),
        Constraint::Min(12),
    ];

    let table = Table::new(table_rows, widths)
        .header(header)
        .column_spacing(1)
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_widget(Clear, sections[0]);
    frame.render_stateful_widget(table, sections[0], &mut table_state);

    let mut help_lines = vec![Line::from(vec![
        Span::styled("  ↑↓ ", Style::default().fg(Color::Green)),
        Span::raw("Select  "),
        Span::styled("Enter ", Style::default().fg(Color::Green)),
        Span::raw("Edit/Toggle  "),
        Span::styled("Esc ", Style::default().fg(Color::Green)),
        Span::raw("Close"),
    ])];
    if let Some(notice) = &settings_state.notice {
        help_lines.push(Line::styled(
            format!("  {notice}"),
            Style::default().fg(Color::Red),
        ));
    }

    let help = Paragraph::new(help_lines).style(Style::default().bg(Color::Black));
    frame.render_widget(help, sections[1]);

    if let InputMode::SettingsEdit(field) = input_mode {
        let prompt = settings_edit_prompt(field);
        let input_line = Line::from(vec![
            Span::styled(format!("  {prompt}"), Style::default().fg(Color::Yellow)),
            Span::raw(input_buffer),
            Span::styled("█", Style::default().fg(Color::Gray)),
        ]);
        let input = Paragraph::new(input_line).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(input, sections[2]);
    }
}

fn settings_rows(app: &AppState) -> Vec<SettingsRow> {
    let mut rows = Vec::new();
    rows.push(SettingsRow {
        field: SettingsField::UiRefreshHz,
        scope: "Global",
        label: "UI refresh",
        value: format!("{} Hz", app.global.ui_refresh_hz),
        action: "Enter to edit",
    });
    rows.push(SettingsRow {
        field: SettingsField::LinkCapacityMbps,
        scope: "Global",
        label: "Link capacity",
        value: app
            .global
            .link_capacity_mbps
            .map(|value| format!("{value:.1} Mbps"))
            .unwrap_or_else(|| "Off".to_string()),
        action: "Enter to edit",
    });

    if let Some(target) = app.selected_target() {
        rows.push(SettingsRow {
            field: SettingsField::TargetInterval,
            scope: "Target",
            label: "Interval",
            value: format!("{}s", target.config.interval.as_secs()),
            action: "Enter to edit",
        });
        rows.push(SettingsRow {
            field: SettingsField::TargetTimeout,
            scope: "Target",
            label: "Timeout",
            value: format!("{}s", target.config.timeout_total.as_secs()),
            action: "Enter to edit",
        });
        rows.push(SettingsRow {
            field: SettingsField::TargetDnsEnabled,
            scope: "Target",
            label: "DNS",
            value: if target.config.dns_enabled {
                "On".to_string()
            } else {
                "Off".to_string()
            },
            action: "Enter to toggle",
        });
        rows.push(SettingsRow {
            field: SettingsField::TargetPane,
            scope: "Target",
            label: "Pane",
            value: target.pane_mode.label().to_string(),
            action: "Enter to cycle",
        });
        rows.push(SettingsRow {
            field: SettingsField::TargetPaused,
            scope: "Target",
            label: "Status",
            value: if target.paused {
                "Paused".to_string()
            } else {
                "Running".to_string()
            },
            action: "Enter to toggle",
        });
    }

    rows
}

fn settings_edit_prompt(field: SettingsField) -> &'static str {
    match field {
        SettingsField::UiRefreshHz => "Set UI refresh (Hz): ",
        SettingsField::LinkCapacityMbps => "Set link capacity Mbps (blank=off): ",
        SettingsField::TargetInterval => "Set probe interval (e.g. 5s): ",
        SettingsField::TargetTimeout => "Set timeout (e.g. 10s): ",
        SettingsField::TargetDnsEnabled
        | SettingsField::TargetPane
        | SettingsField::TargetPaused => "Press Enter to toggle: ",
    }
}

fn seed_settings_input(app: &AppState, field: SettingsField) -> String {
    match field {
        SettingsField::UiRefreshHz => app.global.ui_refresh_hz.to_string(),
        SettingsField::LinkCapacityMbps => app
            .global
            .link_capacity_mbps
            .map(|value| format!("{value:.1}"))
            .unwrap_or_default(),
        SettingsField::TargetInterval => app
            .selected_target()
            .map(|target| format!("{}s", target.config.interval.as_secs()))
            .unwrap_or_default(),
        SettingsField::TargetTimeout => app
            .selected_target()
            .map(|target| format!("{}s", target.config.timeout_total.as_secs()))
            .unwrap_or_default(),
        SettingsField::TargetDnsEnabled
        | SettingsField::TargetPane
        | SettingsField::TargetPaused => String::new(),
    }
}

fn parse_link_capacity_mbps(input: &str) -> Result<Option<f64>, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("off")
        || trimmed.eq_ignore_ascii_case("none")
    {
        return Ok(None);
    }

    let normalized = trimmed
        .strip_suffix("mbps")
        .or_else(|| trimmed.strip_suffix("Mbps"))
        .unwrap_or(trimmed);
    let value: f64 = normalized.trim().parse().map_err(|_| "Invalid number")?;
    if value <= 0.0 {
        return Err("Value must be > 0");
    }
    Ok(Some(value))
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn draw_confirm_delete_popup(frame: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let popup_area = centered_rect(40, 25, area);
    frame.render_widget(Clear, popup_area);

    let target_name = app
        .selected_target()
        .map(|t| t.config.url.as_str())
        .unwrap_or("Unknown");

    let lines = vec![
        Line::from(""),
        Line::styled(
            "  Delete this target?  ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                truncate_string(target_name, 30),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Press "),
            Span::styled(" y ", Style::default().fg(Color::Black).bg(Color::Red)),
            Span::raw(" to delete, "),
            Span::styled(" n ", Style::default().fg(Color::Black).bg(Color::Green)),
            Span::raw(" to cancel"),
        ]),
    ];

    let popup = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Confirm Delete ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .style(Style::default().bg(Color::Black));

    frame.render_widget(popup, popup_area);
}

fn handle_normal_key(
    key: KeyEvent,
    app: &mut AppState,
    input_mode: &mut InputMode,
    input_buffer: &mut String,
    settings_state: &mut SettingsState,
) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }
    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('?') => {
            *input_mode = InputMode::Help;
        }
        KeyCode::Char('S') => {
            *input_mode = InputMode::Settings;
            settings_state.selected = 0;
            settings_state.clear_notice();
        }
        KeyCode::Char('a') => {
            *input_mode = InputMode::AddTarget;
            input_buffer.clear();
        }
        KeyCode::Char('e') => {
            *input_mode = InputMode::Settings;
            settings_state.selected = 0;
            settings_state.clear_notice();
        }
        KeyCode::Char('d') => {
            if !app.targets.is_empty() {
                *input_mode = InputMode::ConfirmDelete;
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
        KeyCode::Char('g') => app.cycle_pane_mode(app.selected_target),
        KeyCode::Char('w') => app.cycle_window(),
        KeyCode::Down | KeyCode::Char('j') => {
            if app.selected_target + 1 < app.targets.len() {
                app.selected_target += 1;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected_target = app.selected_target.saturating_sub(1);
        }
        KeyCode::Tab => {
            if let Some(target) = app.selected_target_mut() {
                if !target.profiles.is_empty() {
                    target.selected_profile = (target.selected_profile + 1) % target.profiles.len();
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

fn handle_help_key(key: KeyEvent, input_mode: &mut InputMode) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            *input_mode = InputMode::Normal;
        }
        _ => {}
    }
}

fn handle_settings_key(
    key: KeyEvent,
    app: &mut AppState,
    input_mode: &mut InputMode,
    input_buffer: &mut String,
    settings_state: &mut SettingsState,
) {
    let rows = settings_rows(app);
    settings_state.clamp(rows.len());

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('S') => {
            *input_mode = InputMode::Normal;
            settings_state.clear_notice();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            settings_state.select_next(rows.len());
            settings_state.clear_notice();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            settings_state.select_prev(rows.len());
            settings_state.clear_notice();
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            settings_state.clear_notice();
            if let Some(row) = rows.get(settings_state.selected) {
                match row.field {
                    SettingsField::TargetDnsEnabled => {
                        if let Some(target) = app.selected_target() {
                            let mut updated = target.config.clone();
                            updated.dns_enabled = !updated.dns_enabled;
                            app.update_target_config(app.selected_target, updated);
                        }
                    }
                    SettingsField::TargetPane => {
                        app.cycle_pane_mode(app.selected_target);
                    }
                    SettingsField::TargetPaused => {
                        app.toggle_pause(app.selected_target);
                    }
                    SettingsField::UiRefreshHz
                    | SettingsField::LinkCapacityMbps
                    | SettingsField::TargetInterval
                    | SettingsField::TargetTimeout => {
                        *input_mode = InputMode::SettingsEdit(row.field);
                        input_buffer.clear();
                        input_buffer.push_str(&seed_settings_input(app, row.field));
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_settings_edit_key(
    key: KeyEvent,
    app: &mut AppState,
    input_mode: &mut InputMode,
    input_buffer: &mut String,
    field: SettingsField,
    settings_state: &mut SettingsState,
) {
    match key.code {
        KeyCode::Esc => {
            *input_mode = InputMode::Settings;
            input_buffer.clear();
            settings_state.clear_notice();
        }
        KeyCode::Enter => {
            let trimmed = input_buffer.trim();
            let mut applied = false;
            settings_state.clear_notice();
            match field {
                SettingsField::UiRefreshHz => {
                    if let Ok(value) = trimmed.parse::<u16>() {
                        if value > 0 {
                            app.global.ui_refresh_hz = value;
                            applied = true;
                        } else {
                            settings_state.notice = Some("Refresh must be > 0".to_string());
                        }
                    } else {
                        settings_state.notice = Some("Invalid refresh value".to_string());
                    }
                }
                SettingsField::LinkCapacityMbps => match parse_link_capacity_mbps(trimmed) {
                    Ok(value) => {
                        app.global.link_capacity_mbps = value;
                        applied = true;
                    }
                    Err(message) => {
                        settings_state.notice = Some(message.to_string());
                    }
                },
                SettingsField::TargetInterval => {
                    if let Some(target) = app.selected_target() {
                        let command = format!("interval={trimmed}");
                        if let Some(updated) = apply_edit_command(target, &command) {
                            app.update_target_config(app.selected_target, updated);
                            applied = true;
                        } else {
                            settings_state.notice = Some("Invalid interval value".to_string());
                        }
                    }
                }
                SettingsField::TargetTimeout => {
                    if let Some(target) = app.selected_target() {
                        let command = format!("timeout={trimmed}");
                        if let Some(updated) = apply_edit_command(target, &command) {
                            app.update_target_config(app.selected_target, updated);
                            applied = true;
                        } else {
                            settings_state.notice = Some("Invalid timeout value".to_string());
                        }
                    }
                }
                SettingsField::TargetDnsEnabled
                | SettingsField::TargetPane
                | SettingsField::TargetPaused => {}
            }

            if applied {
                *input_mode = InputMode::Settings;
                input_buffer.clear();
            }
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

fn handle_confirm_delete_key(key: KeyEvent, app: &mut AppState, input_mode: &mut InputMode) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.remove_target(app.selected_target);
            *input_mode = InputMode::Normal;
        }
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            *input_mode = InputMode::Normal;
        }
        _ => {}
    }
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
                InputMode::Normal
                | InputMode::Help
                | InputMode::Settings
                | InputMode::SettingsEdit(_)
                | InputMode::ConfirmDelete => {}
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
    let url = parse_target_url(url_text)?;
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
    target: &crate::app::TargetRuntime,
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

    // Layout: metrics table and/or chart, plus optional error bar
    let mut constraints = Vec::new();
    match pane_mode {
        TargetPaneMode::Split => {
            constraints.push(Constraint::Length(7));
            constraints.push(Constraint::Min(6));
        }
        TargetPaneMode::Chart => {
            constraints.push(Constraint::Min(10));
        }
        TargetPaneMode::Metrics => {
            constraints.push(Constraint::Min(10));
        }
    }
    if has_error {
        constraints.push(Constraint::Length(2));
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut idx = 0usize;
    if pane_mode != TargetPaneMode::Chart {
        draw_metrics_table(frame, sections[idx], app, target);
        idx += 1;
    }
    if pane_mode != TargetPaneMode::Metrics {
        draw_chart(frame, sections[idx], app, target);
        idx += 1;
    }

    // Draw error bar if there are errors
    if has_error {
        let error_msg: String = errors
            .iter()
            .map(|(name, err)| format!("{}: {:?}", name, err))
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
        frame.render_widget(error_para, sections[idx]);
    }
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

    let rows = metrics.iter().map(|metric| {
        let is_selected = app.selected_metrics.contains(metric);
        let metric_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let mut cells = Vec::new();
        cells.push(Span::styled(metric.label(), metric_style));
        for profile in &profiles {
            let aggregate = app.target_aggregate(target, profile);
            let stats = aggregate.by_metric.get(metric);
            cells.push(Span::raw(format_stat_triplet(*metric, stats)));
        }
        Row::new(cells)
    });

    let widths: Vec<Constraint> = std::iter::once(Constraint::Length(10))
        .chain(profiles.iter().map(|_| Constraint::Length(18)))
        .collect();

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .title(" Metrics (P50/P99/Mean) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(table, area);
}

fn draw_chart(
    frame: &mut ratatui::Frame,
    area: Rect,
    app: &AppState,
    target: &crate::app::TargetRuntime,
) {
    frame.render_widget(Clear, area);

    let window_seconds = app.window.duration().as_secs_f64();
    let mut series_specs: Vec<SeriesSpec> = Vec::new();
    let mut timeout_events: Vec<f64> = Vec::new();
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

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(format!(" Chart [{}] ", app.window.label()))
                .title_bottom(Line::from(legend_spans))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().bg(Color::Black))
        .x_axis(
            ratatui::widgets::Axis::default()
                .bounds([0.0, window_seconds])
                .labels(vec![
                    Span::raw("0"),
                    Span::raw(format!("{}s", window_seconds as u64 / 2)),
                    Span::raw(format!("{}s", window_seconds as u64)),
                ]),
        )
        .y_axis(
            ratatui::widgets::Axis::default()
                .bounds([min_y, max_y])
                .labels(vec![
                    Span::raw(format!("{:.0}", min_y)),
                    Span::raw(format!("{:.0}", (min_y + max_y) / 2.0)),
                    Span::raw(format!("{:.0}", max_y)),
                ]),
        );
    frame.render_widget(chart, area);
}

fn format_stat_triplet(metric: MetricKind, stats: Option<&crate::metrics::MetricStats>) -> String {
    let p50 = format_metric_compact(metric, stats.and_then(|stats| stats.p50));
    let p99 = format_metric_compact(metric, stats.and_then(|stats| stats.p99));
    let mean = format_metric_compact(metric, stats.and_then(|stats| stats.mean));
    format!("{p50}/{p99}/{mean}")
}

fn format_metric_compact(metric: MetricKind, value: Option<f64>) -> String {
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
        | MetricKind::Jitter => format!("{value:.1}"),
        MetricKind::GoodputBps => format!("{:.1}M", value / 1_000_000.0),
        MetricKind::BandwidthUtilization | MetricKind::ProbeLossRate => {
            format!("{:.0}%", value * 100.0)
        }
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

#[cfg(test)]
mod tests {
    use super::parse_link_capacity_mbps;

    #[test]
    fn parse_link_capacity_allows_off_values() {
        assert_eq!(parse_link_capacity_mbps("").unwrap(), None);
        assert_eq!(parse_link_capacity_mbps("off").unwrap(), None);
        assert_eq!(parse_link_capacity_mbps("none").unwrap(), None);
    }

    #[test]
    fn parse_link_capacity_accepts_numbers() {
        assert_eq!(parse_link_capacity_mbps("100").unwrap(), Some(100.0));
        assert_eq!(parse_link_capacity_mbps("250.5").unwrap(), Some(250.5));
        assert_eq!(parse_link_capacity_mbps("42Mbps").unwrap(), Some(42.0));
    }

    #[test]
    fn parse_link_capacity_rejects_invalid() {
        assert!(parse_link_capacity_mbps("-1").is_err());
        assert!(parse_link_capacity_mbps("abc").is_err());
    }
}
