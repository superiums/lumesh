// src/history.rs
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub command: String,
    pub weight: u32,
}

impl HistoryEntry {
    fn new(command: String) -> Self {
        Self { command, weight: 1 }
    }
}

pub struct History {
    entries: Vec<HistoryEntry>,
    max_entries: usize,
    index: Option<usize>,
    saved_line: String,
    search_query: String,
    search_matches: Vec<usize>, // indices into entries, sorted by weight desc
    search_index: usize,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
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
        if let Some(pos) = self.entries.iter().position(|e| e.command == entry) {
            // 权重 +1，移到末尾（保持时间顺序用于 Up/Down 导航）
            let mut e = self.entries.remove(pos);
            e.weight += 1;
            self.entries.push(e);
        } else {
            self.entries.push(HistoryEntry::new(entry));
        }
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        self.index = None;
        self.saved_line.clear();
    }

    // ── 导航（按时间顺序，不受权重影响）──────────────────────────

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
        self.index.map(|i| self.entries[i].command.as_str())
    }

    pub fn next(&mut self, _current_line: &str) -> Option<&str> {
        match self.index {
            Some(i) if i + 1 < self.entries.len() => {
                self.index = Some(i + 1);
                Some(self.entries[i + 1].command.as_str())
            }
            Some(_) => {
                self.index = None;
                self.as_ref_saved()
            }
            None => None,
        }
    }

    fn as_ref_saved(&self) -> Option<&str> {
        if self.saved_line.is_empty() {
            None
        } else {
            Some(self.saved_line.as_str())
        }
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

    // ── Hint（按权重选最优前缀匹配）─────────────────────────────

    pub fn search_hint(&self, current_line: &str) -> Option<String> {
        if current_line.is_empty() {
            return None;
        }
        self.entries
            .iter()
            .filter(|e| e.command.starts_with(current_line))
            .max_by_key(|e| e.weight)
            .map(|e| e.command.trim_start_matches(current_line).to_string())
    }

    // ── Fuzzy 搜索（按权重排序）──────────────────────────────────
    pub fn search_fuzzy_one_cd(&self, query: &str) -> Option<String> {
        self.entries
            .iter()
            .filter(|e| e.command.starts_with("cd ") && fuzzy_match(query, &e.command))
            .max_by_key(|e| e.weight)
            .map(|e| e.command.clone())
    }
    pub fn search_fuzzy_one(&self, query: &str) -> Option<String> {
        self.entries
            .iter()
            .filter(|e| fuzzy_match(query, &e.command))
            .max_by_key(|e| e.weight)
            .map(|e| e.command.clone())
    }
    pub fn search_fuzzy(&self, query: &str) -> Vec<String> {
        let mut matched: Vec<&HistoryEntry> = self
            .entries
            .iter()
            .filter(|e| fuzzy_match(query, &e.command))
            .collect();
        matched.sort_by(|a, b| b.weight.cmp(&a.weight));
        matched.into_iter().map(|e| e.command.clone()).collect()
    }

    // ── 搜索（内部：按权重降序排列 search_matches）───────────────

    fn build_matches(&self, query: &str) -> Vec<usize> {
        let mut matches: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.command.contains(query))
            .map(|(i, _)| i)
            .collect();
        // 权重降序；权重相同则保持时间倒序（index 大的更新）
        matches.sort_by(|&a, &b| {
            self.entries[b]
                .weight
                .cmp(&self.entries[a].weight)
                .then(b.cmp(&a))
        });
        matches
    }

    pub fn start_search(&mut self, current_line: &str) {
        self.saved_line = current_line.to_string();
        self.search_query = current_line.to_string();
        if self.search_query.is_empty() {
            self.search_matches.clear();
            self.search_index = 0;
            return;
        }
        self.search_matches = self.build_matches(&self.search_query);
        self.search_index = 0; // 0 = 权重最高的匹配
    }

    pub fn search_current_match(&self) -> Option<&str> {
        self.search_matches
            .get(self.search_index)
            .map(|&i| self.entries[i].command.as_str())
    }

    pub fn search_append(&mut self, c: char) -> Option<&str> {
        self.search_query.push(c);
        self.search_matches = self.build_matches(&self.search_query);
        self.search_index = 0;
        if let Some(&i) = self.search_matches.first() {
            self.index = Some(i);
            Some(self.entries[i].command.as_str())
        } else {
            self.index = None;
            None
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
        self.search_matches = self.build_matches(&self.search_query);
        self.search_index = 0;
        if let Some(&i) = self.search_matches.first() {
            self.index = Some(i);
            Some(self.entries[i].command.as_str())
        } else {
            self.index = None;
            None
        }
    }

    /// 向下翻（权重次高）
    pub fn search_next(&mut self) -> Option<&str> {
        if self.search_matches.is_empty() || self.search_index + 1 >= self.search_matches.len() {
            return None;
        }
        self.search_index += 1;
        let i = self.search_matches[self.search_index];
        self.index = Some(i);
        Some(self.entries[i].command.as_str())
    }

    /// 向上翻（权重更高）
    pub fn search_prev(&mut self) -> Option<&str> {
        if self.search_matches.is_empty() || self.search_index == 0 {
            return None;
        }
        self.search_index -= 1;
        let i = self.search_matches[self.search_index];
        self.index = Some(i);
        Some(self.entries[i].command.as_str())
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
        let result = self.index.map(|i| self.entries[i].command.clone());
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

    /// 返回当前搜索结果（已按权重排序）
    pub fn search_entries(&self) -> Vec<String> {
        self.search_matches
            .iter()
            .map(|&i| self.entries[i].command.clone())
            .collect()
    }

    pub fn is_searching(&self) -> bool {
        !self.search_query.is_empty() || !self.saved_line.is_empty()
    }

    // ── 文件 I/O（格式：`weight\tcommand`，向后兼容旧格式）──────

    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let mut file = File::create(path)?;
        for entry in &self.entries {
            writeln!(
                file,
                "{}\t{}",
                entry.weight,
                escape_newlines(&entry.command)
            )?;
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
            if line.is_empty() {
                continue;
            }
            // 新格式：`weight\tcommand`；旧格式：直接是命令
            let entry = if let Some((w, cmd)) = line.split_once('\t') {
                let weight = w.parse::<u32>().unwrap_or(1);
                HistoryEntry {
                    command: unescape_newlines(cmd),
                    weight,
                }
            } else {
                HistoryEntry::new(unescape_newlines(&line))
            };
            self.entries.push(entry);
        }
        Ok(())
    }

    // ── 公共访问器 ────────────────────────────────────────────────

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.command.as_str())
    }

    pub fn get(&self, i: usize) -> Option<&str> {
        self.entries.get(i).map(|e| e.command.as_str())
    }

    /// 返回所有命令（时间顺序），供 editor 的 HistorySearch 使用
    pub fn entries(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.command.clone()).collect()
    }

    /// 返回按权重排序的命令列表（供 UI 展示）
    pub fn entries_by_weight(&self) -> Vec<(String, u32)> {
        let mut sorted: Vec<&HistoryEntry> = self.entries.iter().collect();
        sorted.sort_by(|a, b| b.weight.cmp(&a.weight).then(a.command.cmp(&b.command)));
        sorted
            .iter()
            .map(|e| (e.command.clone(), e.weight))
            .collect()
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
                Some(c) => {
                    result.push('\\');
                    result.push(c);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// 简单 fuzzy match：query 的每个字符按序出现在 target 中
fn fuzzy_match(query: &str, target: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let mut chars = target.chars();
    for q in query.chars() {
        if !chars.any(|c| c == q) {
            return false;
        }
    }
    true
}
