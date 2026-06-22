pub struct LineBuffer {
    chars: Vec<char>,
    cursor: usize,
}

impl Default for LineBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl LineBuffer {
    pub fn new() -> Self {
        Self {
            chars: Vec::new(),
            cursor: 0,
        }
    }

    pub fn insert(&mut self, c: char) {
        self.chars.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub fn insert_str(&mut self, s: &str) {
        let new_chars: Vec<char> = s.chars().collect();
        let len = new_chars.len();
        self.chars.splice(self.cursor..self.cursor, new_chars);
        self.cursor += len;
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.chars.remove(self.cursor);
        true
    }

    pub fn delete(&mut self) -> bool {
        if self.cursor >= self.chars.len() {
            return false;
        }
        self.chars.remove(self.cursor);
        true
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    pub fn move_to_start(&mut self) {
        self.cursor = 0;
    }

    pub fn move_to_end(&mut self) {
        self.cursor = self.chars.len();
    }

    pub fn move_word_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut found_non_space = false;
        for i in (0..self.cursor).rev() {
            if self.chars[i] == ' ' {
                if found_non_space {
                    self.cursor = i + 1;
                    return;
                }
            } else {
                found_non_space = true;
            }
        }
        self.cursor = 0;
    }

    pub fn move_word_right(&mut self) {
        if self.cursor >= self.chars.len() {
            return;
        }
        let mut found_non_space = false;
        for i in self.cursor..self.chars.len() {
            if self.chars[i] == ' ' {
                if found_non_space {
                    self.cursor = i;
                    return;
                }
            } else {
                found_non_space = true;
            }
        }
        self.cursor = self.chars.len();
    }

    pub fn delete_word_before(&mut self) -> Option<String> {
        if self.cursor == 0 {
            return None;
        }
        let start = {
            let mut found_non_space = false;
            let mut pos = 0;
            for i in (0..self.cursor).rev() {
                if self.chars[i] == ' ' {
                    if found_non_space {
                        pos = i + 1;
                        break;
                    }
                } else {
                    found_non_space = true;
                }
            }
            pos
        };
        let killed: String = self.chars.drain(start..self.cursor).collect();
        self.cursor = start;
        Some(killed)
    }

    pub fn find_prev_newline(&self) -> Option<usize> {
        self.chars[..self.cursor].iter().rposition(|&c| c == '\n')
    }

    pub fn find_next_newline(&self) -> Option<usize> {
        self.chars[self.cursor..]
            .iter()
            .position(|&c| c == '\n')
            .map(|i| self.cursor + i)
    }

    pub fn line_start(&self) -> usize {
        self.find_prev_newline().map(|i| i + 1).unwrap_or(0)
    }

    pub fn line_end(&self) -> usize {
        self.find_next_newline().unwrap_or(self.chars.len())
    }

    pub fn col_in_line(&self) -> usize {
        self.cursor - self.line_start()
    }

    pub fn cursor_at_indent(&self) -> bool {
        self.chars[self.line_start()..self.cursor]
            .iter()
            .all(|&c| c == ' ' || c == '\t')
    }

    pub fn cursor_on_empty_line(&self) -> bool {
        self.chars[self.line_start()..self.line_end()]
            .iter()
            .all(|&c| c == ' ' || c == '\t')
    }

    pub fn current_line_indent(&self) -> String {
        self.chars[self.line_start()..]
            .iter()
            .take_while(|&&c| c == ' ' || c == '\t')
            .collect()
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor = self.line_start();
    }

    pub fn move_to_line_end(&mut self) {
        self.cursor = self.line_end();
    }

    pub fn move_cursor_up_line(&mut self) {
        if let Some(prev_newline) = self.find_prev_newline() {
            let col = self.col_in_line();
            let prev_line_start = self.chars[..prev_newline]
                .iter()
                .rposition(|&c| c == '\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let prev_len = prev_newline - prev_line_start;
            self.cursor = prev_line_start + col.min(prev_len);
        }
    }

    pub fn move_cursor_down_line(&mut self) {
        if let Some(next_newline) = self.find_next_newline() {
            let col = self.col_in_line();
            let after_newline = next_newline + 1;
            let remaining = &self.chars[after_newline..];
            let next_len = remaining
                .iter()
                .position(|&c| c == '\n')
                .unwrap_or(remaining.len());
            self.cursor = after_newline + col.min(next_len);
        }
    }

    pub fn delete_to_start(&mut self) -> Option<String> {
        if self.cursor == 0 {
            return None;
        }
        let killed: String = self.chars.drain(..self.cursor).collect();
        self.cursor = 0;
        Some(killed)
    }

    pub fn delete_to_end(&mut self) -> Option<String> {
        if self.cursor >= self.chars.len() {
            return None;
        }
        let killed: String = self.chars.drain(self.cursor..).collect();
        Some(killed)
    }

    pub fn delete_to_line_start(&mut self) -> Option<String> {
        let line_start = self.line_start();
        if self.cursor == line_start {
            return None;
        }
        let killed: String = self.chars.drain(line_start..self.cursor).collect();
        self.cursor = line_start;
        Some(killed)
    }

    pub fn delete_to_line_end(&mut self) -> Option<String> {
        let line_end = self.line_end();
        if self.cursor >= line_end {
            return None;
        }
        let killed: String = self.chars.drain(self.cursor..line_end).collect();
        Some(killed)
    }

    pub fn delete_word_after(&mut self) -> Option<String> {
        if self.cursor >= self.chars.len() {
            return None;
        }
        let end = {
            let mut found_non_space = false;
            let mut pos = self.chars.len();
            for i in self.cursor..self.chars.len() {
                if self.chars[i] == ' ' {
                    if found_non_space {
                        pos = i;
                        break;
                    }
                } else {
                    found_non_space = true;
                }
            }
            pos
        };
        let killed: String = self.chars.drain(self.cursor..end).collect();
        Some(killed)
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn byte_cursor(&self) -> usize {
        self.chars.iter().take(self.cursor).map(|c| c.len_utf8()).sum()
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn set_text(&mut self, text: &str) {
        self.chars = text.chars().collect();
        self.cursor = self.chars.len();
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.chars.len());
    }

    pub fn chars_before_cursor(&self) -> String {
        self.chars[..self.cursor].iter().collect()
    }

    pub fn chars_after_cursor(&self) -> String {
        self.chars[self.cursor..].iter().collect()
    }

    pub fn replace_range(&mut self, start: usize, end: usize, text: &str) {
        let end = end.min(self.chars.len());
        self.chars.drain(start..end);
        let new_chars: Vec<char> = text.chars().collect();
        let len = new_chars.len();
        self.chars.splice(start..start, new_chars);
        self.cursor = start + len;
    }

    pub fn transpose_chars(&mut self) {
        let pos = self.cursor;
        if pos == 0 || self.chars.len() < 2 {
            return;
        }
        let swap_pos = if pos == self.chars.len() {
            pos - 2
        } else {
            pos - 1
        };
        self.chars.swap(swap_pos, swap_pos + 1);
        self.cursor = (swap_pos + 2).min(self.chars.len());
    }
}
