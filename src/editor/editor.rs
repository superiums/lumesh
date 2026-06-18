use std::collections::HashMap;
use std::io::{self, Write, stdout};

use crossterm::cursor::MoveTo;
use crossterm::event::{Event, KeyEventKind, read};
use crossterm::queue;
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode, size};

use super::buffer::LineBuffer;
use super::history::History;
use super::key::{Cmd, KeyEvent};
use super::kring::KillRing;

const HINT_COLOR: Color = Color::DarkGrey;
const COMP_BG: Color = Color::DarkGrey;
const COMP_FG: Color = Color::White;
const COMP_SEL_BG: Color = Color::White;
const COMP_SEL_FG: Color = Color::Black;
const MAX_POPUP_HEIGHT: usize = 10;

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub display: String,
    pub replacement: String,
    pub suffix: char,
}

impl CompletionItem {
    pub fn new(replacement: String) -> Self {
        let display = replacement.clone();
        Self {
            display,
            replacement,
            suffix: ' ',
        }
    }

    pub fn with_display(display: String, replacement: String) -> Self {
        Self {
            display,
            replacement,
            suffix: ' ',
        }
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

#[derive(Clone)]
enum EditorMode {
    Normal,
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

    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn readline(&mut self, prompt: &str) -> Result<String, ReadlineError> {
        enable_raw_mode().map_err(ReadlineError::Io)?;

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

        let result = self.event_loop();

        if result.is_ok() {
            let _ = write!(stdout(), "\r\n");
        }

        let _ = disable_raw_mode();
        result
    }

    fn event_loop(&mut self) -> Result<String, ReadlineError> {
        loop {
            self.render()?;

            let event = self.read_event()?;

            // Completion mode: handle events
            if let EditorMode::CompletionSelect { .. } = self.mode.clone() {
                if self.try_completion_event(&event) {
                    continue;
                }
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

            match event {
                KeyEvent::Char(' ') => {
                    self.handle_space();
                }
                KeyEvent::Char(c) | KeyEvent::Shift(c) => {
                    self.leave_completion();
                    self.buffer.insert(c);
                }
                KeyEvent::Ctrl(c) if c == 'c' => {
                    return Err(ReadlineError::Interrupted);
                }
                KeyEvent::Ctrl(c) if c == 'd' => {
                    if self.buffer.is_empty() {
                        return Err(ReadlineError::Eof);
                    }
                    self.buffer.delete();
                    self.leave_completion();
                }
                KeyEvent::Ctrl(c) if c == 's' => {
                    // May want to add Ctrl+S binding later
                }
                KeyEvent::Ctrl(c) if c == 'l' => {
                    let _ = terminal::Clear(ClearType::All);
                    self.prompt_row = 0;
                }
                KeyEvent::Enter => {
                    return self.accept_line();
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
                    if let Some(line) = self.history.previous(&self.buffer.text()) {
                        self.buffer.set_text(line);
                        self.buffer.move_to_end();
                    }
                }
                KeyEvent::Down => {
                    self.leave_completion();
                    if let Some(line) = self.history.next(&self.buffer.text()) {
                        self.buffer.set_text(line);
                        self.buffer.move_to_end();
                    }
                }
                KeyEvent::Home => {
                    self.leave_completion();
                    self.buffer.move_to_start();
                }
                KeyEvent::End => {
                    self.leave_completion();
                    self.buffer.move_to_end();
                }
                KeyEvent::Escape => {
                    if matches!(self.mode, EditorMode::CompletionSelect { .. }) {
                        self.mode = EditorMode::Normal;
                    } else {
                        // do nothing for bare escape
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
        if !text.is_empty() {
            self.history.add(text.clone());
        }
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
                self.mode = EditorMode::Normal;
                true
            }
            KeyEvent::Escape => {
                self.mode = EditorMode::Normal;
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
        let (old_completions, old_sel) = match &self.mode {
            EditorMode::CompletionSelect {
                completions,
                selected,
                ..
            } => (completions.clone(), *selected),
            _ => return,
        };

        let new_completions: Vec<CompletionItem> = if self.is_history_completion {
            let start_pos = Self::find_word_start(&line, pos);
            let query = &line[start_pos..pos];
            old_completions
                .into_iter()
                .filter(|item| fuzzy_match(query, &item.replacement))
                .collect()
        } else {
            // Parameter completion: filter existing completions with fuzzy match
            let query_start = self.init_search_pos.min(pos);
            let query = &line[query_start..pos];
            old_completions
                .into_iter()
                .filter(|item| fuzzy_match(query, &item.replacement))
                .collect()
        };

        let start_pos = Self::find_word_start(&line, pos);
        if new_completions.is_empty() {
            self.mode = EditorMode::Normal;
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
        let replacement = format!("{}{}", item.replacement, item.suffix);
        self.buffer.replace_range(start_pos, end_pos, &replacement);
        self.buffer.move_to_end();
    }

    // ---- Abbreviation handling ----

    fn handle_space(&mut self) {
        let text = self.buffer.text();
        if !text.contains(' ') {
            if let Some(expanded) = self.abbreviations.get(text.trim()) {
                self.buffer.set_text(expanded);
                self.buffer.move_to_end();
                return;
            }
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
            Cmd::AcceptLine => {}
            Cmd::Cancel => {
                self.leave_completion();
                if self.history.is_navigating() {
                    if let Some(restored) = self.history.cancel_navigation() {
                        self.buffer.set_text(&restored);
                    }
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
                let start_pos = Self::find_word_start(&line, pos);
                let query = &line[start_pos..pos];
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
    }

    fn handle_tab(&mut self) {
        let line = self.buffer.text();
        let pos = self.buffer.cursor();
        self.init_search_pos = pos;
        if let Some(ref completer) = self.completer {
            let completions = completer.complete(&line, pos);
            if completions.is_empty() {
                return;
            }
            let start_pos = if line[..pos].contains(|c| c == '/' || c == '\\') {
                Self::find_path_start(&line, pos)
            } else {
                Self::find_word_start(&line, pos)
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
                'k' => Cmd::DeleteToEnd,
                'l' => Cmd::ClearScreen,
                'n' => Cmd::HistoryNext,
                'p' => Cmd::HistoryPrevious,
                'r' => Cmd::HistorySearch,
                't' => Cmd::TransposeChars,
                'u' => Cmd::DeleteToStart,
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
        let text = self.buffer.text();
        let pos = self.buffer.cursor();
        if pos == 0 || text.len() < 2 {
            return;
        }
        let start = if pos == text.len() { pos - 2 } else { pos - 1 };
        let end = start + 2;
        if end > text.len() {
            return;
        }
        let chars: Vec<char> = text.chars().collect();
        let mut new_chars = chars.clone();
        new_chars.swap(start, end - 1);
        let new_text: String = new_chars.iter().collect();
        self.buffer.set_text(&new_text);
        self.buffer.set_cursor(start + 2);
    }

    fn leave_completion(&mut self) {
        if matches!(self.mode, EditorMode::CompletionSelect { .. }) {
            self.mode = EditorMode::Normal;
        }
        self.is_history_completion = false;
    }

    fn find_word_start(line: &str, pos: usize) -> usize {
        let before = &line[..pos.min(line.len())];
        before
            .rfind(
                |c: char| c.is_ascii_whitespace() || c == '.' || c == '>' || c == ':',
            )
            .map(|i| i + 1)
            .unwrap_or(0)
    }

    fn find_path_start(line: &str, pos: usize) -> usize {
        let before = &line[..pos.min(line.len())];
        before
            .rfind(|c: char| {
                c.is_ascii_whitespace()
                    || c == '>' || c == ':' || c == '|' || c == '&' || c == '(' || c == ';'
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

        queue!(
            stdout,
            MoveTo(0, self.prompt_row),
            Clear(ClearType::FromCursorDown)
        )
        .map_err(ReadlineError::Io)?;

        queue!(stdout, Print(&self.prompt)).map_err(ReadlineError::Io)?;

        if let Some(ref hl) = self.highlighter {
            let highlighted = hl.highlight(&line);
            queue!(stdout, Print(&highlighted)).map_err(ReadlineError::Io)?;
        } else {
            queue!(stdout, Print(&line)).map_err(ReadlineError::Io)?;
        }

        // Cache and render hint
        self.current_hint = None;
        if let Some(ref hinter) = self.hinter {
            if let Some(hint) = hinter.hint(&line, cursor) {
                self.current_hint = Some(hint.clone());
                queue!(
                    stdout,
                    SetForegroundColor(HINT_COLOR),
                    Print(&hint),
                    ResetColor
                )
                .map_err(ReadlineError::Io)?;
            }
        }

        // Compute how many terminal rows the prompt+line occupies (for popup positioning)
        let content_end = self.prompt_width + cursor.max(line.len());
        let used_rows = 1 + content_end / self.terminal_width as usize;

        // Render completion popup
        if let EditorMode::CompletionSelect {
            completions,
            selected,
            ..
        } = self.mode.clone()
        {
            self.render_completion_popup(&mut stdout, &completions, selected, used_rows)?;
        }

        // Position cursor
        let total_col = self.prompt_width + cursor;
        let vis_width = self.terminal_width as usize;
        let row = self.prompt_row + (total_col / vis_width) as u16;
        let col = (total_col % vis_width) as u16;

        queue!(stdout, MoveTo(col, row)).map_err(ReadlineError::Io)?;
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

        for i in scroll_start..scroll_end {
            let display_idx = i - scroll_start;
            let row = start_row + display_idx as u16;
            let item = &completions[i];
            let is_selected = i == selected;

            queue!(_stdout, MoveTo(0, row)).map_err(ReadlineError::Io)?;

            if is_selected {
                queue!(
                    _stdout,
                    SetBackgroundColor(COMP_SEL_BG),
                    SetForegroundColor(COMP_SEL_FG)
                )
                .map_err(ReadlineError::Io)?;
            } else {
                queue!(
                    _stdout,
                    SetBackgroundColor(COMP_BG),
                    SetForegroundColor(COMP_FG)
                )
                .map_err(ReadlineError::Io)?;
            }

            let display = if item.display.len() > popup_width {
                let truncated: String = item
                    .display
                    .chars()
                    .take(popup_width.saturating_sub(1))
                    .collect();
                format!("{}…", truncated)
            } else {
                format!("{:width$}", item.display, width = popup_width)
            };

            queue!(_stdout, Print(&display)).map_err(ReadlineError::Io)?;
            queue!(_stdout, ResetColor).map_err(ReadlineError::Io)?;
        }

        let last_popup_row = if scroll_end < total {
            let row = start_row + height as u16;
            queue!(_stdout, MoveTo(0, row)).map_err(ReadlineError::Io)?;
            queue!(
                _stdout,
                SetBackgroundColor(COMP_BG),
                SetForegroundColor(COMP_FG),
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
        width += 1;
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
        if qi < q.len() && c.to_ascii_lowercase() == q[qi].to_ascii_lowercase() {
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
