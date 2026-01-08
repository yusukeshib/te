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

/// Wrap text into lines that fit within the given width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthChar;

    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        // Handle existing line breaks
        if ch == '\n' {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0;
            continue;
        }

        let char_width = UnicodeWidthChar::width(ch).unwrap_or(1);
        if current_width + char_width > width && !current_line.is_empty() {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0;
        }
        current_line.push(ch);
        current_width += char_width;
    }

    if !current_line.is_empty() || lines.is_empty() {
        lines.push(current_line);
    }

    lines
}
use std::fs::OpenOptions;

use crate::{app::App, command::Command};

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

            let prefix_width: u16 = 3; // " X " where X is the shortcut key
            let text_width = area.width.saturating_sub(prefix_width) as usize;

            // Pre-calculate wrapped lines and total height
            let mut wrapped_data: Vec<(String, Vec<String>)> = Vec::new();
            let mut total_height: u16 = 0;

            for (i, component) in components.iter().enumerate() {
                let text = if app.input_mode && i == selected {
                    app.current_input.clone()
                } else {
                    component.to_string()
                };

                let prefix_char = get_prefix_char(i)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| (i + 1).to_string());
                let prefix = format!(" {} ", prefix_char);

                let wrapped_lines = wrap_text(&text, text_width);
                total_height += wrapped_lines.len() as u16;
                wrapped_data.push((prefix, wrapped_lines));
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
            let mut cursor_row = 0u16;
            let mut cursor_col = 0u16;
            let mut cumulative_height: u16 = 0;

            for (i, (prefix, wrapped_lines)) in wrapped_data.into_iter().enumerate() {
                let row_height = wrapped_lines.len() as u16;

                let style = if i == selected {
                    if app.input_mode {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().add_modifier(Modifier::REVERSED)
                    }
                } else {
                    Style::default()
                };

                if app.input_mode && i == selected {
                    // Place cursor on the last wrapped line of the current input,
                    // and at the end of that line using Unicode display width.
                    use unicode_width::UnicodeWidthStr;
                    let last_line_width = wrapped_lines
                        .last()
                        .map(|line| UnicodeWidthStr::width(line.as_str()) as u16)
                        .unwrap_or(0);
                    cursor_row = cumulative_height + row_height.saturating_sub(1);
                    cursor_col = prefix_width + last_line_width;
                }

                let wrapped_text = Text::from(wrapped_lines.join("\n"));
                let row = Row::new(vec![
                    Cell::from(prefix).style(Style::default().add_modifier(Modifier::DIM)),
                    Cell::from(wrapped_text).style(style),
                ])
                .height(row_height);
                rows.push(row);

                cumulative_height += row_height;
            }

            let table = Table::new(
                rows,
                [Constraint::Length(prefix_width), Constraint::Fill(1)],
            );
            f.render_widget(table, list_area);

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
                    KeyCode::Enter if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    KeyCode::Enter => app.confirm_input(),
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Backspace => {
                        app.current_input.pop();
                    }
                    KeyCode::Char('x') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(false);
                    }
                    KeyCode::Char(c) => app.current_input.push(c),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('u') => {
                        app.undo();
                    }
                    KeyCode::Char('r') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.redo();
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
                    KeyCode::Enter if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    KeyCode::Enter => app.start_input(),
                    KeyCode::Char('q') => return Ok(false),
                    KeyCode::Esc => return Ok(false),
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(false);
                    }
                    KeyCode::Char('x') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_empty_string() {
        assert_eq!(wrap_text("", 10), vec![""]);
    }

    #[test]
    fn test_wrap_text_zero_width() {
        assert_eq!(wrap_text("hello", 0), vec!["hello"]);
    }

    #[test]
    fn test_wrap_text_fits_within_width() {
        assert_eq!(wrap_text("hello", 10), vec!["hello"]);
    }

    #[test]
    fn test_wrap_text_exact_width() {
        assert_eq!(wrap_text("hello", 5), vec!["hello"]);
    }

    #[test]
    fn test_wrap_text_exceeds_width() {
        assert_eq!(wrap_text("hello world", 6), vec!["hello ", "world"]);
    }

    #[test]
    fn test_wrap_text_long_word_must_break() {
        assert_eq!(wrap_text("abcdefghij", 5), vec!["abcde", "fghij"]);
    }

    #[test]
    fn test_wrap_text_preserves_newlines() {
        assert_eq!(wrap_text("hello\nworld", 20), vec!["hello", "world"]);
    }

    #[test]
    fn test_wrap_text_newline_and_wrap() {
        assert_eq!(
            wrap_text("hello\nworld test", 6),
            vec!["hello", "world ", "test"]
        );
    }

    #[test]
    fn test_wrap_text_multiple_spaces() {
        // Width 8: "hello  " (7) + "w" (1) = 8, fits on first line
        assert_eq!(wrap_text("hello  world", 8), vec!["hello  w", "orld"]);
        // Width 7: "hello  " (7) fits exactly, "world" goes to next line
        assert_eq!(wrap_text("hello  world", 7), vec!["hello  ", "world"]);
    }

    #[test]
    fn test_wrap_text_wide_characters_cjk() {
        // CJK characters are typically 2 display units wide
        // "你好" = 4 display units, "世界" = 4 display units
        assert_eq!(wrap_text("你好世界", 4), vec!["你好", "世界"]);
    }

    #[test]
    fn test_wrap_text_wide_characters_mixed() {
        // "a" = 1, "你" = 2, "b" = 1 -> total 4 display units
        assert_eq!(wrap_text("a你b", 4), vec!["a你b"]);
        assert_eq!(wrap_text("a你b", 3), vec!["a你", "b"]);
    }

    #[test]
    fn test_wrap_text_emoji() {
        // Most emojis are 2 display units wide
        assert_eq!(wrap_text("ab", 4), vec!["ab"]);
    }

    #[test]
    fn test_wrap_text_long_sentence() {
        assert_eq!(
            wrap_text("the quick brown fox", 10),
            vec!["the quick ", "brown fox"]
        );
    }

    #[test]
    fn test_wrap_text_trailing_space() {
        assert_eq!(wrap_text("hello ", 10), vec!["hello "]);
    }

    #[test]
    fn test_wrap_text_leading_space() {
        assert_eq!(wrap_text(" hello", 10), vec![" hello"]);
    }
}
