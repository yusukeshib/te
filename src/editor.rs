use crate::wrap_text::{WrapText, WrapTextLine};

pub struct Editor {
    content: WrapText,
    cursor_index: usize,
    is_editing: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            content: WrapText::new("", 80),
            cursor_index: 0,
            is_editing: false,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.is_editing
    }

    pub fn start_input(&mut self, value: &str) {
        self.content.set_text(value);
        self.cursor_index = self.content.len();
        self.is_editing = true;
    }

    pub fn end_input(&mut self) -> String {
        self.is_editing = false;
        let value = self.content.text().to_string();
        self.clear();
        value
    }

    pub fn cancel_input(&mut self) {
        self.is_editing = false;
        self.clear();
    }

    pub fn set_content_width(&mut self, width: usize) {
        self.content.set_width(width);
    }

    pub fn content_height(&self) -> u16 {
        self.content.height() as u16
    }

    pub fn lines(&self) -> Vec<WrapTextLine> {
        self.content.lines().clone()
    }

    pub fn cursor_position(&self) -> [usize; 2] {
        self.content.position(self.cursor_index)
    }

    pub fn move_cursor_up(&mut self) {
        let p = self.content.position(self.cursor_index);
        let next_index = self.content.index_at_position([p[0], p[1] - 1]);
        self.cursor_index = next_index;
    }

    pub fn move_cursor_down(&mut self) {
        let p = self.content.position(self.cursor_index);
        let next_index = self.content.index_at_position([p[0], p[1] + 1]);
        self.cursor_index = next_index;
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_index > 0 {
            let text = self.content.text();
            let mut new_idx = self.cursor_index - 1;
            while new_idx > 0 && !text.is_char_boundary(new_idx) {
                new_idx -= 1;
            }
            self.cursor_index = new_idx;
        }
    }

    pub fn move_cursor_right(&mut self) {
        let text = self.content.text();
        if self.cursor_index < text.len() {
            let mut new_idx = self.cursor_index + 1;
            while new_idx < text.len() && !text.is_char_boundary(new_idx) {
                new_idx += 1;
            }
            self.cursor_index = new_idx;
        }
    }

    /// Delete the character at the cursor position (Delete key behavior)
    pub fn delete_forward(&mut self) {
        let pos = self.cursor_index;
        if pos < self.content.len() {
            self.content.remove(pos);
        }
    }

    /// Delete the character at the cursor position -1 (Backspace key behavior)
    pub fn delete_backward(&mut self) {
        if self.cursor_index > 0 {
            let text = self.content.text();
            let mut prev_idx = self.cursor_index - 1;
            while prev_idx > 0 && !text.is_char_boundary(prev_idx) {
                prev_idx -= 1;
            }
            self.content.remove(prev_idx);
            self.cursor_index = prev_idx;
        }
    }

    /// Add a character at the cursor position
    pub fn input_char(&mut self, c: char) {
        let pos = self.cursor_index;
        self.content.insert(pos, c);
        self.cursor_index += c.len_utf8();
    }

    fn clear(&mut self) {
        self.content.clear();
        self.cursor_index = 0;
    }
}
