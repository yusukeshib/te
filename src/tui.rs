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
use std::collections::HashMap;
use std::fs::OpenOptions;

use crate::app::{App, CommandComponent, quote_if_needed};
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
    let components = parse_command(&command_str)?;

    // Extract base_command for history loading
    let base_command: Vec<String> = components
        .iter()
        .filter_map(|c| match c {
            CommandComponent::Base(s) => Some(s.clone()),
            _ => None,
        })
        .collect();

    // Load history
    let history = match crate::history::load_history_for_command(&base_command) {
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

    // Start TUI from the current line
    let mut app = App::new(components, history, cursor_y);
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
                Ok(Some(app.build_final_command()))
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

            // Render only the preview line as the main UI
            let preview_area = ratatui::layout::Rect {
                x: area.x,
                y: start_y,
                width: area.width,
                height: 1,
            };

            // Build styled preview with highlighted selected component
            let selected = app.list_state.selected().unwrap_or(0);
            let mut spans = vec![Span::raw("> ")];
            let mut cursor_offset = 2u16; // Start after "> "
            let mut target_cursor_offset = None;

            for (i, component) in app.components.iter().enumerate() {
                // Skip line breaks in rendering
                if matches!(component, CommandComponent::LineBreak) {
                    continue;
                }

                let text = if app.input_mode && i == selected {
                    // Show current input for the selected component when in input mode
                    quote_if_needed(&app.current_input)
                } else {
                    match component {
                        CommandComponent::Base(s) => quote_if_needed(s),
                        CommandComponent::Flag(s) => quote_if_needed(s),
                        CommandComponent::Value(s) => quote_if_needed(s),
                        CommandComponent::LineBreak => unreachable!(), // Already skipped above
                    }
                };

                if !text.is_empty() {
                    let style = if i == selected {
                        if app.input_mode {
                            Style::default().add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().add_modifier(Modifier::REVERSED)
                        }
                    } else {
                        Style::default()
                    };

                    // Calculate cursor position if this is the selected component in input mode
                    if app.input_mode && i == selected {
                        target_cursor_offset = Some(cursor_offset + app.current_input.len() as u16);
                    }

                    let text_len = text.len() as u16;
                    spans.push(Span::styled(text, style));

                    // Add arrow indicator if this component has multiple options
                    if matches!(component, CommandComponent::Value(_))
                        && app
                            .history_options
                            .get(&i)
                            .map(|opts| opts.len() > 1)
                            .unwrap_or(false)
                    {
                        spans.push(Span::styled(
                            "▼",
                            Style::default().add_modifier(Modifier::DIM),
                        ));
                        cursor_offset += text_len + 1;
                    } else {
                        cursor_offset += text_len;
                    }

                    spans.push(Span::raw(" "));
                    cursor_offset += 1;
                }
            }

            let preview = Paragraph::new(Line::from(spans));
            f.render_widget(preview, preview_area);

            // Set cursor position if in input mode
            if app.input_mode {
                if let Some(offset) = target_cursor_offset {
                    f.set_cursor_position((preview_area.x + offset, preview_area.y));
                }
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
                    KeyCode::Right => app.next(),
                    KeyCode::Left => app.previous(),
                    KeyCode::Up => app.previous_option(),
                    KeyCode::Down => app.next_option(),
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
