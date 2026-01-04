use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::fs::OpenOptions;

use crate::{app::App, command::Command};

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

pub fn run_tui(command_str: &str) -> Result<Option<String>> {
    let cmd: Command = command_str.try_into()?;

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

    // Start TUI from the current line
    let mut app = App::new(cmd, cursor_y);
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;

    // Clear the TUI content from the current line down
    let backend = terminal.backend_mut();
    execute!(
        backend,
        DisableMouseCapture,
        cursor::MoveTo(0, cursor_y),
        Clear(ClearType::FromCursorDown),
        cursor::MoveTo(cursor_x, cursor_y)
    )?;
    terminal.show_cursor()?;

    match result {
        Ok(should_execute) => {
            if should_execute {
                Ok(Some(app.cmd.to_string()))
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

            // Build vertical list of components
            let selected = app.list_state.selected().unwrap_or(0);
            let components: Vec<_> = app.cmd.iter_components().collect();
            let component_count = components.len() as u16;

            // Render area for the vertical list
            let list_area = ratatui::layout::Rect {
                x: area.x,
                y: start_y,
                width: area.width,
                height: component_count.min(area.height.saturating_sub(start_y)),
            };

            // Build lines for each component
            let mut lines = Vec::new();
            let mut cursor_row = 0u16;
            let mut cursor_col = 0u16;
            let num_width = component_count.to_string().len();

            for (i, component) in components.iter().enumerate() {
                let text = if app.input_mode && i == selected {
                    app.current_input.clone()
                } else {
                    component.to_string()
                };

                let style = if i == selected {
                    if app.input_mode {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().add_modifier(Modifier::REVERSED)
                    }
                } else {
                    Style::default()
                };

                // Track cursor position for input mode
                let prefix = format!(" {:>width$} ", i + 1, width = num_width);
                if app.input_mode && i == selected {
                    cursor_row = i as u16;
                    cursor_col = prefix.len() as u16 + app.current_input.len() as u16;
                }

                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().add_modifier(Modifier::DIM)),
                    Span::styled(text, style),
                ]));
            }

            let list = Paragraph::new(lines);
            f.render_widget(list, list_area);

            // Set cursor position if in input mode
            if app.input_mode {
                f.set_cursor_position((list_area.x + cursor_col, list_area.y + cursor_row));
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
                    KeyCode::Backspace => {
                        app.current_input.pop();
                    }
                    KeyCode::Char(c) => {
                        if c == 'x' && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                            return Ok(true);
                        } else if c == 'c' && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                            return Ok(false);
                        } else {
                            app.current_input.push(c)
                        }
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => return Ok(false),
                    KeyCode::Esc => return Ok(false),
                    KeyCode::Down => app.select_next_component(),
                    KeyCode::Up => app.select_previous_component(),
                    KeyCode::Enter => app.start_input(),
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(false);
                    }
                    KeyCode::Char('x') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
    }
}
