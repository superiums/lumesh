pub struct LineBuffer {
    chars: Vec<char>,
    cursor: usize,
}

impl LineBuffer {
    pub fn new() -> Self {
        Self { chars: Vec::new(), cursor: 0 }
    }

    pub fn insert(&mut self, c: char) {
        self.chars.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            self.chars.insert(self.cursor, c);
            self.cursor += 1;
        }
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
}
