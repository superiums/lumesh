use std::{error::Error, time::Duration};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame, Terminal,
};

struct App {
    // 输入状态
    input: String,
    input_history: Vec<String>,
    output_history: Vec<String>,
    error_history: Vec<String>,
    history_index: usize,
    should_quit: bool,
    input_height: u16,
    
    // 补全状态
    suggestions: Vec<String>,
    suggestion_index: usize,
    active_tab: usize,
    
    // 布局状态
    show_errors: bool,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            input_history: vec![],
            output_history: vec![],
            error_history: vec![],
            history_index: 0,
            should_quit: false,
            input_height: 1,
            suggestions: vec![],
            suggestion_index: 0,
            active_tab: 0,
            show_errors: false,
        }
    }

    fn submit_input(&mut self) {
        if self.input.trim().is_empty() {
            return;
        }

        let input = self.input.clone();
        self.input_history.push(input.clone());
        self.history_index = self.input_history.len();

        // 模拟命令执行
        let output = if input == "clear" {
            self.output_history.clear();
            self.error_history.clear();
            "".to_string()
        } else if input == "exit" {
            self.should_quit = true;
            "Exiting...".to_string()
        } else if input == "toggle errors" {
            self.show_errors = !self.show_errors;
            format!("Error panel {}", if self.show_errors { "shown" } else { "hidden" })
        } else {
            // 模拟有时成功有时失败
            if input.contains("error") {
                self.error_history.push(format!("Error executing: {}", input));
                format!("Command '{}' failed", input)
            } else {
                format!("Executed: {}", input)
            }
        };

        self.output_history.push(output);
        self.input.clear();
        self.input_height = 1;
        self.suggestions.clear();
    }

    fn adjust_input_height(&mut self) {
        self.input_height = self.input.lines().count() as u16 + 1;
    }

    fn update_suggestions(&mut self) {
        if self.input.is_empty() {
            self.suggestions = self.input_history.clone();
        } else {
            self.suggestions = self.input_history
                .iter()
                .filter(|cmd| cmd.contains(&self.input))
                .cloned()
                .collect();
        }
        self.suggestion_index = 0;
    }

    fn accept_suggestion(&mut self, full: bool) {
        if self.suggestions.is_empty() {
            return;
        }

        if full {
            self.input = self.suggestions[self.suggestion_index].clone();
        } else {
            // 只接受下一个单词
            let suggestion = &self.suggestions[self.suggestion_index];
            let input_words: Vec<&str> = self.input.split_whitespace().collect();
            let suggestion_words: Vec<&str> = suggestion.split_whitespace().collect();
            
            if input_words.len() < suggestion_words.len() {
                let new_input = suggestion_words[..input_words.len()+1].join(" ");
                self.input = new_input;
            }
        }
        
        self.adjust_input_height();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // 设置终端
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建应用
    let mut app = App::new();

    // 主循环
    while !app.should_quit {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => app.submit_input(),
                    KeyCode::Char('l') if key.modifiers == KeyModifiers::CONTROL => {
                        app.active_tab = (app.active_tab + 1) % 3;
                    }
                    KeyCode::Char('h') if key.modifiers == KeyModifiers::CONTROL => {
                        app.active_tab = (app.active_tab + 2) % 3; // 3-1=2
                    }
                    KeyCode::Tab => {
                        app.update_suggestions();
                        if !app.suggestions.is_empty() {
                            app.suggestion_index = (app.suggestion_index + 1) % app.suggestions.len();
                        }
                    }
                    KeyCode::BackTab => {
                        app.update_suggestions();
                        if !app.suggestions.is_empty() {
                            app.suggestion_index = (app.suggestion_index + app.suggestions.len() - 1) % app.suggestions.len();
                        }
                    }
                    KeyCode::Right if key.modifiers == KeyModifiers::CONTROL => {
                        app.accept_suggestion(true); // 接受全部建议
                    }
                    KeyCode::Down if key.modifiers == KeyModifiers::CONTROL => {
                        app.accept_suggestion(false); // 接受一个单词
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                        app.adjust_input_height();
                        app.update_suggestions();
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                        app.adjust_input_height();
                        app.update_suggestions();
                    }
                    KeyCode::Up if app.history_index > 0 => {
                        app.history_index -= 1;
                        app.input = app.input_history[app.history_index].clone();
                        app.adjust_input_height();
                    }
                    KeyCode::Down if app.history_index < app.input_history.len() => {
                        app.history_index += 1;
                        app.input = if app.history_index == app.input_history.len() {
                            String::new()
                        } else {
                            app.input_history[app.history_index].clone()
                        };
                        app.adjust_input_height();
                    }
                    KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    _ => {}
                }
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

fn ui(frame: &mut Frame, app: &App) {
    // 主垂直布局
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 顶部功能区
            Constraint::Min(1),    // 主内容区
            Constraint::Length(app.input_height), // 底部输入区
        ])
        .split(frame.size());

    // 顶部功能区
    let top_tabs = Tabs::new(vec!["History", "Favorites", "Help"])
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(app.active_tab);
    frame.render_widget(top_tabs, main_layout[0]);

    // 主内容区水平分割
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // 左侧功能区
            Constraint::Percentage(80), // 右侧主区域
        ])
        .split(main_layout[1]);

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
                "ls", "cd", "clear", "exit", "cat", "grep", 
                "find", "ps", "top", "vim", "git status"
            ];
            let favorite_items = favorites
                .iter()
                .map(|cmd| ListItem::new(Line::from(*cmd)));
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
            List::new(help_list)
                .block(Block::default().borders(Borders::RIGHT).title("Help"))
        }
    };
    frame.render_widget(left_panel, content_layout[0]);

    // 右侧主区域垂直分割
    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 当前命令显示
            Constraint::Min(1),    // 输出区域
            Constraint::Length(if app.show_errors { 5 } else { 0 }), // 错误区域
        ])
        .split(content_layout[1]);

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
    frame.render_widget(cmd_paragraph, right_layout[0]);

    // 输出区域
    let output_text = if let Some(last_output) = app.output_history.last() {
        last_output.as_str()
    } else {
        "Output will appear here"
    };
    let output_paragraph = Paragraph::new(Line::from(output_text))
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(output_paragraph, right_layout[1]);

    // 错误区域
    if app.show_errors {
        let error_text = if let Some(last_error) = app.error_history.last() {
            last_error.as_str()
        } else if app.error_history.is_empty() {
            "No errors"
        } else {
            "Multiple errors occurred"
        };
        let error_paragraph = Paragraph::new(Line::from(Span::styled(
            error_text,
            Style::default().fg(Color::Red),
        )))
        .block(Block::default().borders(Borders::TOP).title("Errors"));
        frame.render_widget(error_paragraph, right_layout[2]);
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
    frame.render_widget(input_paragraph, main_layout[2]);
}