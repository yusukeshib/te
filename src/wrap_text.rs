#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapTextLine {
    pub content: String,
    pub start_index: usize,
    pub end_index: usize,
}

pub struct WrapText {
    /// (line content, start index, end index)
    lines: Vec<WrapTextLine>,
    content: String,
    width: usize,
}

impl WrapText {
    /// Wrap text into lines that fit within the given width
    pub fn new(text: &str, width: usize) -> Self {
        let lines = wrap_text(text, width);
        Self {
            lines,
            content: text.to_string(),
            width,
        }
    }

    pub fn index_at_position(&self, position: [usize; 2]) -> usize {
        use unicode_width::UnicodeWidthChar;

        let line_num = position[1];
        let target_col = position[0];

        if line_num >= self.lines.len() {
            return self.content.len();
        }

        let line = &self.lines[line_num];
        let mut current_col = 0;
        let mut byte_offset = 0;

        for ch in line.content.chars() {
            if current_col >= target_col {
                break;
            }
            current_col += UnicodeWidthChar::width(ch).unwrap_or(1);
            byte_offset += ch.len_utf8();
        }

        line.start_index + byte_offset
    }

    pub fn position(&self, index: usize) -> [usize; 2] {
        use unicode_width::UnicodeWidthChar;

        let mut line_num = 0;
        let mut col_num = 0;
        for line in &self.lines {
            if index >= line.start_index && index <= line.end_index {
                // Calculate display width of characters before the cursor
                let byte_offset = index - line.start_index;
                col_num = line.content[..byte_offset]
                    .chars()
                    .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
                    .sum();
                break;
            }
            line_num += 1;
        }
        [col_num, line_num]
    }

    pub fn lines(&self) -> &Vec<WrapTextLine> {
        &self.lines
    }

    pub fn height(&self) -> usize {
        self.lines.len()
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    fn update(&mut self) {
        self.lines = wrap_text(&self.content, self.width);
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.lines.clear();
    }

    pub fn set_text(&mut self, text: &str) {
        self.content = text.to_string();
        self.update();
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width;
        self.update();
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn insert(&mut self, index: usize, ch: char) {
        self.content.insert(index, ch);
        self.update();
    }

    pub fn remove(&mut self, index: usize) {
        self.content.remove(index);
        self.update();
    }
}

pub fn wrap_text(text: &str, width: usize) -> Vec<WrapTextLine> {
    use unicode_width::UnicodeWidthChar;

    if width == 0 {
        return vec![WrapTextLine {
            content: text.to_string(),
            start_index: 0,
            end_index: text.len(),
        }];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    let mut line_start_index = 0;
    let mut current_index = 0;

    for ch in text.chars() {
        let char_len = ch.len_utf8();

        // Handle existing line breaks
        if ch == '\n' {
            lines.push(WrapTextLine {
                content: current_line,
                start_index: line_start_index,
                end_index: current_index,
            });
            current_line = String::new();
            current_width = 0;
            current_index += char_len;
            line_start_index = current_index;
            continue;
        }

        let char_width = UnicodeWidthChar::width(ch).unwrap_or(1);
        if current_width + char_width > width && !current_line.is_empty() {
            lines.push(WrapTextLine {
                content: current_line,
                start_index: line_start_index,
                end_index: current_index,
            });
            current_line = String::new();
            current_width = 0;
            line_start_index = current_index;
        }
        current_line.push(ch);
        current_width += char_width;
        current_index += char_len;
    }

    if !current_line.is_empty() || lines.is_empty() {
        lines.push(WrapTextLine {
            content: current_line,
            start_index: line_start_index,
            end_index: current_index,
        });
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contents(lines: Vec<WrapTextLine>) -> Vec<String> {
        lines.into_iter().map(|l| l.content).collect()
    }

    #[test]
    fn test_wrap_text_empty_string() {
        assert_eq!(contents(wrap_text("", 10)), vec![""]);
    }

    #[test]
    fn test_wrap_text_zero_width() {
        assert_eq!(contents(wrap_text("hello", 0)), vec!["hello"]);
    }

    #[test]
    fn test_wrap_text_fits_within_width() {
        assert_eq!(contents(wrap_text("hello", 10)), vec!["hello"]);
    }

    #[test]
    fn test_wrap_text_exact_width() {
        assert_eq!(contents(wrap_text("hello", 5)), vec!["hello"]);
    }

    #[test]
    fn test_wrap_text_exceeds_width() {
        assert_eq!(
            contents(wrap_text("hello world", 6)),
            vec!["hello ", "world"]
        );
    }

    #[test]
    fn test_wrap_text_long_word_must_break() {
        assert_eq!(contents(wrap_text("abcdefghij", 5)), vec!["abcde", "fghij"]);
    }

    #[test]
    fn test_wrap_text_preserves_newlines() {
        assert_eq!(
            contents(wrap_text("hello\nworld", 20)),
            vec!["hello", "world"]
        );
    }

    #[test]
    fn test_wrap_text_newline_and_wrap() {
        assert_eq!(
            contents(wrap_text("hello\nworld test", 6)),
            vec!["hello", "world ", "test"]
        );
    }

    #[test]
    fn test_wrap_text_multiple_spaces() {
        // Width 8: "hello  " (7) + "w" (1) = 8, fits on first line
        assert_eq!(
            contents(wrap_text("hello  world", 8)),
            vec!["hello  w", "orld"]
        );
        // Width 7: "hello  " (7) fits exactly, "world" goes to next line
        assert_eq!(
            contents(wrap_text("hello  world", 7)),
            vec!["hello  ", "world"]
        );
    }

    #[test]
    fn test_wrap_text_wide_characters_cjk() {
        // CJK characters are typically 2 display units wide
        // "你好" = 4 display units, "世界" = 4 display units
        assert_eq!(contents(wrap_text("你好世界", 4)), vec!["你好", "世界"]);
    }

    #[test]
    fn test_wrap_text_wide_characters_mixed() {
        // "a" = 1, "你" = 2, "b" = 1 -> total 4 display units
        assert_eq!(contents(wrap_text("a你b", 4)), vec!["a你b"]);
        assert_eq!(contents(wrap_text("a你b", 3)), vec!["a你", "b"]);
    }

    #[test]
    fn test_wrap_text_emoji() {
        // Most emojis are 2 display units wide
        assert_eq!(contents(wrap_text("ab", 4)), vec!["ab"]);
    }

    #[test]
    fn test_wrap_text_long_sentence() {
        assert_eq!(
            contents(wrap_text("the quick brown fox", 10)),
            vec!["the quick ", "brown fox"]
        );
    }

    #[test]
    fn test_wrap_text_trailing_space() {
        assert_eq!(contents(wrap_text("hello ", 10)), vec!["hello "]);
    }

    #[test]
    fn test_wrap_text_leading_space() {
        assert_eq!(contents(wrap_text(" hello", 10)), vec![" hello"]);
    }
}
