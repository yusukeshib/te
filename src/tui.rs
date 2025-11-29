use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal, TerminalOptions, Viewport,
};
use std::io;

use crate::app::App;
use crate::command_parser::parse_command;

pub fn run_tui(command_str: String) -> Result<Option<String>> {
    let parsed = parse_command(&command_str)?;

    if parsed.arguments.is_empty() {
        println!("No arguments to edit. Command: {}", command_str);
        return Ok(Some(command_str));
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(14),
        },
    )?;

    let mut app = App::new(parsed.base_command, parsed.arguments);
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture)?;
    terminal.show_cursor()?;

    match result {
        Ok(should_execute) => {
            if should_execute {
                Ok(Some(app.preview_command))
            } else {
                Ok(None)
            }
        }
        Err(err) => Err(err),
    }
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<bool> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ])
                .split(f.area());

            let title = Paragraph::new(format!(
                "te - Interactive CLI Helper | Command: {}",
                app.base_command.join(" ")
            ))
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));
            f.render_widget(title, chunks[0]);

            let items: Vec<ListItem> = app
                .arguments
                .iter()
                .map(|arg| {
                    let value_display = arg
                        .value
                        .as_ref()
                        .map(|v| v.as_str())
                        .unwrap_or("<not set>");

                    let content = if arg.flag.is_empty() {
                        format!("(positional) = {}", value_display)
                    } else {
                        format!("{} = {}", arg.flag, value_display)
                    };

                    ListItem::new(content)
                })
                .collect();

            let list =
                List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(
                        "Arguments (↑/↓: navigate, Enter: edit, Ctrl+X: execute, ESC: cancel)",
                    ))
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");

            f.render_stateful_widget(list, chunks[1], &mut app.list_state);

            if app.input_mode {
                let input = Paragraph::new(app.current_input.as_str())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Input Value (Enter: confirm, ESC: cancel)"),
                    )
                    .style(Style::default().fg(Color::Yellow));
                f.render_widget(input, chunks[2]);
            } else {
                let help_text = Paragraph::new("Press Enter to edit selected argument")
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                f.render_widget(help_text, chunks[2]);
            }

            let preview = Paragraph::new(app.preview_command.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Command Preview"),
                )
                .style(Style::default().fg(Color::Green));
            f.render_widget(preview, chunks[3]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if app.input_mode {
                match key.code {
                    KeyCode::Enter => app.confirm_input(),
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Char(c) => app.current_input.push(c),
                    KeyCode::Backspace => {
                        app.current_input.pop();
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => return Ok(false),
                    KeyCode::Esc => return Ok(false),
                    KeyCode::Down => app.next(),
                    KeyCode::Up => app.previous(),
                    KeyCode::Enter => app.start_input(),
                    KeyCode::Char('x') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
    }
}

