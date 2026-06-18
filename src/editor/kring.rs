pub struct KillRing {
    entries: Vec<String>,
    max_entries: usize,
}

impl KillRing {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 10,
        }
    }

    pub fn push(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        self.entries.push(text);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    pub fn yank(&self) -> Option<&str> {
        self.entries.last().map(|s| s.as_str())
    }
}
