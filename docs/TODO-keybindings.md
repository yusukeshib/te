# Keybinding Improvements TODO

## Completed

- [x] Change undo from `z` to `u` (vim-style)
- [x] Change redo from `Z` to `Ctrl+R` (vim-style)
- [x] Add `Shift+Enter` for execute (keeps `Ctrl+X` as fallback)

## Future: Input Mode Readline Keybindings

Add full readline/emacs keybindings to input mode. Requires adding `cursor_pos` field to track cursor position within `current_input`.

### Cursor Movement

- [ ] `Ctrl+A` / `Home` - Move to beginning of line
- [ ] `Ctrl+E` / `End` - Move to end of line
- [ ] `Ctrl+B` / `Left` - Move back one character
- [ ] `Ctrl+F` / `Right` - Move forward one character

### Deletion

- [ ] `Ctrl+H` / `Backspace` - Delete character before cursor (already have Backspace)
- [ ] `Ctrl+D` - Delete character at cursor (or cancel if empty)
- [ ] `Ctrl+K` - Kill (delete) to end of line
- [ ] `Ctrl+U` - Kill (delete) to beginning of line
- [ ] `Ctrl+W` - Delete word backward

### Implementation Notes

1. Add `cursor_pos: usize` field to `App` struct in `src/app.rs`
2. Add helper methods:
   - `move_cursor_left()` / `move_cursor_right()`
   - `move_cursor_to_start()` / `move_cursor_to_end()`
   - `delete_char_at_cursor()`
   - `kill_to_end()` / `kill_to_start()`
   - `delete_word_backward()`
   - `insert_char_at_cursor(c: char)`
3. Update `start_input()` to set `cursor_pos = current_input.len()`
4. Update cursor rendering in `tui.rs` to use `cursor_pos` instead of `current_input.len()`
5. Handle Unicode properly with `chars().count()` for cursor positioning
