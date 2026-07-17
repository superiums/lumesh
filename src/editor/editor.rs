use super::buffer::LineBuffer;
use super::history::History;
use super::key::{Cmd, KeyEvent};
use super::kring::KillRing;
use crate::ai::{AIClient, MockAIClient};
use crate::editor::key::shift_char;
use crossterm::cursor::MoveTo;
use crossterm::event::{Event, KeyEventKind, read};
use crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::queue;
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode, size};
use std::collections::HashMap;
use std::io::{self, Write, stdout};
use std::sync::{Arc, mpsc};
use unicode_width::UnicodeWidthChar;

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
            completion_bg: Color::Black,
            completion_fg: Color::DarkYellow,
            completion_selected_bg: Color::Red,
            completion_selected_fg: Color::White,
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
    pub fn with(replacement: String, suffix: char) -> Self {
        Self {
            display: None,
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
    ai_client: Option<Arc<MockAIClient>>,
    is_ai_hinting: bool,
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
            ai_client: None,
            is_ai_hinting: false,
        }
    }

    pub fn set_ai_client(&mut self, client: Arc<MockAIClient>) {
        self.ai_client = Some(client);
    }

    fn trigger_ai_hint(&mut self) {
        let line = self.buffer.text();
        if line.trim().is_empty() {
            return;
        }

        if self.ai_client.is_some() {
            self.show_ai_temp_hint(" [AI Hinting...]".to_string());
        } else {
            self.show_ai_temp_hint(" [AI Disabled]".to_string());
        }

        if let Some(ref ai) = self.ai_client {
            let ai = Arc::clone(ai);
            let prompt = line.clone();
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let result = ai.as_ref().complete(&prompt).ok();
                let _ = tx.send(result);
            });

            match rx.recv_timeout(std::time::Duration::from_secs(10)) {
                Ok(Some(hint)) => {
                    if !hint.is_empty() {
                        self.current_hint = Some(hint);
                        self.is_ai_hinting = true;
                    } else {
                        self.show_ai_temp_hint(" [AI Answer Blank]".to_string());
                    }
                }
                Ok(_) => {
                    self.show_ai_temp_hint(" [AI No Answer]".to_string());
                }
                Err(e) => {
                    self.show_ai_temp_hint(" [AI Err]".to_string() + &e.to_string());
                }
            }
        } else {
            self.show_ai_temp_hint(" [AI Disabled]".to_string());
        }
    }

    fn trigger_ai_replace(&mut self) {
        let line = self.buffer.text();
        if line.trim().is_empty() {
            return;
        }
        if self.ai_client.is_some() {
            self.show_ai_temp_hint(" [AI Thinking...]".to_string());
        } else {
            self.show_ai_temp_hint(" [AI Disabled]".to_string());
        }
        if let Some(ref ai) = self.ai_client {
            let ai = Arc::clone(ai);
            let prompt = line.clone();
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let result = ai.chat(false, &prompt).ok();
                let _ = tx.send(result);
            });
            match rx.recv_timeout(std::time::Duration::from_secs(20)) {
                Ok(Some(result)) => {
                    let clean = result.trim().to_string();
                    if !clean.is_empty() {
                        self.buffer.set_text(&clean);
                        self.buffer.move_to_end();
                        self.is_ai_hinting = false;
                        self.current_hint = None;
                        self.show_hint = false;
                    }
                }
                Ok(_) => {
                    self.show_ai_temp_hint(" [AI No Answer]".to_string());
                }
                Err(e) => {
                    self.show_ai_temp_hint(" [AI Err]".to_string() + &e.to_string());
                }
            }
        }
    }

    fn show_ai_temp_hint(&mut self, msg: String) {
        self.current_hint = Some(msg);
        self.is_ai_hinting = true;
        let _ = self.render();
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
        // 在 raw mode 下重新启用 ONLCR，使 \n 自动转换为 \r\n
        // 这样外部程序（dbg!、fd 等）的输出也能正确换行
        #[cfg(unix)]
        unsafe {
            let mut termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDOUT_FILENO, &mut termios) == 0 {
                termios.c_oflag |= libc::OPOST | libc::ONLCR;
                libc::tcsetattr(libc::STDOUT_FILENO, libc::TCSANOW, &termios);
            }
        }

        // 在进入 raw mode 后启用  Kitty Keyboard Protocol
        let _ = execute!(
            std::io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        );

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
        // 退出时恢复  Kitty Keyboard Protocol 之前的状态
        let _ = execute!(std::io::stdout(), PopKeyboardEnhancementFlags);

        result
    }

    fn event_loop(&mut self) -> Result<String, ReadlineError> {
        loop {
            self.render()?;
            let event = self.read_event()?;

            // 1. Completion mode intercepts first
            if matches!(self.mode, EditorMode::CompletionSelect { .. })
                && self.try_completion_event(&event)
            {
                continue;
            }

            // 2. Hotkey function bindings
            if let Some(f) = self.hotkey_fns.get(&event) {
                let result = f(&self.buffer.text());

                // hotkey 内部命令可能禁用了 raw mode，必须恢复
                let _ = enable_raw_mode();
                // 命令输出可能导致终端滚动，重新查询 prompt_row
                if let Ok((_col, row)) = crossterm::cursor::position() {
                    self.prompt_row = row;
                }

                if let Some(text) = result {
                    self.buffer.delete_to_line_start();
                    self.buffer.insert_str(&text);
                    self.leave_completion();
                    if &text == "\n" {
                        return self.accept_line();
                    }
                }
                continue;
            }

            // 3. Custom bindings
            let custom_cmd = self.custom_bindings.get(&event).cloned();
            if let Some(cmd) = custom_cmd {
                match cmd {
                    Cmd::InsertStr(ref s) => {
                        self.buffer.insert_str(s);
                        self.leave_completion();
                    }
                    Cmd::AcceptLine => return self.accept_line(),
                    Cmd::Noop => {}
                    ref c => {
                        self.leave_completion();
                        self.handle_cmd(c.clone());
                    }
                }
                continue;
            }

            // 4. Global events — 与模式无关，统一处理
            match &event {
                KeyEvent::Paste(text) => {
                    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
                    self.buffer.insert_str(&normalized);
                    if normalized.contains('\n') {
                        self.mode = EditorMode::Multiline;
                    }
                    self.leave_completion();
                }
                KeyEvent::Ctrl('c') => return Err(ReadlineError::Interrupted),
                KeyEvent::Ctrl('d') => {
                    if self.buffer.is_empty() {
                        return Err(ReadlineError::Eof);
                    }
                    self.buffer.delete();
                    self.leave_completion();
                }
                KeyEvent::Alt('o') | KeyEvent::AltEnter => {
                    self.trigger_ai_replace();
                }
                KeyEvent::Alt('i') => {
                    self.trigger_ai_hint();
                }
                KeyEvent::Ctrl('l') => {
                    let _ = terminal::Clear(ClearType::All);
                    self.prompt_row = 0;
                }
                // KeyEvent::Backspace => {
                //     self.is_ai_hinting = false;
                //     self.leave_completion();
                //     self.buffer.backspace();
                //     true
                // }
                // KeyEvent::Delete => {
                //     self.leave_completion();
                //     self.buffer.delete();
                //     true
                // }
                KeyEvent::Tab => {
                    self.handle_tab();
                }
                KeyEvent::BackTab => {
                    self.handle_backtab();
                }
                KeyEvent::None => {}
                _ => {
                    // 5. 模式分发
                    self.show_hint = false;
                    let accepted = match self.mode {
                        EditorMode::Multiline => self.handle_multiline_event(event)?,
                        _ => self.handle_normal_event(event)?,
                    };
                    if accepted {
                        return self.accept_line();
                    }
                }
            };
        }
    }

    /// Normal 模式下的按键处理。返回 true 表示接受输入。
    fn handle_normal_event(&mut self, event: KeyEvent) -> Result<bool, ReadlineError> {
        match event {
            KeyEvent::Char(' ') => {
                self.is_ai_hinting = false;
                self.show_hint = true;
                self.handle_space();
            }
            KeyEvent::Shift(c) => {
                // only for third part raw mdoe, such as inquire
                let c = shift_char(c, false);
                self.is_ai_hinting = false;
                self.show_hint = true;
                self.leave_completion();
                self.buffer.insert(c);
            }
            KeyEvent::Char(c) => {
                self.is_ai_hinting = false;
                self.show_hint = true;
                self.leave_completion();
                self.buffer.insert(c);
            }
            KeyEvent::Enter => {
                self.is_ai_hinting = false;
                self.current_hint = None;
                self.show_hint = false;
                if self.should_accept() {
                    let _ = self.render(); // 清除 hint 后再接受
                    return Ok(true);
                } else {
                    let indent = self.buffer.current_line_indent();
                    self.buffer.insert('\n');
                    if !indent.is_empty() {
                        self.buffer.insert_str(&indent);
                    }
                    self.mode = EditorMode::Multiline;
                }
            }
            KeyEvent::Up => {
                self.leave_completion();
                if let Some(entry) = self.history.previous(&self.buffer.text()) {
                    self.buffer.set_text(entry);
                    self.buffer.move_to_end();
                    self.set_normal_mode();
                }
            }
            KeyEvent::Down => {
                self.leave_completion();
                if let Some(entry) = self.history.next(&self.buffer.text()) {
                    self.buffer.set_text(entry);
                    self.buffer.move_to_end();
                    self.set_normal_mode();
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
            // KeyEvent::Left => {
            //     self.leave_completion();
            //     self.buffer.move_left();
            // }
            // KeyEvent::Right => {
            //     self.leave_completion();
            //     self.buffer.move_right();
            // }
            _ => self.handle_other_events(event),
        }
        Ok(false)
    }

    /// Multiline 模式下的按键处理。返回 true 表示接受输入。
    fn handle_multiline_event(&mut self, event: KeyEvent) -> Result<bool, ReadlineError> {
        match event {
            KeyEvent::Char(' ') => {
                self.is_ai_hinting = false;
                self.show_hint = true;
                self.handle_space();
            }
            KeyEvent::Char(c) => {
                self.is_ai_hinting = false;
                self.show_hint = true;
                self.leave_completion();
                self.buffer.insert(c);
            }
            KeyEvent::Escape => {
                self.leave_completion();
                self.buffer.move_to_end();
            }
            KeyEvent::Enter => {
                self.is_ai_hinting = false;
                self.current_hint = None;
                self.show_hint = false;
                if self.buffer.cursor_on_empty_line() {
                    return Ok(true);
                }
                let indent = self.buffer.current_line_indent();
                self.buffer.insert('\n');
                if !indent.is_empty() {
                    self.buffer.insert_str(&indent);
                }
            }
            KeyEvent::Up => {
                self.leave_completion();
                self.move_cursor_up();
            }
            KeyEvent::Down => {
                self.leave_completion();
                self.move_cursor_down();
            }
            KeyEvent::Home => {
                self.leave_completion();
                self.buffer.move_to_line_start();
            }
            KeyEvent::End => {
                self.leave_completion();
                self.buffer.move_to_line_end();
            }
            // KeyEvent::Left => {
            //     self.leave_completion();
            //     self.buffer.move_left();
            // }
            // KeyEvent::Right => {
            //     self.leave_completion();
            //     self.buffer.move_right();
            // }
            _ => self.handle_other_events(event),
        }
        Ok(false)
    }

    fn accept_line(&mut self) -> Result<String, ReadlineError> {
        Ok(self.buffer.text())
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
            KeyEvent::Up | KeyEvent::Ctrl('p') | KeyEvent::BackTab => {
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
            KeyEvent::Enter | KeyEvent::Char(' ') => {
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
            // 修复：移除冗余的内层 match，直接计算
            KeyEvent::Ctrl('r') if self.is_history_completion => {
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
            KeyEvent::Char(c) => {
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
        // 确认当前处于补全模式，否则不处理
        if !matches!(self.mode, EditorMode::CompletionSelect { .. }) {
            return;
        }
        if self.is_history_completion {
            return self.handle_cmd(Cmd::HistorySearch);
        }

        let line = self.buffer.text();
        let pos = self.buffer.cursor();
        let byte_pos = line
            .char_indices()
            .nth(pos)
            .map(|(i, _)| i)
            .unwrap_or(line.len());

        let Some(ref completer) = self.completer else {
            return;
        };

        // 直接重新查询，不依赖旧列表
        let new_completions = completer.complete(&line, byte_pos);

        if new_completions.is_empty() {
            self.set_normal_mode();
        } else {
            let count = new_completions.len();
            // 保持选中项不越界，但不强行保留旧选中
            let current_sel = match &self.mode {
                EditorMode::CompletionSelect { selected, .. } => {
                    (*selected).min(count.saturating_sub(1))
                }
                _ => 0,
            };

            let start_pos = if line[..byte_pos].starts_with('-') {
                Self::find_word_start(&line, byte_pos)
            } else if line[..byte_pos].contains(['/', '\\'])
                || new_completions
                    .get(current_sel)
                    .is_some_and(|c| c.replacement.starts_with("."))
            {
                Self::find_path_start(&line, byte_pos)
            } else {
                Self::find_word_start(&line, byte_pos)
            };

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
        // 修改 buffer 的命令自动清除 AI hint
        // if self.is_ai_hinting {
        match &cmd {
                Cmd::Insert(_)
                | Cmd::InsertStr(_)
                | Cmd::InsertStrAtBeginning(_)
                | Cmd::Backspace
                | Cmd::Delete
                | Cmd::DeleteWordBefore
                | Cmd::DeleteWordAfter
                | Cmd::DeleteToStart
                | Cmd::DeleteToEnd
                | Cmd::DeleteToLineStart
                | Cmd::DeleteToLineEnd
                | Cmd::ClearBuffer
                // | Cmd::ToggleSudo(_)
                | Cmd::TransposeChars
                | Cmd::MoveLeft
                | Cmd::MoveRight
                | Cmd::MoveWordLeft
                | Cmd::MoveWordRight
                | Cmd::MoveToStart
                | Cmd::MoveToEnd
                // | Cmd::Yank
                // | Cmd::AcceptHint
                // | Cmd::AcceptHintWord(_)
                 => {
                    self.is_ai_hinting = false;
                    self.current_hint = None;
                }
                _ => {}
            }
        // }

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
            // 修复：MoveUp/MoveDown 实际移动光标（仅 Multiline 有意义）
            Cmd::MoveUp => {
                self.leave_completion();
                if matches!(self.mode, EditorMode::Multiline) {
                    self.move_cursor_up();
                }
            }
            Cmd::MoveDown => {
                self.leave_completion();
                if matches!(self.mode, EditorMode::Multiline) {
                    self.move_cursor_down();
                }
            }
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
                    .filter(|e| fuzzy_match(query, e))
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
                    let mut it = clean
                        // .trim_start()
                        .split_terminator('\0');
                    let trimmed = it.next().unwrap_or(&clean);
                    self.buffer.insert_str(trimmed);
                    // display params hint or None
                    self.current_hint = it.next().map(|x| format!("\0{}", x));
                }
                self.leave_completion();
            }
            Cmd::AcceptHintWord => {
                if let Some(ref hint) = self.current_hint.clone() {
                    let clean = strip_ansi(hint);
                    let word = {
                        let trimmed = clean
                            .trim_start()
                            .split_terminator('\0')
                            .next()
                            .unwrap_or(clean.trim_start());
                        let pos = trimmed.find(|c: char| c.is_ascii_whitespace() || c == '/');
                        let end = pos.map_or(trimmed.len(), |x| {
                            x + if clean.starts_with(' ') { 2 } else { 1 }
                        });
                        clean[..end].to_string()
                    };

                    self.buffer.insert_str(&word);
                    let remaining = clean[word.len()..].to_string();
                    if remaining.is_empty() {
                        self.current_hint = None;
                        if self.is_ai_hinting {
                            self.is_ai_hinting = false;
                        }
                    } else {
                        self.current_hint = Some(remaining);
                        self.is_ai_hinting = true; //借助它来避免在render中被清理
                    }
                }
                self.leave_completion();
            }
            Cmd::ClearBuffer => {
                self.buffer.set_text("");
                self.leave_completion();
            }
            Cmd::ToggleSudo(sudo_cmd) => {
                let sudo_cmd = sudo_cmd + " ";
                let text = self.buffer.text();
                if text.starts_with(&sudo_cmd) {
                    let new_text = text[sudo_cmd.len()..].to_string();
                    self.buffer.set_text(&new_text);
                } else {
                    let new_text = format!("{}{}", sudo_cmd, text);
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

    /// 删除当前行的一级缩进（1 个 tab 或最多 4 个空格）。
    /// 在 Normal 和 Multiline 模式下均有效。
    fn handle_backtab(&mut self) {
        let line = self.buffer.text();
        let cursor = self.buffer.cursor();

        // 找到当前行在 buffer 中的字节起始位置
        let byte_cursor = line
            .char_indices()
            .nth(cursor)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        let line_start_byte = line[..byte_cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);

        let current_line = &line[line_start_byte..];

        // 计算要删除的字符数：1 个 tab 或最多 4 个空格
        let remove_count = if current_line.starts_with('\t') {
            1
        } else {
            let spaces = current_line.chars().take_while(|c| *c == ' ').count();
            spaces.min(4)
        };

        if remove_count == 0 {
            return;
        }

        // replace_range 使用 char 索引
        let line_start_char = line[..line_start_byte].chars().count();
        self.buffer
            .replace_range(line_start_char, line_start_char + remove_count, "");
    }

    fn default_binding(&self, event: &KeyEvent) -> Cmd {
        match event {
            KeyEvent::Ctrl(c) => match c {
                'a' => Cmd::MoveToStart,
                'b' => Cmd::MoveLeft,
                'c' => Cmd::Noop, // 已在全局事件处理
                'd' => Cmd::Noop, // 已在全局事件处理
                'e' => Cmd::MoveToEnd,
                'f' => Cmd::MoveRight,
                'h' => Cmd::Backspace,
                'j' => Cmd::AcceptHint,
                'k' => Cmd::DeleteToLineEnd,
                // 'l' 已在全局事件处理，此处不再重复
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
                // 's' => Cmd::ToggleSudo,
                'j' => Cmd::AcceptHintWord,
                _ => Cmd::Noop,
            },
            // 导航与编辑键直接映射，统一走 handle_cmd
            KeyEvent::Backspace => Cmd::Backspace,
            KeyEvent::Delete => Cmd::Delete,
            KeyEvent::Left => Cmd::MoveLeft,
            KeyEvent::Right => Cmd::MoveRight,
            KeyEvent::CtrlBackspace => Cmd::DeleteWordBefore,
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

        if let Some((start, end)) = self.popup_rendered {
            for row in start..=end {
                let _ = queue!(stdout, MoveTo(0, row), Clear(ClearType::CurrentLine));
            }
            self.popup_rendered = None;
        }

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

        // 计算光标位置
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

        // hint 显示
        if !self.is_ai_hinting {
            if self.show_hint {
                if let Some(ref hinter) = self.hinter {
                    if let Some(hint) = hinter.hint(&line, byte_cursor) {
                        // 命令和参数hint
                        self.current_hint = Some(hint);
                    } else if let Some(hint) = self.history.search_hint(&line) {
                        // 历史命令hint
                        self.current_hint = Some(hint);
                    } else {
                        // 清空
                        self.current_hint = None;
                    };
                }
            }
        }

        if let Some(ref hint) = self.current_hint.clone() {
            let display = strip_ansi(hint);
            queue!(
                stdout,
                MoveTo(cursor_col, cursor_row),
                SetForegroundColor(self.theme.hint_color),
                Print(&display),
                ResetColor
            )
            .map_err(ReadlineError::Io)?;
        }

        // 渲染补全弹窗
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

        // 定位光标
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

// ---- 工具函数 ----

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

#[derive(Debug)]
pub enum ReadlineError {
    Interrupted,
    Eof,
    Io(io::Error),
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
