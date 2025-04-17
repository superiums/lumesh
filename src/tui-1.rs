use lumesh::Environment;
use lumesh::runtime::{run_file, run_text};
use lumesh::syntax::highlight;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Tabs, Wrap},
};
use std::{error::Error, io::Write, time::Duration};

const INPUT_LINES_DEFAULT: u16 = 6;

use environment::Environment;

struct App {
    // 输入相关
    input: String,
    input_history: Vec<String>,
    history_index: usize,
    input_height: u16,

    // 输出相关
    output_buffer: String,
    error_buffer: String,
    fullscreen: Option<usize>, // None/Some(0)=output/Some(1)=errors
    show_errors: bool,

    // 补全相关
    suggestions: Vec<String>,
    suggestion_index: usize,
    show_suggestions: bool,

    // 环境
    env: Environment,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            input_history: vec![],
            history_index: 0,
            input_height: INPUT_LINES_DEFAULT,
            output_buffer: "Welcome to TUI Shell\n".to_string(),
            error_buffer: String::new(),
            fullscreen: None,
            show_errors: true,
            suggestions: vec![],
            suggestion_index: 0,
            show_suggestions: false,
            env: Environment::new(),
        }
    }

    fn submit_input(&mut self) {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            return;
        }

        self.input_history.push(input.clone());
        self.history_index = self.input_history.len();

        // 执行并捕获输出
        let old_stdout = std::io::stdout();
        let old_stderr = std::io::stderr();

        // 重定向输出
        let mut output = Vec::new();
        {
            let _stdout_guard = StdoutRedirect::new(&mut output);
            let _stderr_guard = StderrRedirect::new(&mut self.error_buffer);

            match run_text(&input, self.env) {
                Ok(result) => {
                    println!("{}", result);
                    self.output_buffer.push_str(&format!("> {}\n", input));
                }
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        }

        // 恢复标准输出
        self.output_buffer
            .push_str(&String::from_utf8_lossy(&output));
        self.input.clear();
        self.input_height = INPUT_LINES_DEFAULT;
    }

    // 输入区高度管理
    fn adjust_input_height(&mut self) {
        let line_count = self.input.lines().count().max(1) as u16 + INPUT_LINES_DEFAULT;
        self.input_height = line_count.min(10); // 限制最大10行
    }

    // ... 省略其他方法 ...
}
// 完整补全功能实现
impl App {
    fn update_suggestions(&mut self) {
        if self.input.is_empty() {
            self.suggestions = self.input_history.clone();
        } else {
            self.suggestions = self
                .input_history
                .iter()
                .filter(|cmd| cmd.contains(&self.input))
                .cloned()
                .collect();
        }
        self.suggestion_index = 0;
        self.show_suggestions = !self.suggestions.is_empty();
    }

    fn accept_suggestion(&mut self, full: bool) {
        if !self.show_suggestions || self.suggestions.is_empty() {
            return;
        }

        if full {
            self.input = self.suggestions[self.suggestion_index].clone();
        } else {
            // 部分接受逻辑
            let suggestion = &self.suggestions[self.suggestion_index];
            let mut input_words = self.input.split_whitespace().collect::<Vec<_>>();
            let suggestion_words = suggestion.split_whitespace().collect::<Vec<_>>();

            if !suggestion_words.is_empty()
                && (input_words.is_empty() || input_words.len() < suggestion_words.len())
            {
                input_words.push(suggestion_words[input_words.len()]);
                self.input = input_words.join(" ");
            }
        }
        self.adjust_input_height();
    }
}

// ### 4. 全屏视图管理

impl App {
    fn toggle_fullscreen(&mut self, area: usize) {
        self.fullscreen = match self.fullscreen {
            None => Some(area),
            Some(current) if current == area => None,
            _ => Some(area),
        };
    }
}

// ### 5. UI渲染优化

fn ui(frame: &mut Frame, app: &mut App) {
    // 主布局（上下分割）
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),                   // 上方区域
            Constraint::Length(app.input_height), // 输入区
        ])
        .split(frame.area());

    // 上方区域左右分割
    let top_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // 左侧功能区
            Constraint::Percentage(70), // 右侧主区
        ])
        .split(main_layout[0]);

    // 右侧区域上下分割（根据全屏状态调整）
    let right_layout = match app.fullscreen {
        None => Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Length(if app.show_errors { 5 } else { 0 }),
            ])
            .split(top_layout[1]),
        Some(0) => vec![top_layout[1], Rect::default()].into(),
        Some(1) => vec![Rect::default(), top_layout[1]].into(),
        _ => unreachable!(),
    };

    // 渲染各区域...
}

// 输出重定向辅助结构
struct StdoutRedirect<'a> {
    inner: Box<dyn Write + 'a>,
}

impl<'a> StdoutRedirect<'a> {
    fn new(buffer: &'a mut Vec<u8>) -> Self {
        Self {
            inner: Box::new(buffer),
        }
    }
}

impl<'a> Write for StdoutRedirect<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

// 使用重定向辅助结构
struct StderrRedirect<'a> {
    inner: &'a mut String,
}

impl<'a> Write for StderrRedirect<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        self.inner.push_str(&s);
        Ok(buf.len())
    }
}

// 在事件处理中
// KeyCode::Char('1') if key.modifiers == KeyModifiers::CONTROL => {
//     app.toggle_fullscreen(0); // 输出区全屏
// }
// KeyCode::Char('2') if key.modifiers == KeyModifiers::CONTROL => {
//     app.toggle_fullscreen(1); // 错误区全屏
// }
