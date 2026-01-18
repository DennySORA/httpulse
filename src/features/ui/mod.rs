mod input;
mod render;
mod state;

use crate::app::AppState;
use crate::probe::ProbeSample;
use crossterm::event::{self, Event};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{QueueableCommand, execute};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::io::{self, Stdout, Write};
use std::time::{Duration, Instant};

use input::{
    handle_confirm_delete_key, handle_glossary_key, handle_help_key, handle_input_key,
    handle_normal_key, handle_settings_edit_key, handle_settings_key,
};
use render::{
    draw_confirm_delete_popup, draw_footer, draw_glossary_popup, draw_header, draw_help_popup,
    draw_main, draw_settings_popup, draw_terminal_too_small,
};
use state::{InputMode, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, SettingsState};

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
    let mut glossary_page: usize = 0;
    let mut should_quit = false;
    let mut last_tick = Instant::now();

    while !should_quit {
        while let Ok(sample) = sample_rx.try_recv() {
            app.apply_sample(sample);
        }

        terminal.draw(|frame| {
            let size = frame.area();

            // Check minimum terminal size
            if size.width < MIN_TERMINAL_WIDTH || size.height < MIN_TERMINAL_HEIGHT {
                draw_terminal_too_small(frame, size);
                return;
            }

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
                InputMode::Glossary => draw_glossary_popup(frame, size, glossary_page),
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
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
            match input_mode {
                InputMode::Normal => {
                    if handle_normal_key(
                        key,
                        &mut app,
                        &mut input_mode,
                        &mut input_buffer,
                        &mut settings_state,
                        &mut glossary_page,
                    ) {
                        should_quit = true;
                    }
                }
                InputMode::Help => {
                    handle_help_key(key, &mut input_mode);
                }
                InputMode::Glossary => {
                    handle_glossary_key(key, &mut input_mode, &mut glossary_page);
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
