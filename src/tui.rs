use lumesh::runtime::{run_file, run_text};
use lumesh::syntax::highlight;
use lumesh::{Environment, Expression};
mod binary;

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
use std::cell::RefCell;
use std::rc::Rc;
use std::{
    error::Error,
    io::{self, Write},
    time::Duration,
};
const INPUT_LINES_DEFAULT: u16 = 10;

// #[macro_export]
// macro_rules! tui_println {
//     ($($arg:tt)*) => {{
//         use std::io::Write;
//         let msg = format!($($arg)*);
//         crate::app::get_app().stdout_redirect.write_all(msg.as_bytes()).unwrap();
//         crate::app::get_app().stdout_redirect.write_all(b"\n").unwrap();
//     }};
// }

struct App {
    /// 输入状态
    input: String,
    input_history: Vec<String>,
    history_index: usize,
    input_height: u16,
    error_history: Vec<String>,
    output_history: Vec<(String, String)>,

    /// 输出状态
    output_buffer: String,
    error_buffer: String,
    show_errors: bool,

    /// 显示控制
    fullscreen: Option<usize>, // None/Some(0)=output/Some(1)=errors
    active_tab: usize,

    /// 补全功能
    suggestions: Vec<String>,
    suggestion_index: usize,
    show_suggestions: bool,

    /// 环境
    env: Environment,
    should_quit: bool,
    // 重定向
    stdout_redirect: OutputRedirect,
    stderr_redirect: ErrorRedirect,
}

impl App {
    fn new() -> io::Result<Self> {
        let stdout_redirect = OutputRedirect::new();
        let stderr_redirect = ErrorRedirect::new();
        let mut env = Environment::new();
        // init bin
        binary::init(&mut env);
        // 设置全局重定向
        set_output(Rc::new(RefCell::new(stdout_redirect.clone())));
        set_error(Rc::new(RefCell::new(stderr_redirect.clone())));
        // replace_globals()?;

        Ok(Self {
            stdout_redirect,
            stderr_redirect,
            input: String::new(),
            input_history: vec![],
            error_history: vec![],
            output_history: vec![],
            history_index: 0,
            input_height: INPUT_LINES_DEFAULT,
            output_buffer: "Welcome to TUI Shell\n".to_string(),
            error_buffer: String::new(),
            show_errors: true,
            fullscreen: None,
            active_tab: 0,
            suggestions: vec![],
            suggestion_index: 0,
            show_suggestions: false,
            should_quit: false,
            env,
        })
    }

    fn submit_input(&mut self) -> io::Result<()> {
        // 在命令执行前清空所有缓冲区
        self.stdout_redirect.clear();
        self.stderr_redirect.clear();
        self.error_buffer.clear(); // 新增：清空错误显示缓冲区
        self.output_buffer.clear();

        // ...执行命令...

        let input = self.input.trim().to_string();
        if input.is_empty() {
            return Ok(());
        }

        // 保存历史
        self.input_history.push(input.clone());
        self.history_index = self.input_history.len();

        // 执行命令
        match run_text(input.as_str(), &mut self.env) {
            Ok(output) => {
                // output 内部换行问题
                self.output_buffer.push_str(&format!("> {}\n", output));

                match Expression::Apply(
                    Box::new(Expression::Symbol("report".to_string())),
                    vec![output],
                )
                .eval(&mut self.env)
                {
                    Ok(out2) => self.output_buffer.push_str(&format!("> {}\n", out2)),
                    Err(e) => {
                        self.error_history.push(e.to_string());
                        self.error_buffer = e.to_string();
                    }
                }

                // tui_println!(">{}", output); // 这会自动重定向到我们的缓冲区
                // println!(">>{}", output); // 这会自动重定向到我们的缓冲区
            }
            Err(e) => {
                // eprintln!("[err]>{}", e); // 这会自动重定向到错误缓冲区
                self.error_history.push(e.to_string());
                self.error_buffer = e.to_string(); // 替换而不是追加错误
            }
        }

        // 获取输出并追加到相应区域
        // self.output_buffer
        //     .push_str(&self.stdout_redirect.get_output());
        // self.error_buffer
        //     .push_str(&self.stderr_redirect.get_error());
        // 获取输出时追加而不是替换
        let new_output = self.stdout_redirect.get_output();
        if !new_output.is_empty() {
            self.output_buffer.push_str(&new_output);
        }

        let new_error = self.stderr_redirect.get_error();
        if !new_error.is_empty() {
            self.error_buffer = new_error; // 替换而不是追加错误
        }

        // 重置输入
        self.input.clear();
        self.input_height = INPUT_LINES_DEFAULT;
        self.show_suggestions = false;
        Ok(())
    }

    fn adjust_input_height(&mut self) {
        // let line_count = self.input.lines().count().max(1) as u16 + INPUT_LINES_DEFAULT;
        // self.input_height = line_count.clamp(1, 10); // 限制1-10行
    }

    // 补全功能实现
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
    // 全屏
    fn toggle_fullscreen(&mut self, area: usize) {
        self.fullscreen = match self.fullscreen {
            None => Some(area),
            Some(current) if current == area => None,
            _ => Some(area),
        };
    }
}
fn main() -> io::Result<()> {
    // 替换全局标准输出/错误
    // replace_globals();

    // 初始化终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建应用
    let mut app = App::new()?;

    // 主循环
    while !app.should_quit {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_input(&mut app, key);
            }
        }
    }

    // 清理终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

fn handle_input(app: &mut App, key: event::KeyEvent) -> Result<(), std::io::Error> {
    return match key.code {
        // 输入控制
        KeyCode::Enter => app.submit_input(),

        KeyCode::Backspace => {
            app.input.pop();
            app.adjust_input_height();
            app.update_suggestions();
            return Ok(());
        }

        // 历史导航
        KeyCode::Up if app.history_index > 0 => {
            app.history_index -= 1;
            app.input = app.input_history[app.history_index].clone();
            app.adjust_input_height();
            return Ok(());
        }
        KeyCode::Down if app.history_index < app.input_history.len() => {
            app.history_index += 1;
            app.input = if app.history_index == app.input_history.len() {
                String::new()
            } else {
                app.input_history[app.history_index].clone()
            };
            app.adjust_input_height();
            return Ok(());
        }
        // 补全控制
        KeyCode::Tab => {
            app.show_suggestions = !app.suggestions.is_empty();
            if app.show_suggestions {
                app.suggestion_index = (app.suggestion_index + 1) % app.suggestions.len();
            }
            return Ok(());
        }
        KeyCode::BackTab => {
            app.update_suggestions();
            if !app.suggestions.is_empty() {
                app.suggestion_index =
                    (app.suggestion_index + app.suggestions.len() - 1) % app.suggestions.len();
            }
            return Ok(());
        }

        KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.accept_suggestion(true); // 接受全部建议
            return Ok(());
        }
        KeyCode::Down if key.modifiers == KeyModifiers::CONTROL => {
            app.accept_suggestion(false); // 接受一个单词
            return Ok(());
        }
        // 视图控制
        KeyCode::Char(c) => {
            match c {
                '1' if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.toggle_fullscreen(0);
                }
                '2' if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.toggle_fullscreen(1);
                }
                'j' if key.modifiers == KeyModifiers::CONTROL => {
                    app.active_tab = (app.active_tab + 1) % 3;
                }
                'k' if key.modifiers == KeyModifiers::CONTROL => {
                    app.active_tab = (app.active_tab + 2) % 3;
                }
                _ => {
                    app.input.push(c);
                    app.adjust_input_height();
                    app.update_suggestions();
                }
            }
            return Ok(());
        }
        // 退出
        KeyCode::Esc => {
            app.should_quit = true;
            return Ok(());
        }
        _ => {
            return Ok(());
        }
    };
}

fn ui(frame: &mut Frame, app: &mut App) {
    // 顶层布局（上下分割）
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),                   // 主内容区
            Constraint::Length(app.input_height), // 输入区
        ])
        .split(frame.area());

    // 主内容区左右分割
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22), // 左侧功能区
            Constraint::Percentage(78), // 右侧显示区
        ])
        .split(main_layout[0]);

    // 右侧显示区垂直分割
    let display_layout = match app.fullscreen {
        None => Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // 当前命令显示
                Constraint::Min(1),    // 输出区
                Constraint::Length(6), // 错误区
                                       // Constraint::Length(if app.show_errors { 5 } else { 0 }), // 错误区
            ])
            .split(content_layout[1]),
        Some(0) => vec![content_layout[1], Rect::default(), Rect::default()].into(),
        Some(1) => vec![Rect::default(), content_layout[1], Rect::default()].into(),
        _ => unreachable!(),
    };

    // 根据全屏状态渲染不同内容
    match app.fullscreen {
        None => {
            // 正常渲染所有区域
        }
        Some(0) => {
            // 只渲染输出区域
            let output_paragraph = Paragraph::new(app.output_buffer.clone()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Output (Fullscreen)"),
            );
            frame.render_widget(output_paragraph, display_layout[0]);
        }
        Some(1) => {
            // 只渲染错误区域
            let error_text = if let Some(last_error) = app.error_history.last() {
                last_error.as_str()
            } else {
                "No errors"
            };
            let error_paragraph = Paragraph::new(error_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Errors (Fullscreen)"),
            );
            frame.render_widget(error_paragraph, display_layout[1]);
        }
        _ => unreachable!(),
    }
    // 渲染各个组件...
    // 左侧功能区
    let left_panel = match app.active_tab {
        0 => {
            // 历史命令
            let history_items: Vec<ListItem> = app
                .input_history
                .iter()
                .rev()
                .take(10)
                .map(|cmd| ListItem::new(Line::from(cmd.as_str())))
                .collect();
            List::new(history_items)
                .block(Block::default().borders(Borders::RIGHT).title("History"))
        }
        1 => {
            // 常用命令
            let favorites = vec![
                "ls",
                "cd",
                "clear",
                "exit",
                "cat",
                "grep",
                "find",
                "ps",
                "top",
                "vim",
                "git status",
            ];
            let favorite_items = favorites.iter().map(|cmd| ListItem::new(Line::from(*cmd)));
            List::new(favorite_items)
                .block(Block::default().borders(Borders::RIGHT).title("Favorites"))
        }
        _ => {
            // 帮助信息
            let help_items = vec![
                "Ctrl+H: Switch left",
                "Ctrl+L: Switch right",
                "Tab: Next suggestion",
                "Shift+Tab: Prev suggestion",
                "Ctrl+→: Accept all",
                "Ctrl+↓: Accept word",
                "Esc: Quit",
            ];
            let help_list = help_items
                .iter()
                .map(|text| ListItem::new(Line::from(*text)));
            List::new(help_list).block(Block::default().borders(Borders::RIGHT).title("Help"))
        }
    };
    frame.render_widget(left_panel, content_layout[0]);

    // 当前命令显示
    let current_cmd = if app.input.is_empty() && !app.input_history.is_empty() {
        app.input_history.last().unwrap()
    } else {
        &app.input
    };
    let cmd_line = Line::from(Span::styled(
        format!("> {}", current_cmd),
        Style::default().fg(Color::Yellow),
    ));
    let cmd_block = Block::default().borders(Borders::BOTTOM);
    let cmd_paragraph = Paragraph::new(cmd_line).block(cmd_block);
    frame.render_widget(cmd_paragraph, display_layout[0]);

    // 输出区域
    let output_paragraph = Paragraph::new(Line::from(app.output_buffer.clone()))
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(output_paragraph, display_layout[1]);

    // 错误区域
    if app.show_errors {
        // let error_text = if let Some(last_error) = app.error_history.last() {
        //     last_error.as_str()
        // } else if app.error_history.is_empty() {
        //     "No errors"
        // } else {
        //     "Multiple errors occurred"
        // };
        // let error_paragraph = Paragraph::new(Line::from(Span::styled(
        //     error_text,
        //     Style::default().fg(Color::Red),
        // )))
        // .block(Block::default().borders(Borders::TOP).title("Errors"));
        let error_paragraph = Paragraph::new(app.error_buffer.clone())
            .block(Block::default().borders(Borders::ALL).title("Errors"));

        frame.render_widget(error_paragraph, display_layout[2]);
    }

    // 底部输入区
    let input_line = if app.suggestions.is_empty() {
        Line::from(Span::styled(&app.input, Style::default()))
    } else {
        let suggestion = &app.suggestions[app.suggestion_index];
        Line::from(vec![
            Span::styled(&app.input, Style::default()),
            Span::styled(
                &suggestion[app.input.len()..],
                Style::default().fg(Color::DarkGray),
            ),
        ])
    };
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Input")
        .style(Style::default().fg(Color::White));
    let input_paragraph = Paragraph::new(input_line).block(input_block);
    frame.render_widget(input_paragraph, main_layout[1]);
}

// 输出重定向辅助结构

#[derive(Clone)]
pub struct OutputRedirect {
    buffer: Rc<RefCell<String>>,
}

impl OutputRedirect {
    pub fn new() -> Self {
        Self {
            buffer: Rc::new(RefCell::new(String::new())),
        }
    }

    // pub fn get_output(&self) -> String {
    //     self.buffer.borrow().clone()
    // }

    pub fn clear(&self) {
        self.buffer.borrow_mut().clear();
    }
}
impl OutputRedirect {
    pub fn get_output(&self) -> String {
        let output = self.buffer.borrow().clone();
        self.buffer.borrow_mut().clear(); // 获取后自动清空
        output
    }
}

impl ErrorRedirect {
    pub fn get_error(&self) -> String {
        let error = self.buffer.borrow().clone();
        self.buffer.borrow_mut().clear(); // 获取后自动清空
        error
    }
}
impl Write for OutputRedirect {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        self.buffer.borrow_mut().push_str(&s);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// 同样的实现用于错误输出
#[derive(Clone)]
pub struct ErrorRedirect {
    buffer: Rc<RefCell<String>>,
}

impl ErrorRedirect {
    pub fn new() -> Self {
        Self {
            buffer: Rc::new(RefCell::new(String::new())),
        }
    }

    // pub fn get_error(&self) -> String {
    //     self.buffer.borrow().clone()
    // }

    pub fn clear(&self) {
        self.buffer.borrow_mut().clear();
    }
}

impl Write for ErrorRedirect {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        self.buffer.borrow_mut().push_str(&s);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

//
// =====

thread_local! {
    static GLOBAL_STDOUT: RefCell<Option<Rc<RefCell<dyn Write>>>> = RefCell::new(None);
    static GLOBAL_STDERR: RefCell<Option<Rc<RefCell<dyn Write>>>> = RefCell::new(None);
}

pub fn set_output(writer: Rc<RefCell<dyn Write>>) {
    GLOBAL_STDOUT.with(|g| {
        *g.borrow_mut() = Some(writer);
    });
}

pub fn set_error(writer: Rc<RefCell<dyn Write>>) {
    GLOBAL_STDERR.with(|g| {
        *g.borrow_mut() = Some(writer);
    });
}

pub struct GlobalStdout;
pub struct GlobalStderr;

impl Write for GlobalStdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        GLOBAL_STDOUT.with(|g| {
            if let Some(ref w) = *g.borrow() {
                w.borrow_mut().write(buf)
            } else {
                io::stdout().write(buf)
            }
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        GLOBAL_STDOUT.with(|g| {
            if let Some(ref w) = *g.borrow() {
                w.borrow_mut().flush()
            } else {
                io::stdout().flush()
            }
        })
    }
}

impl Write for GlobalStderr {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        GLOBAL_STDERR.with(|g| {
            if let Some(ref w) = *g.borrow() {
                w.borrow_mut().write(buf)
            } else {
                io::stderr().write(buf)
            }
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        GLOBAL_STDERR.with(|g| {
            if let Some(ref w) = *g.borrow() {
                w.borrow_mut().flush()
            } else {
                io::stderr().flush()
            }
        })
    }
}

// 替换标准输出的安全方法
// pub fn replace_globals() -> io::Result<()> {
//     let stdout = GlobalStdout;
//     let stderr = GlobalStderr;

//     // 注意：这使用了不安全的代码，但在单线程TUI应用中是安全的
//     // unsafe {
//     //     io::set_print(Some(Box::new(stdout)));
//     //     io::set_panic(Some(Box::new(stderr)));
//     // }
//     Ok(())
// }

// ## 新增快捷键说明

// | 快捷键        | 功能                      |
// |---------------|--------------------------|
// | Ctrl+→        | 接受完整补全建议          |
// | Ctrl+↓        | 接受单词级补全建议        |
// | Ctrl+1        | 输出区全屏                |
// | Ctrl+2        | 错误区全屏                |
// | Ctrl+E        | 显示/隐藏错误区           |
