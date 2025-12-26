use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};
use std::collections::HashMap;
use std::fs::OpenOptions;

use crate::app::{App, Value};
use crate::command_parser::parse_command;

/// Get cursor position by querying /dev/tty directly using ANSI escape codes
fn get_cursor_position(tty: &mut std::fs::File) -> Result<(u16, u16)> {
    use std::io::{Read, Write};

    // Query cursor position with ANSI escape code
    tty.write_all(b"\x1b[6n")?;
    tty.flush()?;

    // Read response: ESC [ row ; col R
    let mut buf = [0u8; 32];
    let mut response = Vec::new();

    // Set a simple timeout by reading byte by byte
    for _ in 0..32 {
        match tty.read(&mut buf[0..1]) {
            Ok(1) => {
                response.push(buf[0]);
                if buf[0] == b'R' {
                    break;
                }
            }
            _ => break,
        }
    }

    // Parse response: ESC [ {row} ; {col} R
    let response_str = String::from_utf8_lossy(&response);
    if let Some(pos_str) = response_str.strip_prefix("\x1b[") {
        if let Some(pos_str) = pos_str.strip_suffix('R') {
            if let Some((row_str, col_str)) = pos_str.split_once(';') {
                if let (Ok(row), Ok(col)) = (row_str.parse::<u16>(), col_str.parse::<u16>()) {
                    // Convert from 1-based to 0-based
                    return Ok((col.saturating_sub(1), row.saturating_sub(1)));
                }
            }
        }
    }

    Ok((0, 0))
}

pub fn run_tui(command_str: String) -> Result<Option<String>> {
    let parsed = parse_command(&command_str)?;

    // if parsed.arguments.is_empty() {
    //     println!("No arguments to edit. Command: {}", command_str);
    //     return Ok(Some(command_str));
    // }

    // Load history
    let history = match crate::history::load_history_for_command(&parsed.base_command) {
        Ok(h) => h,
        Err(_) => HashMap::new(),
    };

    // Enable raw mode first to prevent escape sequences from echoing
    enable_raw_mode()?;

    // Get cursor position from /dev/tty
    let (cursor_x, cursor_y) = {
        let mut tty_read = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
        get_cursor_position(&mut tty_read).unwrap_or((0, 0))
    };

    // Open /dev/tty directly for both reading and writing (like fzf does)
    // This allows the TUI to work inside command substitution
    let mut tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
    execute!(tty, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(tty);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Fullscreen,
        },
    )?;

    let mut app = App::new(parsed.base_command, parsed.arguments, history, cursor_y);
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;

    // Clear the TUI content and restore cursor to original position
    let backend = terminal.backend_mut();
    execute!(
        backend,
        DisableMouseCapture,
        cursor::MoveTo(cursor_x, cursor_y),
        Clear(ClearType::FromCursorDown)
    )?;
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
            let area = f.area();

            // Start from the cursor position
            let start_y = app.cursor_y;

            // Help text on first line
            let help_area = ratatui::layout::Rect {
                x: area.x,
                y: start_y,
                width: area.width,
                height: 1,
            };
            let help = Paragraph::new(
                "↑/↓: navigate, ←/→: history, Space: toggle, Enter: edit, Ctrl+X: execute, ESC: cancel",
            )
            .style(Style::default().fg(Color::DarkGray));
            f.render_widget(help, help_area);

            // Render each argument
            let selected = app.list_state.selected().unwrap_or(0);
            for (i, arg) in app.arguments.iter().enumerate() {
                let row_area = ratatui::layout::Rect {
                    x: area.x,
                    y: start_y + 1 + i as u16,
                    width: area.width,
                    height: 1,
                };

                // Argument name (max 20 chars)
                let name_display = if arg.flag.is_empty() {
                    "(positional)".to_string()
                } else {
                    arg.flag.clone()
                };
                let name_display = if name_display.len() > 20 {
                    format!("{}...", &name_display[..17])
                } else {
                    name_display
                };

                // Apply style based on selection and input mode
                let (name_style, value_style) = if i == selected {
                    if app.input_mode {
                        // Actively editing: use dark background with white text
                        (
                            Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD),
                            Style::default().fg(Color::White).bg(Color::DarkGray),
                        )
                    } else {
                        // Selected but not editing: darker gray background with bold
                        (
                            Style::default().fg(Color::White).bg(Color::Rgb(60, 60, 60)).add_modifier(Modifier::BOLD),
                            Style::default().fg(Color::White).bg(Color::Rgb(60, 60, 60)).add_modifier(Modifier::BOLD),
                        )
                    }
                } else {
                    (
                        Style::default().fg(Color::Gray),
                        Style::default().fg(Color::White),
                    )
                };

                // Name area (left 20 chars)
                let name_area = ratatui::layout::Rect {
                    x: row_area.x,
                    y: row_area.y,
                    width: 20,
                    height: 1,
                };

                let name_widget = Paragraph::new(name_display).style(name_style);
                f.render_widget(name_widget, name_area);

                // Value display (right side) depends on Value type
                match &arg.value {
                    Value::Checked(checked) => {
                        // Layout: [name 20 chars] [checkbox flex]
                        let checkbox_area = ratatui::layout::Rect {
                            x: row_area.x + 20,
                            y: row_area.y,
                            width: row_area.width.saturating_sub(20),
                            height: 1,
                        };

                        let display = if *checked { "TRUE" } else { "FALSE" };
                        let checkbox_widget = Paragraph::new(display).style(value_style);
                        f.render_widget(checkbox_widget, checkbox_area);
                    }
                    Value::String(s) => {
                        let mut display = if app.input_mode && i == selected {
                            app.current_input.clone()
                        } else {
                            s.clone()
                        };

                        // Add (X/Y) if history options exist
                        if !app.input_mode || i != selected {
                            if let Some((current, total)) = app.get_option_status(i) {
                                display = format!("{} ({}/{})", display, current, total);
                            }
                        }

                        // Layout: [name 20 chars] [value flex]
                        let value_area = ratatui::layout::Rect {
                            x: row_area.x + 20,
                            y: row_area.y,
                            width: row_area.width.saturating_sub(20),
                            height: 1,
                        };

                        let value_widget = Paragraph::new(display).style(value_style);
                        f.render_widget(value_widget, value_area);

                        // Show cursor when actively editing
                        if app.input_mode && i == selected {
                            f.set_cursor_position((
                                value_area.x + app.current_input.len() as u16,
                                value_area.y,
                            ));
                        }
                    }
                };
            }
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
                    KeyCode::Right => app.next_option(),
                    KeyCode::Left => app.previous_option(),
                    KeyCode::Char(' ') => app.toggle_checkbox(),
                    KeyCode::Enter => app.handle_enter(),
                    KeyCode::Char('x') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
    }
}
