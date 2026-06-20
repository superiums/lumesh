use std::fs::File;
use std::io::{self, Write, BufRead, BufReader};
use std::path::Path;

pub struct History {
    entries: Vec<String>,
    max_entries: usize,
    index: Option<usize>,
    saved_line: String,
    search_query: String,
    search_matches: Vec<usize>,
    search_index: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 1000,
            index: None,
            saved_line: String::new(),
            search_query: String::new(),
            search_matches: Vec::new(),
            search_index: 0,
        }
    }

    pub fn add(&mut self, entry: String) {
        if entry.trim().is_empty() {
            return;
        }
        if let Some(pos) = self.entries.iter().position(|e| e == &entry) {
            self.entries.remove(pos);
        }
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    pub fn previous(&mut self, current_line: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        match self.index {
            None => {
                self.saved_line = current_line.to_string();
                self.index = Some(self.entries.len() - 1);
            }
            Some(i) if i > 0 => {
                self.index = Some(i - 1);
            }
            _ => return None,
        }
        self.index.map(|i| self.entries[i].as_str())
    }

    pub fn next(&mut self, current_line: &str) -> Option<&str> {
        let _ = current_line;
        match self.index {
            Some(i) if i + 1 < self.entries.len() => {
                self.index = Some(i + 1);
                Some(self.entries[i + 1].as_str())
            }
            Some(_) => {
                self.index = None;
                if self.saved_line.is_empty() {
                    None
                } else {
                    self.as_ref_saved()
                }
            }
            None => None,
        }
    }

    fn as_ref_saved(&self) -> Option<&str> {
        if self.saved_line.is_empty() { None } else { Some(self.saved_line.as_str()) }
    }

    pub fn cancel_navigation(&mut self) -> Option<String> {
        let saved = self.saved_line.clone();
        self.index = None;
        self.saved_line.clear();
        if saved.is_empty() { None } else { Some(saved) }
    }

    pub fn is_navigating(&self) -> bool {
        self.index.is_some()
    }

    pub fn start_search(&mut self, current_line: &str) {
        self.saved_line = current_line.to_string();
        self.search_query = current_line.to_string();
        if self.search_query.is_empty() {
            self.search_matches.clear();
            self.search_index = 0;
            return;
        }
        self.search_matches = self.entries.iter()
            .enumerate()
            .filter(|(_, e)| e.contains(&self.search_query))
            .map(|(i, _)| i)
            .collect();
        self.search_index = if self.search_matches.is_empty() {
            0
        } else {
            self.search_matches.len() - 1
        };
    }

    pub fn search_current_match(&self) -> Option<&str> {
        if self.search_matches.is_empty() {
            None
        } else {
            Some(self.entries[self.search_matches[self.search_index]].as_str())
        }
    }

    pub fn search_append(&mut self, c: char) -> Option<&str> {
        self.search_query.push(c);
        self.search_matches = self.entries.iter()
            .enumerate()
            .filter(|(_, e)| e.contains(&self.search_query))
            .map(|(i, _)| i)
            .collect();
        self.search_index = if self.search_matches.is_empty() {
            0
        } else {
            self.search_matches.len() - 1
        };
        if self.search_matches.is_empty() {
            None
        } else {
            self.index = Some(self.search_matches[self.search_index]);
            Some(self.entries[self.search_matches[self.search_index]].as_str())
        }
    }

    pub fn search_backspace(&mut self) -> Option<&str> {
        self.search_query.pop();
        if self.search_query.is_empty() {
            self.search_matches.clear();
            self.search_index = 0;
            self.index = None;
            return self.as_ref_saved();
        }
        self.search_matches = self.entries.iter()
            .enumerate()
            .filter(|(_, e)| e.contains(&self.search_query))
            .map(|(i, _)| i)
            .collect();
        self.search_index = if self.search_matches.is_empty() {
            0
        } else {
            self.search_matches.len() - 1
        };
        if self.search_matches.is_empty() {
            self.index = None;
            None
        } else {
            self.index = Some(self.search_matches[self.search_index]);
            Some(self.entries[self.search_matches[self.search_index]].as_str())
        }
    }

    pub fn search_next(&mut self) -> Option<&str> {
        if self.search_matches.is_empty() || self.search_index == 0 {
            None
        } else {
            self.search_index -= 1;
            self.index = Some(self.search_matches[self.search_index]);
            Some(self.entries[self.search_matches[self.search_index]].as_str())
        }
    }

    pub fn search_prev(&mut self) -> Option<&str> {
        if self.search_matches.is_empty() || self.search_index + 1 >= self.search_matches.len() {
            None
        } else {
            self.search_index += 1;
            self.index = Some(self.search_matches[self.search_index]);
            Some(self.entries[self.search_matches[self.search_index]].as_str())
        }
    }

    pub fn cancel_search(&mut self) -> Option<String> {
        let saved = self.saved_line.clone();
        self.search_query.clear();
        self.search_matches.clear();
        self.search_index = 0;
        self.index = None;
        self.saved_line.clear();
        if saved.is_empty() { None } else { Some(saved) }
    }

    pub fn accept_search(&mut self) -> Option<String> {
        let result = self.index.map(|i| self.entries[i].clone());
        self.search_query.clear();
        self.search_matches.clear();
        self.search_index = 0;
        self.index = None;
        self.saved_line.clear();
        result
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn search_match_count(&self) -> usize {
        self.search_matches.len()
    }

    pub fn search_match_index(&self) -> usize {
        self.search_index
    }

    pub fn search_entries(&self) -> Vec<String> {
        self.search_matches.iter()
            .map(|&i| self.entries[i].clone())
            .collect()
    }

    pub fn is_searching(&self) -> bool {
        !self.search_query.is_empty() || !self.saved_line.is_empty()
    }

    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let mut file = File::create(path)?;
        for entry in &self.entries {
            writeln!(file, "{}", escape_newlines(entry))?;
        }
        Ok(())
    }

    pub fn load_from_file(&mut self, path: &str) -> io::Result<()> {
        if !Path::new(path).exists() {
            File::create(path)?;
            return Ok(());
        }
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        self.entries.clear();
        for line in reader.lines() {
            let line = line?;
            self.entries.push(unescape_newlines(&line));
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|s| s.as_str())
    }

    pub fn get(&self, i: usize) -> Option<&str> {
        self.entries.get(i).map(|s| s.as_str())
    }

    pub fn entries(&self) -> Vec<String> {
        self.entries.clone()
    }
}

fn escape_newlines(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n")
}

fn unescape_newlines(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('\\') => result.push('\\'),
                Some(c) => { result.push('\\'); result.push(c); }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}
