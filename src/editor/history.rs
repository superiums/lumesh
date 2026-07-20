// src/history.rs
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub command: String,
    pub weight: u32,
    pub last_path: String,
    pub last_order: u64,
    pub is_multi_dir: bool, // 是否在多个目录下运行过
}

impl HistoryEntry {
    fn new(command: String, path: String, order: u64) -> Self {
        Self {
            command,
            weight: 1,
            last_path: path,
            last_order: order,
            is_multi_dir: false,
        }
    }
}

pub struct History {
    entries: Vec<HistoryEntry>, // 按时间顺序（用于 Up/Down 导航）
    max_entries: usize,
    global_order: u64,
    log_path: Option<String>,
    current_dir: String, // 当前目录缓存，供搜索和 add 使用
    // ── 导航状态 ──
    index: Option<usize>,
    saved_line: String,
    // ── 搜索状态 ──
    search_query: String,
    search_matches: Vec<usize>,
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
            global_order: 0,
            log_path: None,
            current_dir: String::new(),
            index: None,
            saved_line: String::new(),
            search_query: String::new(),
            search_matches: Vec::new(),
            search_index: 0,
        }
    }

    /// 更新当前目录缓存。
    /// 在每次命令执行后（parse_and_eval 之后）调用，确保 add 使用最新路径。
    pub fn set_current_dir(&mut self, path: String) {
        self.current_dir = path;
    }

    pub fn current_dir(&self) -> &str {
        &self.current_dir
    }

    /// 添加一条历史记录，使用 self.current_dir 作为执行路径。
    /// 调用前须先调用 set_current_dir。
    pub fn add(&mut self, entry: String) {
        if entry.trim().is_empty() {
            return;
        }
        self.global_order += 1;
        let order = self.global_order;
        let path = self.current_dir.clone();

        // 1. 追加到日志文件（崩溃安全）
        if self.log_path.is_some() {
            if let Err(e) = self.append_log_entry(&entry, &path, order) {
                eprintln!("Failed to append history log: {e}");
            }
        }

        // 2. 更新内存索引
        if let Some(pos) = self.entries.iter().position(|e| e.command == entry) {
            let mut e = self.entries.remove(pos);
            e.weight += 1;
            if path != e.last_path {
                e.is_multi_dir = true; // 路径变化：标记为多目录命令
            }
            e.last_path = path;
            e.last_order = order;
            self.entries.push(e);
        } else {
            self.entries.push(HistoryEntry::new(entry, path, order));
        }

        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        self.index = None;
        self.saved_line.clear();
    }

    fn append_log_entry(&self, cmd: &str, path: &str, order: u64) -> io::Result<()> {
        let log_path = self.log_path.as_ref().unwrap();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        writeln!(
            file,
            "{}\t{}\t{}\t{}",
            order,
            ts,
            escape_field(path),
            escape_field(cmd)
        )?;
        Ok(())
    }

    // ── 复合评分（用于多结果排序）────────────────────────────────

    /// 本目录专属命令获得 1_000_000 加权，使其排在全局命令之前。
    fn dir_score(&self, entry: &HistoryEntry) -> u64 {
        let local_boost = if !entry.is_multi_dir && entry.last_path == self.current_dir {
            1_000_000u64
        } else {
            0u64
        };
        local_boost + entry.weight as u64
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
            Some(&self.saved_line)
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

    // ── Hint（两阶段：本目录专属优先，再全局）───────────────────

    pub fn search_hint(&self, current_line: &str) -> Option<String> {
        if current_line.is_empty() {
            return None;
        }
        // 阶段1：本目录专属命令（!is_multi_dir && last_path == current_dir）
        let local = self
            .entries
            .iter()
            .filter(|e| {
                !e.is_multi_dir
                    && e.last_path == self.current_dir
                    && e.command.starts_with(current_line)
            })
            .max_by_key(|e| e.weight);

        if let Some(e) = local {
            return Some(e.command.trim_start_matches(current_line).to_string());
        }

        // 阶段2：全局命令（按权重）
        self.entries
            .iter()
            .filter(|e| e.is_multi_dir && e.command.starts_with(current_line))
            .max_by_key(|e| e.weight)
            .map(|e| e.command.trim_start_matches(current_line).to_string())
    }

    // ── Fuzzy 搜索（两阶段：本目录专属优先，再全局）─────────────

    pub fn search_fuzzy_one_cd(&self, query: &str) -> Option<String> {
        // 阶段1：本目录专属 cd 命令
        let local = self
            .entries
            .iter()
            .filter(|e| {
                !e.is_multi_dir
                    && e.last_path == self.current_dir
                    && e.command.starts_with("cd ")
                    && fuzzy_match(query, &e.command)
            })
            .max_by_key(|e| e.weight)
            .map(|e| e.command.clone());

        if local.is_some() {
            return local;
        }

        // 阶段2：全局 cd 命令
        self.entries
            .iter()
            .filter(|e| {
                e.command.starts_with("cd ") && e.is_multi_dir && fuzzy_match(query, &e.command)
            })
            .max_by_key(|e| e.weight)
            .map(|e| e.command.clone())
    }

    pub fn search_fuzzy_one(&self, query: &str) -> Option<String> {
        // 阶段1：本目录专属命令
        let local = self
            .entries
            .iter()
            .filter(|e| {
                !e.is_multi_dir && e.last_path == self.current_dir && fuzzy_match(query, &e.command)
            })
            .max_by_key(|e| e.weight)
            .map(|e| e.command.clone());

        if local.is_some() {
            return local;
        }

        // 阶段2：全局命令
        self.entries
            .iter()
            .filter(|e| e.is_multi_dir && fuzzy_match(query, &e.command))
            .max_by_key(|e| e.weight)
            .map(|e| e.command.clone())
    }

    /// 多结果 本目录专属命令 prefix搜索
    pub fn search_local_startswith(&self, prefix: &str) -> Vec<String> {
        let mut matched: Vec<&HistoryEntry> = self
            .entries
            .iter()
            .filter(|e| {
                !e.is_multi_dir
                    && e.last_path == self.current_dir
                    && (prefix.is_empty() || e.command.starts_with(prefix))
            })
            .collect();
        matched.sort_by_key(|e| e.weight);
        matched.into_iter().map(|e| e.command.clone()).collect()
    }
    /// 多结果 多目录适用命令 prefix搜索
    pub fn search_multidir_startswith(&self, prefix: &str) -> Vec<String> {
        let mut matched: Vec<&HistoryEntry> = self
            .entries
            .iter()
            .filter(|e| e.is_multi_dir && (prefix.is_empty() || e.command.starts_with(prefix)))
            .collect();
        matched.sort_by_key(|e| e.weight);
        matched.into_iter().map(|e| e.command.clone()).collect()
    }
    /// 多结果 本目录适用命令 prefix搜索
    pub fn search_startswith(&self, prefix: &str) -> Vec<String> {
        let mut matched: Vec<&HistoryEntry> = self
            .entries
            .iter()
            .filter(|e| {
                ((!e.is_multi_dir && e.last_path == self.current_dir) || e.is_multi_dir)
                    && (prefix.is_empty() || e.command.starts_with(prefix))
            })
            .collect();
        matched.sort_by_key(|e| e.weight);
        matched.into_iter().map(|e| e.command.clone()).collect()
    }
    /// 多结果 fuzzy 搜索：使用 dir_score 排序（本目录专属命令排前）
    // pub fn search_fuzzy(&self, query: &str) -> Vec<String> {
    //     let mut matched: Vec<&HistoryEntry> = self
    //         .entries
    //         .iter()
    //         .filter(|e| fuzzy_match(query, &e.command))
    //         .collect();
    //     matched.sort_by(|a, b| self.dir_score(b).cmp(&self.dir_score(a)));
    //     matched.into_iter().map(|e| e.command.clone()).collect()
    // }

    // ── Ctrl+R 搜索（按 dir_score 排序）─────────────────────────

    fn build_matches(&self, query: &str) -> Vec<usize> {
        let mut matches: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.command.contains(query))
            .map(|(i, _)| i)
            .collect();
        matches.sort_by(|&a, &b| {
            self.dir_score(&self.entries[b])
                .cmp(&self.dir_score(&self.entries[a]))
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
        self.search_index = 0;
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

    pub fn search_next(&mut self) -> Option<&str> {
        if self.search_matches.is_empty() || self.search_index + 1 >= self.search_matches.len() {
            return None;
        }
        self.search_index += 1;
        let i = self.search_matches[self.search_index];
        self.index = Some(i);
        Some(self.entries[i].command.as_str())
    }

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

    pub fn search_entries(&self) -> Vec<String> {
        self.search_matches
            .iter()
            .map(|&i| self.entries[i].command.clone())
            .collect()
    }

    pub fn is_searching(&self) -> bool {
        !self.search_query.is_empty() || !self.saved_line.is_empty()
    }

    // ── 文件 I/O ──────────────────────────────────────────────────

    /// 保存索引文件（退出时调用）。
    /// 格式：`weight\torder\tpath\tmulti\tcommand`
    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let mut file = File::create(path)?;
        for entry in &self.entries {
            writeln!(
                file,
                "{}\t{}\t{}\t{}\t{}",
                entry.weight,
                entry.last_order,
                escape_field(&entry.last_path),
                if entry.is_multi_dir { 1 } else { 0 },
                escape_field(&entry.command)
            )?;
        }
        Ok(())
    }

    /// 加载历史记录（启动时调用）。
    /// 日志文件路径自动派生为 `{path}.log`。
    /// 调用后建议立即调用 set_current_dir 初始化当前目录。
    pub fn load_from_file(&mut self, path: &str) -> io::Result<()> {
        let log_path = format!("{}.log", path);
        self.log_path = Some(log_path.clone());

        if Path::new(path).exists() {
            self.load_index(path)?;
        } else if Path::new(&log_path).exists() {
            self.rebuild_from_log(&log_path)?;
        } else {
            File::create(&log_path)?;
        }
        Ok(())
    }

    /// 从索引文件加载。
    /// 支持新格式（5字段）、旧4字段格式、旧2字段格式、纯命令格式。
    fn load_index(&mut self, path: &str) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        self.entries.clear();
        self.global_order = 0;

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            // splitn(5) 确保 command 字段中的 \t 不被分割
            let parts: Vec<&str> = line.splitn(5, '\t').collect();
            let entry = match parts.as_slice() {
                // 新格式：weight\torder\tpath\tmulti\tcommand
                [w, o, p, m, cmd] => HistoryEntry {
                    command: unescape_field(cmd),
                    weight: w.parse().unwrap_or(1),
                    last_order: o.parse().unwrap_or(0),
                    last_path: unescape_field(p),
                    is_multi_dir: *m == "1",
                },
                // 旧4字段格式：weight\torder\tpath\tcommand
                [w, o, p, cmd] => HistoryEntry {
                    command: unescape_field(cmd),
                    weight: w.parse().unwrap_or(1),
                    last_order: o.parse().unwrap_or(0),
                    last_path: unescape_field(p),
                    is_multi_dir: false,
                },
                // 旧2字段格式：weight\tcommand
                [w, cmd] => HistoryEntry {
                    command: unescape_field(cmd),
                    weight: w.parse().unwrap_or(1),
                    last_order: 0,
                    last_path: String::new(),
                    is_multi_dir: false,
                },
                // 纯命令格式
                [cmd] => HistoryEntry {
                    command: unescape_field(cmd),
                    weight: 1,
                    last_order: 0,
                    last_path: String::new(),
                    is_multi_dir: false,
                },
                _ => continue,
            };
            if entry.last_order > self.global_order {
                self.global_order = entry.last_order;
            }
            self.entries.push(entry);
        }
        Ok(())
    }

    /// 从日志文件重建索引（索引丢失时的恢复路径）。
    /// 日志格式：`order\ttimestamp\tpath\tcommand`
    fn rebuild_from_log(&mut self, log_path: &str) -> io::Result<()> {
        let file = File::open(log_path)?;
        let reader = BufReader::new(file);
        self.entries.clear();
        self.global_order = 0;

        let mut agg: HashMap<String, (u32, String, u64)> = HashMap::new();
        let mut path_sets: HashMap<String, HashSet<String>> = HashMap::new();
        let mut ordered_cmds: Vec<String> = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(4, '\t').collect();
            if parts.len() < 4 {
                continue;
            }
            let order = parts[0].parse::<u64>().unwrap_or(0);
            let path = unescape_field(parts[2]);
            let cmd = unescape_field(parts[3]);

            if order > self.global_order {
                self.global_order = order;
            }

            // 记录该命令出现过的所有目录（用于计算 is_multi_dir）
            path_sets
                .entry(cmd.clone())
                .or_default()
                .insert(path.clone());

            if let Some(e) = agg.get_mut(&cmd) {
                e.0 += 1;
                e.1 = path;
                e.2 = order;
            } else {
                ordered_cmds.push(cmd.clone());
                agg.insert(cmd, (1, path, order));
            }
        }

        for cmd in ordered_cmds {
            if let Some((weight, last_path, last_order)) = agg.remove(&cmd) {
                let is_multi_dir = path_sets.get(&cmd).map_or(false, |s| s.len() > 1);
                self.entries.push(HistoryEntry {
                    command: cmd,
                    weight,
                    last_path,
                    last_order,
                    is_multi_dir,
                });
            }
        }

        if self.entries.len() > self.max_entries {
            let drain = self.entries.len() - self.max_entries;
            self.entries.drain(0..drain);
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

    pub fn entries(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.command.clone()).collect()
    }

    pub fn cmdstr_by_weight(&self) -> Vec<String> {
        let mut sorted: Vec<&HistoryEntry> = self.entries.iter().collect();
        sorted.sort_by(|a, b| {
            self.dir_score(b)
                .cmp(&self.dir_score(a))
                .then(a.command.cmp(&b.command))
        });
        sorted.iter().map(|e| e.command.clone()).collect()
    }

    pub fn entries_by_weight(&self) -> Vec<&HistoryEntry> {
        let mut sorted: Vec<&HistoryEntry> = self.entries.iter().collect();
        sorted.sort_by(|a, b| {
            self.dir_score(b)
                .cmp(&self.dir_score(a))
                .then(a.command.cmp(&b.command))
        });
        sorted
    }

    pub fn global_order(&self) -> u64 {
        self.global_order
    }
}

// ── 字段转义（制表符分隔格式）────────────────────────────────────

fn escape_field(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn unescape_field(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
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
