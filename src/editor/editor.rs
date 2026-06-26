use std::collections::HashMap;
use std::io::{self, Write, stdout};

use crossterm::cursor::MoveTo;
use crossterm::event::{Event, KeyEventKind, read};
use crossterm::queue;
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode, size};
use unicode_width::UnicodeWidthChar;

use super::buffer::LineBuffer;
use super::history::History;
use super::key::{Cmd, KeyEvent};
use super::kring::KillRing;

type HotkeyFn = Box<dyn Fn(&str) -> Option<String>>;

const MAX_POPUP_HEIGHT: usize = 10;

#[derive(Debug, Clone)]
pub struct EditorTheme {
    pub hint_color: Color,
    pub completion_bg: Color,
    pub completion_fg: Color,
    pub completion_selected_bg: Color,
    pub completion_selected_fg: Color,
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self {
            hint_color: Color::DarkGrey,
            completion_bg: Color::DarkGrey,
            completion_fg: Color::White,
            completion_selected_bg: Color::White,
            completion_selected_fg: Color::Black,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub display: Option<String>,
    pub replacement: String,
    pub suffix: Option<char>,
}

impl CompletionItem {
    pub fn new(replacement: String) -> Self {
        Self {
            display: None,
            replacement,
            suffix: None,
        }
    }

    pub fn with_display(display: String, replacement: String) -> Self {
        Self {
            display: Some(display),
            replacement,
            suffix: None,
        }
    }

    pub fn with(display: String, replacement: String, suffix: char) -> Self {
        Self {
            display: Some(display),
            replacement,
            suffix: Some(suffix),
        }
    }

    pub fn display_text(&self) -> &str {
        self.display.as_deref().unwrap_or(&self.replacement)
    }
}

pub trait Completer {
    fn complete(&self, line: &str, pos: usize) -> Vec<CompletionItem>;
}

pub trait Highlighter {
    fn highlight(&self, line: &str) -> String;
    fn highlight_char(&self, line: &str, pos: usize) -> bool;
}

pub trait Hinter {
    fn hint(&self, line: &str, pos: usize) -> Option<String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Valid,
    Incomplete,
    Invalid(String),
}

pub trait Validator {
    fn validate(&self, input: &str) -> ValidationResult;
}

#[derive(Clone)]
enum EditorMode {
    Normal,
    Multiline,
    CompletionSelect {
        completions: Vec<CompletionItem>,
        selected: usize,
        start_pos: usize,
    },
}

pub struct Editor {
    buffer: LineBuffer,
    history: History,
    kill_ring: KillRing,
    completer: Option<Box<dyn Completer>>,
    highlighter: Option<Box<dyn Highlighter>>,
    hinter: Option<Box<dyn Hinter>>,
    custom_bindings: HashMap<KeyEvent, Cmd>,
    hotkey_fns: HashMap<KeyEvent, HotkeyFn>,
    abbreviations: HashMap<String, String>,
    sudo_cmd: String,
    is_history_completion: bool,
    init_search_pos: usize,
    mode: EditorMode,
    prompt_row: u16,
    terminal_width: u16,
    terminal_height: u16,
    prompt: String,
    prompt_width: usize,
    current_hint: Option<String>,
    popup_rendered: Option<(u16, u16)>,
    theme: EditorTheme,
    cont_prompt: String,
    cont_prompt_width: usize,
    show_hint: bool,
    validator: Option<Box<dyn Validator>>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        let (w, h) = size().unwrap_or((80, 24));
        Self {
            buffer: LineBuffer::new(),
            history: History::new(),
            kill_ring: KillRing::new(),
            completer: None,
            highlighter: None,
            hinter: None,
            custom_bindings: HashMap::new(),
            hotkey_fns: HashMap::new(),
            abbreviations: HashMap::new(),
            sudo_cmd: "sudo".to_string(),
            is_history_completion: false,
            init_search_pos: 0,
            mode: EditorMode::Normal,
            prompt_row: 0,
            terminal_width: w,
            terminal_height: h,
            prompt: String::new(),
            prompt_width: 0,
            current_hint: None,
            popup_rendered: None,
            theme: EditorTheme::default(),
            cont_prompt: "... ".to_string(),
            cont_prompt_width: visible_width("... "),
            show_hint: false,
            validator: None,
        }
    }

    pub fn set_theme(&mut self, theme: EditorTheme) {
        self.theme = theme;
    }

    pub fn set_cont_prompt(&mut self, prompt: &str) {
        self.cont_prompt = prompt.to_string();
        self.cont_prompt_width = visible_width(&self.cont_prompt);
    }

    pub fn cont_prompt(&self) -> &str {
        &self.cont_prompt
    }

    pub fn set_validator(&mut self, validator: Box<dyn Validator>) {
        self.validator = Some(validator);
    }

    fn should_accept(&self) -> bool {
        let text = self.buffer.text();

        if text.ends_with("\n\n") {
            return true;
        }

        let trimmed = text.trim_end();
        if trimmed.ends_with('\\') && !trimmed.ends_with("\\\\") {
            return false;
        }

        match self.validator {
            Some(ref validator) => validator.validate(&text) == ValidationResult::Valid,
            None => true,
        }
    }

    pub fn set_completer(&mut self, completer: Box<dyn Completer>) {
        self.completer = Some(completer);
    }

    pub fn set_highlighter(&mut self, highlighter: Box<dyn Highlighter>) {
        self.highlighter = Some(highlighter);
    }

    pub fn set_hinter(&mut self, hinter: Box<dyn Hinter>) {
        self.hinter = Some(hinter);
    }

    pub fn set_abbreviations(&mut self, abbrs: HashMap<String, String>) {
        self.abbreviations = abbrs;
    }

    pub fn set_sudo_cmd(&mut self, cmd: &str) {
        self.sudo_cmd = cmd.to_string();
    }

    pub fn bind_sequence(&mut self, key: KeyEvent, cmd: Cmd) {
        self.custom_bindings.insert(key, cmd);
    }

    pub fn bind_hotkey_fn<F>(&mut self, key: KeyEvent, f: F)
    where
        F: Fn(&str) -> Option<String> + 'static,
    {
        self.hotkey_fns.insert(key, Box::new(f));
    }

    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn readline(&mut self, prompt: &str) -> Result<String, ReadlineError> {
        enable_raw_mode().map_err(ReadlineError::Io)?;
        let _ = write!(stdout(), "\x1b[?2004h");
        let _ = stdout().flush();

        let (w, h) = size().unwrap_or((80, 24));
        self.terminal_width = w;
        self.terminal_height = h;

        self.prompt_row = 0;
        if let Ok((_col, row)) = crossterm::cursor::position() {
            self.prompt_row = row;
        }

        self.prompt = prompt.to_string();
        self.prompt_width = visible_width(&self.prompt);
        self.buffer = LineBuffer::new();
        self.mode = EditorMode::Normal;
        self.is_history_completion = false;
        self.init_search_pos = 0;
        self.current_hint = None;
        self.show_hint = false;

        let result = self.event_loop();

        if result.is_ok() {
            let _ = write!(stdout(), "\r\n");
        }

        let _ = write!(stdout(), "\x1b[?2004l");
        let _ = stdout().flush();
        let _ = disable_raw_mode();
        result
    }

    fn event_loop(&mut self) -> Result<String, ReadlineError> {
        loop {
            self.render()?;

            let event = self.read_event()?;

            // Completion mode: handle events
            if matches!(self.mode, EditorMode::CompletionSelect { .. })
                && self.try_completion_event(&event)
            {
                continue;
            }

            // Check hotkey function bindings first (for function/lambda values)
            if let Some(f) = self.hotkey_fns.get(&event) {
                self.buffer.insert_str("\n");
                let result = f(&self.buffer.text());
                if let Some(text) = result {
                    self.buffer.insert_str(&text);
                    self.leave_completion();
                } else {
                    return self.accept_line();
                }
                continue;
            }

            // Check custom bindings
            let custom_cmd = self.custom_bindings.get(&event).cloned();
            if let Some(cmd) = custom_cmd {
                match cmd {
                    Cmd::InsertStr(ref s) => {
                        self.buffer.insert_str(s);
                        self.leave_completion();
                    }
                    Cmd::AcceptLine => {
                        return self.accept_line();
                    }
                    Cmd::Noop => {}
                    ref c => {
                        self.leave_completion();
                        self.handle_cmd(c.clone());
                    }
                }
                continue;
            }

            self.show_hint = false;

            match event {
                KeyEvent::Paste(text) => {
                    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
                    self.buffer.insert_str(&normalized);
                    if normalized.contains('\n') {
                        self.mode = EditorMode::Multiline;
                    }
                    self.leave_completion();
                }
                KeyEvent::Char(' ') => {
                    self.show_hint = true;
                    self.handle_space();
                }
                KeyEvent::Char(c) | KeyEvent::Shift(c) => {
                    self.show_hint = true;
                    self.leave_completion();
                    self.buffer.insert(c);
                }
                KeyEvent::Ctrl('c') => {
                    return Err(ReadlineError::Interrupted);
                }
                KeyEvent::Ctrl('d') => {
                    if self.buffer.is_empty() {
                        return Err(ReadlineError::Eof);
                    }
                    self.buffer.delete();
                    self.leave_completion();
                }
                KeyEvent::Ctrl('s') => {
                    // May want to add Ctrl+S binding later
                }
                KeyEvent::Ctrl('l') => {
                    let _ = terminal::Clear(ClearType::All);
                    self.prompt_row = 0;
                }
                KeyEvent::Enter => {
                    self.current_hint = None;
                    self.show_hint = false;
                    if matches!(self.mode, EditorMode::Multiline) {
                        if self.buffer.cursor_on_empty_line() {
                            return self.accept_line();
                        }
                        let indent = self.buffer.current_line_indent();
                        self.buffer.insert('\n');
                        if !indent.is_empty() {
                            self.buffer.insert_str(&indent);
                        }
                    } else if self.should_accept() {
                        return self.accept_line();
                    } else {
                        let indent = self.buffer.current_line_indent();
                        self.buffer.insert('\n');
                        if !indent.is_empty() {
                            self.buffer.insert_str(&indent);
                        }
                        self.mode = EditorMode::Multiline;
                    }
                }
                KeyEvent::Tab => {
                    self.handle_tab();
                }
                KeyEvent::BackTab => {
                    self.handle_backtab();
                }
                KeyEvent::Backspace => {
                    self.leave_completion();
                    self.buffer.backspace();
                }
                KeyEvent::Delete => {
                    self.leave_completion();
                    self.buffer.delete();
                }
                KeyEvent::Left => {
                    self.leave_completion();
                    self.buffer.move_left();
                }
                KeyEvent::Right => {
                    self.leave_completion();
                    self.buffer.move_right();
                }
                KeyEvent::Up => {
                    self.leave_completion();
                    if matches!(self.mode, EditorMode::Multiline) {
                        self.move_cursor_up();
                    } else if let Some(entry) = self.history.previous(&self.buffer.text()) {
                        self.buffer.set_text(entry);
                        self.buffer.move_to_end();
                        self.set_normal_mode();
                    }
                }
                KeyEvent::Down => {
                    self.leave_completion();
                    if matches!(self.mode, EditorMode::Multiline) {
                        self.move_cursor_down();
                    } else if let Some(entry) = self.history.next(&self.buffer.text()) {
                        self.buffer.set_text(entry);
                        self.buffer.move_to_end();
                        self.set_normal_mode();
                    }
                }
                KeyEvent::Home => {
                    self.leave_completion();
                    if matches!(self.mode, EditorMode::Multiline) {
                        self.buffer.move_to_line_start();
                    } else {
                        self.buffer.move_to_start();
                    }
                }
                KeyEvent::End => {
                    self.leave_completion();
                    if matches!(self.mode, EditorMode::Multiline) {
                        self.buffer.move_to_line_end();
                    } else {
                        self.buffer.move_to_end();
                    }
                }
                KeyEvent::Escape => {
                    if matches!(self.mode, EditorMode::CompletionSelect { .. }) {
                        self.set_normal_mode();
                    } else if matches!(self.mode, EditorMode::Multiline) {
                        self.mode = EditorMode::Normal;
                    }
                }
                _ => {
                    self.handle_other_events(event);
                }
            }
        }
    }

    fn accept_line(&mut self) -> Result<String, ReadlineError> {
        let text = self.buffer.text();
        Ok(text)
    }

    // ---- Completion event handling ----

    fn try_completion_event(&mut self, event: &KeyEvent) -> bool {
        let (completions, selected, start_pos) = match &self.mode {
            EditorMode::CompletionSelect {
                completions,
                selected,
                start_pos,
            } => (completions.clone(), *selected, *start_pos),
            _ => return false,
        };

        match event {
            KeyEvent::Up | KeyEvent::Ctrl('p') => {
                let new_sel = if selected == 0 {
                    completions.len() - 1
                } else {
                    selected - 1
                };
                self.mode = EditorMode::CompletionSelect {
                    completions,
                    selected: new_sel,
                    start_pos,
                };
                true
            }
            KeyEvent::Down | KeyEvent::Ctrl('n') | KeyEvent::Tab => {
                let new_sel = if selected + 1 >= completions.len() {
                    0
                } else {
                    selected + 1
                };
                self.mode = EditorMode::CompletionSelect {
                    completions,
                    selected: new_sel,
                    start_pos,
                };
                true
            }
            KeyEvent::Enter | KeyEvent::Char(' ') | KeyEvent::Shift(' ') => {
                if let Some(item) = completions.get(selected) {
                    self.apply_completion(item, start_pos);
                }
                self.set_normal_mode();
                true
            }
            KeyEvent::Escape => {
                self.set_normal_mode();
                true
            }
            KeyEvent::BackTab => {
                let new_sel = if selected == 0 {
                    completions.len() - 1
                } else {
                    selected - 1
                };
                self.mode = EditorMode::CompletionSelect {
                    completions,
                    selected: new_sel,
                    start_pos,
                };
                true
            }
            KeyEvent::Ctrl('r') | KeyEvent::Ctrl('s') if self.is_history_completion => {
                let new_sel = match event {
                    KeyEvent::Ctrl('r') => {
                        if selected == 0 {
                            completions.len() - 1
                        } else {
                            selected - 1
                        }
                    }
                    _ => {
                        if selected + 1 >= completions.len() {
                            0
                        } else {
                            selected + 1
                        }
                    }
                };
                self.mode = EditorMode::CompletionSelect {
                    completions,
                    selected: new_sel,
                    start_pos,
                };
                true
            }
            KeyEvent::Char(c) | KeyEvent::Shift(c) => {
                self.buffer.insert(*c);
                self.refresh_completions();
                true
            }
            KeyEvent::Backspace => {
                self.buffer.backspace();
                self.refresh_completions();
                true
            }
            _ => false,
        }
    }

    fn refresh_completions(&mut self) {
        let line = self.buffer.text();
        let pos = self.buffer.cursor();
        let byte_pos = line
            .char_indices()
            .nth(pos)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        let (old_completions, old_sel) = match &self.mode {
            EditorMode::CompletionSelect {
                completions,
                selected,
                ..
            } => (completions.clone(), *selected),
            _ => return,
        };

        let new_completions: Vec<CompletionItem> = if self.is_history_completion {
            let start_pos = Self::find_word_start(&line, byte_pos);
            let query = &line[start_pos..byte_pos];
            old_completions
                .into_iter()
                .filter(|item| fuzzy_match(query, &item.replacement))
                .collect()
        } else {
            // Parameter completion: filter existing completions with fuzzy match
            let query_start = self.init_search_pos.min(byte_pos);
            let query = &line[query_start..byte_pos];
            old_completions
                .into_iter()
                .filter(|item| fuzzy_match(query, &item.replacement))
                .collect()
        };

        let start_pos = if line[..byte_pos].contains(['/', '\\']) {
            Self::find_path_start(&line, byte_pos)
        } else {
            Self::find_word_start(&line, byte_pos)
        };
        if new_completions.is_empty() {
            self.set_normal_mode();
        } else {
            let count = new_completions.len();
            let current_sel = old_sel.min(count.saturating_sub(1));
            self.mode = EditorMode::CompletionSelect {
                completions: new_completions,
                selected: current_sel,
                start_pos,
            };
        }
    }

    fn apply_completion(&mut self, item: &CompletionItem, start_pos: usize) {
        let cursor = self.buffer.cursor();
        let end_pos = cursor.max(start_pos);
        match item.suffix {
            Some(suf) => self.buffer.replace_range(
                start_pos,
                end_pos,
                &format!("{}{}", item.replacement, suf),
            ),
            None => self
                .buffer
                .replace_range(start_pos, end_pos, &item.replacement),
        }
    }

    // ---- Abbreviation handling ----

    fn handle_space(&mut self) {
        let text = self.buffer.text();
        if !text.contains(' ')
            && let Some(expanded) = self.abbreviations.get(text.trim())
        {
            self.buffer.set_text(expanded);
            self.buffer.insert(' ');
            self.buffer.move_to_end();
            return;
        }
        self.buffer.insert(' ');
    }

    // ---- Other methods ----

    fn handle_other_events(&mut self, event: KeyEvent) {
        let cmd = self.default_binding(&event);
        self.handle_cmd(cmd);
    }

    fn handle_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Insert(c) => {
                self.leave_completion();
                self.buffer.insert(c);
            }
            Cmd::InsertStr(s) => {
                self.leave_completion();
                self.buffer.insert_str(&s);
            }
            Cmd::InsertStrAtBeginning(s) => {
                self.leave_completion();
                let text = self.buffer.text();
                let new_text = format!("{}{}", s, text);
                self.buffer.set_text(&new_text);
                self.buffer.move_to_end();
            }
            Cmd::Backspace => {
                self.leave_completion();
                self.buffer.backspace();
            }
            Cmd::Delete => {
                self.leave_completion();
                self.buffer.delete();
            }
            Cmd::MoveLeft => {
                self.leave_completion();
                self.buffer.move_left();
            }
            Cmd::MoveRight => {
                self.leave_completion();
                self.buffer.move_right();
            }
            Cmd::MoveUp => self.leave_completion(),
            Cmd::MoveDown => self.leave_completion(),
            Cmd::MoveToStart => {
                self.leave_completion();
                self.buffer.move_to_start();
            }
            Cmd::MoveToEnd => {
                self.leave_completion();
                self.buffer.move_to_end();
            }
            Cmd::MoveWordLeft => {
                self.leave_completion();
                self.buffer.move_word_left();
            }
            Cmd::MoveWordRight => {
                self.leave_completion();
                self.buffer.move_word_right();
            }
            Cmd::DeleteWordBefore => {
                if let Some(killed) = self.buffer.delete_word_before() {
                    self.kill_ring.push(killed);
                }
                self.leave_completion();
            }
            Cmd::DeleteWordAfter => {
                if let Some(killed) = self.buffer.delete_word_after() {
                    self.kill_ring.push(killed);
                }
                self.leave_completion();
            }
            Cmd::DeleteToStart => {
                if let Some(killed) = self.buffer.delete_to_start() {
                    self.kill_ring.push(killed);
                }
                self.leave_completion();
            }
            Cmd::DeleteToEnd => {
                if let Some(killed) = self.buffer.delete_to_end() {
                    self.kill_ring.push(killed);
                }
                self.leave_completion();
            }
            Cmd::DeleteToLineStart => {
                if let Some(killed) = self.buffer.delete_to_line_start() {
                    self.kill_ring.push(killed);
                }
                self.leave_completion();
            }
            Cmd::DeleteToLineEnd => {
                if let Some(killed) = self.buffer.delete_to_line_end() {
                    self.kill_ring.push(killed);
                }
                self.leave_completion();
            }
            Cmd::AcceptLine => {}
            Cmd::Cancel => {
                self.leave_completion();
                if self.history.is_navigating()
                    && let Some(restored) = self.history.cancel_navigation()
                {
                    self.buffer.set_text(&restored);
                }
            }
            Cmd::Complete => {
                self.handle_tab();
            }
            Cmd::HistoryPrevious => {
                if let Some(line) = self.history.previous(&self.buffer.text()) {
                    self.buffer.set_text(line);
                    self.buffer.move_to_end();
                }
            }
            Cmd::HistoryNext => {
                if let Some(line) = self.history.next(&self.buffer.text()) {
                    self.buffer.set_text(line);
                    self.buffer.move_to_end();
                }
            }
            Cmd::HistorySearch => {
                let line = self.buffer.text();
                let pos = self.buffer.cursor();
                let byte_pos = line
                    .char_indices()
                    .nth(pos)
                    .map(|(i, _)| i)
                    .unwrap_or(line.len());
                let start_pos = if line[..byte_pos].contains(['/', '\\']) {
                    Self::find_path_start(&line, byte_pos)
                } else {
                    Self::find_word_start(&line, byte_pos)
                };
                let query = &line[start_pos..byte_pos];
                let all: Vec<String> = self.history.entries();
                let completions: Vec<CompletionItem> = all
                    .iter()
                    .filter(|e| {
                        e.to_ascii_lowercase()
                            .starts_with(&query.to_ascii_lowercase())
                    })
                    .map(|s| CompletionItem::with_display(s.clone(), s.clone()))
                    .collect();
                if !completions.is_empty() {
                    self.mode = EditorMode::CompletionSelect {
                        completions,
                        selected: 0,
                        start_pos,
                    };
                    self.is_history_completion = true;
                }
            }
            Cmd::Yank => {
                if let Some(text) = self.kill_ring.yank() {
                    self.buffer.insert_str(text);
                }
                self.leave_completion();
            }
            Cmd::TransposeChars => {
                self.leave_completion();
                self.transpose_chars();
            }
            Cmd::ClearScreen => {
                let _ = terminal::Clear(ClearType::All);
                self.prompt_row = 0;
            }
            Cmd::AcceptHint => {
                if let Some(ref hint) = self.current_hint.clone() {
                    let clean = strip_ansi(hint);
                    self.buffer.insert_str(&clean);
                }
                self.leave_completion();
            }
            Cmd::AcceptHintWord(mode) => {
                if let Some(ref hint) = self.current_hint.clone() {
                    let clean = strip_ansi(hint);
                    let word = match mode {
                        1 => {
                            let pos = clean.find(['<', '[']);
                            let end = pos.unwrap_or(clean.len());
                            clean[..end].to_string()
                        }
                        _ => {
                            let trimmed = clean.trim_start();
                            let pos = trimmed.find(|c: char| c.is_ascii_whitespace() || c == '/');
                            let end = pos.map_or(trimmed.len(), |x| {
                                x + if clean.starts_with(' ') { 2 } else { 1 }
                            });
                            clean[..end].to_string()
                        }
                    };
                    self.buffer.insert_str(&word);
                }
                self.leave_completion();
            }
            Cmd::ClearBuffer => {
                self.buffer.set_text("");
                self.leave_completion();
            }
            Cmd::ToggleSudo => {
                let text = self.buffer.text();
                let prefix = format!("{} ", self.sudo_cmd);
                if text.starts_with(&prefix) {
                    let new_text = text[prefix.len()..].to_string();
                    self.buffer.set_text(&new_text);
                } else {
                    let new_text = format!("{}{}", prefix, text);
                    self.buffer.set_text(&new_text);
                }
                self.buffer.move_to_end();
                self.leave_completion();
            }
            Cmd::Noop => {}
        }
        if !matches!(self.mode, EditorMode::CompletionSelect { .. }) {
            self.set_normal_mode();
        }
    }

    fn handle_tab(&mut self) {
        if self.buffer.cursor_at_indent() {
            self.buffer.insert_str("    ");
            return;
        }
        let line = self.buffer.text();
        let pos = self.buffer.cursor();
        let byte_pos = line
            .char_indices()
            .nth(pos)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        self.init_search_pos = byte_pos;
        if let Some(ref completer) = self.completer {
            let completions = completer.complete(&line, byte_pos);
            if completions.is_empty() {
                return;
            }
            let start_pos = if line[..byte_pos].contains(['/', '\\']) {
                Self::find_path_start(&line, byte_pos)
            } else {
                Self::find_word_start(&line, byte_pos)
            };
            if completions.len() == 1 {
                self.apply_completion(&completions[0], start_pos);
                return;
            }
            self.mode = EditorMode::CompletionSelect {
                completions,
                selected: 0,
                start_pos,
            };
            self.is_history_completion = false;
        }
    }

    fn handle_backtab(&mut self) {
        match &self.mode {
            EditorMode::CompletionSelect {
                completions,
                selected,
                start_pos,
            } => {
                let new_sel = if *selected == 0 {
                    completions.len() - 1
                } else {
                    selected - 1
                };
                self.mode = EditorMode::CompletionSelect {
                    completions: completions.clone(),
                    selected: new_sel,
                    start_pos: *start_pos,
                };
            }
            EditorMode::Multiline => {
                // delete a indent
            }
            _ => {}
        }
    }

    fn default_binding(&self, event: &KeyEvent) -> Cmd {
        match event {
            KeyEvent::Ctrl(c) => match c {
                'a' => Cmd::MoveToStart,
                'b' => Cmd::MoveLeft,
                'c' => Cmd::Noop,
                'd' => Cmd::Noop,
                'e' => Cmd::MoveToEnd,
                'f' => Cmd::MoveRight,
                'h' => Cmd::Backspace,
                'k' => Cmd::DeleteToLineEnd,
                'l' => Cmd::ClearScreen,
                'n' => Cmd::HistoryNext,
                'p' => Cmd::HistoryPrevious,
                'r' => Cmd::HistorySearch,
                't' => Cmd::TransposeChars,
                'u' => Cmd::DeleteToLineStart,
                'w' => Cmd::DeleteWordBefore,
                'y' => Cmd::Yank,
                _ => Cmd::Noop,
            },
            KeyEvent::Alt(c) => match c {
                'b' => Cmd::MoveWordLeft,
                'f' => Cmd::MoveWordRight,
                'd' => Cmd::DeleteWordAfter,
                _ => Cmd::Noop,
            },
            KeyEvent::Shift(c) => Cmd::Insert(*c),
            _ => Cmd::Noop,
        }
    }

    fn transpose_chars(&mut self) {
        self.buffer.transpose_chars();
    }

    fn set_normal_mode(&mut self) {
        self.mode = if self.buffer.text().contains('\n') {
            EditorMode::Multiline
        } else {
            EditorMode::Normal
        };
    }

    fn leave_completion(&mut self) {
        if matches!(self.mode, EditorMode::CompletionSelect { .. }) {
            self.set_normal_mode();
        }
        self.is_history_completion = false;
    }

    fn move_cursor_up(&mut self) {
        self.buffer.move_cursor_up_line();
    }

    fn move_cursor_down(&mut self) {
        self.buffer.move_cursor_down_line();
    }

    fn find_word_start(line: &str, pos: usize) -> usize {
        let pos = pos.min(line.len());
        let before = &line[..pos];
        before
            .rfind(|c: char| c.is_ascii_whitespace() || c == '.' || c == '>' || c == ':')
            .map(|i| i + 1)
            .unwrap_or(0)
    }

    fn find_path_start(line: &str, pos: usize) -> usize {
        let pos = pos.min(line.len());
        let before = &line[..pos];
        before
            .rfind(|c: char| {
                c.is_ascii_whitespace()
                    || c == '>'
                    || c == ':'
                    || c == '|'
                    || c == '&'
                    || c == '('
                    || c == ';'
            })
            .map(|i| i + 1)
            .unwrap_or(0)
    }

    // ---- Rendering ----
    fn render(&mut self) -> Result<(), ReadlineError> {
        // Auto-scroll: if in completion mode and not enough room below, clear screen
        if matches!(self.mode, EditorMode::CompletionSelect { .. }) {
            let est_space =
                (self.terminal_height as usize).saturating_sub(self.prompt_row as usize + 2);
            if est_space < 3 {
                let _ = terminal::Clear(ClearType::All);
                let _ = crossterm::queue!(std::io::stdout(), MoveTo(0, 0));
                self.prompt_row = 0;
                self.popup_rendered = None;
            }
        }

        let mut stdout = stdout();
        let line = self.buffer.text();
        let cursor = self.buffer.cursor();

        // Clear previously rendered popup area (which may be above prompt_row)
        if let Some((start, end)) = self.popup_rendered {
            for row in start..=end {
                let _ = queue!(stdout, MoveTo(0, row), Clear(ClearType::CurrentLine));
            }
            self.popup_rendered = None;
        }

        // full clear when input has newlines or wraps beyond visible area (to avoid scrolling ghosts)
        if line.contains('\n') {
            queue!(stdout, MoveTo(0, 0), Clear(ClearType::All)).map_err(ReadlineError::Io)?;
            self.prompt_row = 0;
        } else {
            let est_visual_rows =
                1 + (self.prompt_width + line.len()) / self.terminal_width as usize;
            let available_rows =
                (self.terminal_height as usize).saturating_sub(self.prompt_row as usize);
            if est_visual_rows > available_rows {
                queue!(stdout, MoveTo(0, 0), Clear(ClearType::All)).map_err(ReadlineError::Io)?;
                self.prompt_row = 0;
            } else {
                queue!(
                    stdout,
                    MoveTo(0, self.prompt_row),
                    Clear(ClearType::FromCursorDown)
                )
                .map_err(ReadlineError::Io)?;
            }
        }

        if line.contains('\n') {
            let parts: Vec<&str> = line.split('\n').collect();
            for (i, part) in parts.iter().enumerate() {
                let prefix = if i == 0 {
                    &self.prompt
                } else {
                    &self.cont_prompt
                };
                queue!(stdout, Print(prefix)).map_err(ReadlineError::Io)?;
                if let Some(ref hl) = self.highlighter {
                    queue!(stdout, Print(&hl.highlight(part))).map_err(ReadlineError::Io)?;
                } else {
                    queue!(stdout, Print(part)).map_err(ReadlineError::Io)?;
                }
                if i + 1 < parts.len() {
                    queue!(stdout, Print("\r\n")).map_err(ReadlineError::Io)?;
                }
            }
        } else {
            queue!(stdout, Print(&self.prompt)).map_err(ReadlineError::Io)?;
            if let Some(ref hl) = self.highlighter {
                queue!(stdout, Print(&hl.highlight(&line))).map_err(ReadlineError::Io)?;
            } else {
                queue!(stdout, Print(&line)).map_err(ReadlineError::Io)?;
            }
        }

        // Compute cursor position
        let byte_cursor = line
            .char_indices()
            .nth(cursor)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        let lines_before_cursor: Vec<&str> = line[..byte_cursor].split('\n').collect();
        let cursor_row_offset = lines_before_cursor.len() - 1;
        let col_in_last = visible_width(lines_before_cursor.last().copied().unwrap_or(""));
        let total_col = if cursor_row_offset == 0 {
            self.prompt_width + col_in_last
        } else {
            self.cont_prompt_width + col_in_last
        };
        let vis_width = self.terminal_width as usize;
        let used_rows = cursor_row_offset + 1 + total_col / vis_width;
        let cursor_row =
            self.prompt_row + cursor_row_offset as u16 + (total_col / vis_width) as u16;
        let cursor_col = (total_col % vis_width) as u16;

        // Cache and render hint at cursor position
        self.current_hint = None;
        if self.show_hint
            && let Some(ref hinter) = self.hinter
            && let Some(hint) = hinter.hint(&line, byte_cursor)
        {
            self.current_hint = Some(hint.clone());
            queue!(
                stdout,
                MoveTo(cursor_col, cursor_row),
                SetForegroundColor(self.theme.hint_color),
                Print(&hint),
                ResetColor
            )
            .map_err(ReadlineError::Io)?;
        }

        // Render completion popup
        let popup_data = match &self.mode {
            EditorMode::CompletionSelect {
                completions,
                selected,
                ..
            } => Some((completions.clone(), *selected)),
            _ => None,
        };
        if let Some((completions, selected)) = popup_data {
            self.render_completion_popup(&mut stdout, &completions, selected, used_rows)?;
        }

        // Position cursor
        queue!(stdout, MoveTo(cursor_col, cursor_row)).map_err(ReadlineError::Io)?;
        stdout.flush().map_err(ReadlineError::Io)?;
        Ok(())
    }

    fn render_completion_popup(
        &mut self,
        _stdout: &mut io::Stdout,
        completions: &[CompletionItem],
        selected: usize,
        used_rows: usize,
    ) -> Result<(), ReadlineError> {
        let total = completions.len();
        let max_height = MAX_POPUP_HEIGHT.min(total);
        let needs_more = total > max_height;

        let space_below =
            (self.terminal_height as usize).saturating_sub(self.prompt_row as usize + used_rows);
        let start_row = self.prompt_row + used_rows as u16;
        let height = if needs_more {
            max_height.min(space_below.saturating_sub(1))
        } else {
            max_height.min(space_below)
        };
        if height == 0 {
            return Ok(());
        }

        let scroll_start = if selected >= height {
            selected - height + 1
        } else {
            0
        };
        let scroll_end = (scroll_start + height).min(total);
        let popup_width = self.terminal_width as usize;

        for (idx, item) in completions
            .iter()
            .enumerate()
            .skip(scroll_start)
            .take(scroll_end - scroll_start)
        {
            let display_idx = idx - scroll_start;
            let row = start_row + display_idx as u16;
            let is_selected = idx == selected;

            queue!(_stdout, MoveTo(0, row)).map_err(ReadlineError::Io)?;

            if is_selected {
                queue!(
                    _stdout,
                    SetBackgroundColor(self.theme.completion_selected_bg),
                    SetForegroundColor(self.theme.completion_selected_fg)
                )
                .map_err(ReadlineError::Io)?;
            } else {
                queue!(
                    _stdout,
                    SetBackgroundColor(self.theme.completion_bg),
                    SetForegroundColor(self.theme.completion_fg)
                )
                .map_err(ReadlineError::Io)?;
            }

            let display_text = item.display_text();
            let display = if display_text.len() > popup_width {
                let truncated: String = display_text
                    .chars()
                    .take(popup_width.saturating_sub(1))
                    .collect();
                format!("{}…", truncated)
            } else {
                format!("{:width$}", display_text, width = popup_width)
            };

            queue!(_stdout, Print(&display)).map_err(ReadlineError::Io)?;
            queue!(_stdout, ResetColor).map_err(ReadlineError::Io)?;
        }

        let last_popup_row = if scroll_end < total {
            let row = start_row + height as u16;
            queue!(_stdout, MoveTo(0, row)).map_err(ReadlineError::Io)?;
            queue!(
                _stdout,
                SetBackgroundColor(self.theme.completion_bg),
                SetForegroundColor(self.theme.completion_fg),
                Print(&format!(
                    "{:width$}",
                    format!("… {} more", total - scroll_end),
                    width = popup_width
                )),
                ResetColor
            )
            .map_err(ReadlineError::Io)?;
            row
        } else if scroll_end > scroll_start {
            start_row + (scroll_end - scroll_start - 1) as u16
        } else {
            start_row
        };

        self.popup_rendered = Some((start_row, last_popup_row));

        Ok(())
    }

    fn read_event(&self) -> Result<KeyEvent, ReadlineError> {
        loop {
            match read() {
                Ok(Event::Key(ke))
                    if ke.kind == KeyEventKind::Press || ke.kind == KeyEventKind::Repeat =>
                {
                    return Ok(KeyEvent::from(ke));
                }
                Ok(Event::Paste(content)) => {
                    return Ok(KeyEvent::Paste(content));
                }
                Ok(Event::Resize(_w, _h)) => continue,
                Ok(_) => continue,
                Err(e) => return Err(ReadlineError::Io(e)),
            }
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    let mut in_csi = false;
    for c in s.chars() {
        if in_escape {
            if c == '[' {
                in_csi = true;
            } else if !in_csi && c.is_ascii_alphabetic() {
                in_escape = false;
            } else if in_csi && c.is_ascii_alphabetic() {
                in_escape = false;
                in_csi = false;
            }
            continue;
        }
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        result.push(c);
    }
    result
}

fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;
    let mut in_csi = false;
    for c in s.chars() {
        if in_escape {
            if c == '[' {
                in_csi = true;
            } else if !in_csi && c.is_ascii_alphabetic() {
                in_escape = false;
            } else if in_csi && c.is_ascii_alphabetic() {
                in_escape = false;
                in_csi = false;
            }
            continue;
        }
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        width += c.width().unwrap_or(0);
    }
    width
}

#[derive(Debug)]
pub enum ReadlineError {
    Interrupted,
    Eof,
    Io(io::Error),
}

fn fuzzy_match(query: &str, candidate: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let q: Vec<char> = query.chars().collect();
    let mut qi = 0;
    for c in candidate.chars() {
        if qi < q.len() && c.eq_ignore_ascii_case(&q[qi]) {
            qi += 1;
        }
    }
    qi == q.len()
}

impl std::fmt::Display for ReadlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadlineError::Interrupted => write!(f, "Interrupted"),
            ReadlineError::Eof => write!(f, "EOF"),
            ReadlineError::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for ReadlineError {}
