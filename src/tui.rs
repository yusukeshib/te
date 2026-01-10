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
    layout::Constraint,
    style::{Modifier, Style},
    text::Text,
    widgets::{Cell, Row, Table},
};

use std::fs::OpenOptions;

use crate::{
    app::App,
    command::Command,
    wrap_text::{WrapTextLine, wrap_text},
};

/// Prefix characters for row shortcuts: 1-9, then available letters (excluding reserved shortcuts)
const PREFIX_CHARS: [char; 28] = [
    '1', '2', '3', '4', '5', '6', '7', '8', '9', 'b', 'c', 'e', 'f', 'g', 'h', 'l', 'm', 'n', 'o',
    'p', 'r', 's', 't', 'v', 'w', 'x', 'y', 'z',
];

/// Get prefix character for a given row index (0-based)
fn get_prefix_char(index: usize) -> Option<char> {
    PREFIX_CHARS.get(index).copied()
}

/// Get row index for a given prefix character
fn get_index_for_prefix(c: char) -> Option<usize> {
    PREFIX_CHARS.iter().position(|&ch| ch == c)
}

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
                Ok(Some(app.cmd.to_shell_string()))
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
            let input_mode = app.editor.is_editing();

            // Build vertical list of components
            let selected = app.list_state.selected().unwrap_or(0);
            let components: Vec<_> = app.cmd.iter_components().collect();

            let prefix_width: u16 = 3; // " X " where X is the shortcut key
            let text_width = area.width.saturating_sub(prefix_width) as usize;

            // Set editor to update the layout
            app.editor.set_content_width(text_width);

            // Pre-calculate wrapped lines and total height
            let mut wrapped_data: Vec<(String, Vec<WrapTextLine>)> = Vec::new();
            let mut total_height: u16 = 0;

            for (i, component) in components.iter().enumerate() {
                let prefix_char = get_prefix_char(i)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| (i + 1).to_string());
                let prefix = format!(" {} ", prefix_char);

                if input_mode && i == selected {
                    let wrapped_lines = app.editor.lines();
                    total_height += app.editor.content_height();
                    wrapped_data.push((prefix, wrapped_lines));
                } else {
                    let text = component.to_string();
                    let wrapped_lines = wrap_text(&text, text_width);
                    total_height += wrapped_lines.len() as u16;
                    wrapped_data.push((prefix, wrapped_lines));
                };
            }

            // Render area for the vertical list
            let list_area = ratatui::layout::Rect {
                x: area.x,
                y: start_y,
                width: area.width,
                height: total_height.min(area.height.saturating_sub(start_y)),
            };

            // Build rows for the table
            let mut rows = Vec::new();
            let mut row_heights: Vec<u16> = Vec::new();

            for (i, (prefix, wrapped_lines)) in wrapped_data.into_iter().enumerate() {
                let row_height = wrapped_lines.len() as u16;
                row_heights.push(row_height);

                let style = if i == selected {
                    if input_mode {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().add_modifier(Modifier::REVERSED)
                    }
                } else {
                    Style::default()
                };

                let wrapped_text = Text::from(
                    wrapped_lines
                        .iter()
                        .map(|line| line.content.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                let row = Row::new(vec![
                    Cell::from(prefix).style(Style::default().add_modifier(Modifier::DIM)),
                    Cell::from(wrapped_text).style(style),
                ])
                .height(row_height);
                rows.push(row);
            }

            let table = Table::new(
                rows,
                [Constraint::Length(prefix_width), Constraint::Fill(1)],
            );
            f.render_widget(table, list_area);

            // Set cursor position if in input mode
            if input_mode {
                // Calculate Y offset by summing heights of rows before the selected row
                let y_offset: u16 = row_heights.iter().take(selected).sum();
                let p = app.editor.cursor_position();
                f.set_cursor_position((
                    list_area.x + prefix_width + p[0] as u16 + 1,
                    list_area.y + y_offset + p[1] as u16,
                ));
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            let input_mode = app.editor.is_editing();

            if input_mode {
                match key.code {
                    KeyCode::Enter if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    KeyCode::Enter => app.confirm_input(),
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Delete => {
                        app.editor.delete_forward();
                    }
                    KeyCode::Backspace => {
                        app.editor.delete_backward();
                    }
                    KeyCode::Up => {
                        app.editor.move_cursor_up();
                    }
                    KeyCode::Down => {
                        app.editor.move_cursor_down();
                    }
                    KeyCode::Left => {
                        app.editor.move_cursor_left();
                    }
                    KeyCode::Right => {
                        app.editor.move_cursor_right();
                    }
                    KeyCode::Char('x') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(false);
                    }
                    KeyCode::Char(c) => app.editor.input_char(c),
                    _ => {}
                }
            } else {
                match key.code {
                    // Ctrl+* shortcuts (must come before non-modifier versions)
                    KeyCode::Char('r') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.redo();
                    }
                    KeyCode::Char('y') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.redo();
                    }
                    KeyCode::Char('n') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.select_next_component();
                    }
                    KeyCode::Char('p') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.select_previous_component();
                    }
                    KeyCode::Char('a') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.list_state.select(Some(0));
                    }
                    KeyCode::Char('e') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        let count = app.cmd.iter_components().count();
                        if count > 0 {
                            app.list_state.select(Some(count - 1));
                        }
                    }
                    KeyCode::Char('z') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.undo();
                    }
                    KeyCode::Char('Z') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.redo();
                    }
                    KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.delete_selected_component();
                    }
                    KeyCode::Enter if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(false);
                    }
                    KeyCode::Char('x') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    // Non-modifier shortcuts
                    KeyCode::Char('u') => {
                        app.undo();
                    }
                    KeyCode::Char('i') => {
                        app.insert_new_component();
                        app.start_input();
                    }
                    KeyCode::Char('a') => {
                        app.append_new_component();
                        app.start_input();
                    }
                    KeyCode::Char('d') | KeyCode::Delete | KeyCode::Backspace => {
                        app.delete_selected_component()
                    }
                    KeyCode::Down | KeyCode::Char('j') => app.select_next_component(),
                    KeyCode::Up | KeyCode::Char('k') => app.select_previous_component(),
                    KeyCode::Home => {
                        app.list_state.select(Some(0));
                    }
                    KeyCode::End => {
                        let count = app.cmd.iter_components().count();
                        if count > 0 {
                            app.list_state.select(Some(count - 1));
                        }
                    }
                    KeyCode::Char('G') => {
                        let count = app.cmd.iter_components().count();
                        if count > 0 {
                            app.list_state.select(Some(count - 1));
                        }
                    }
                    KeyCode::Enter => app.start_input(),
                    KeyCode::Char('q') => return Ok(false),
                    KeyCode::Esc => return Ok(false),
                    KeyCode::Char(c) => {
                        if let Some(index) = get_index_for_prefix(c) {
                            let component_count = app.cmd.iter_components().count();
                            if index < component_count {
                                app.list_state.select(Some(index));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
