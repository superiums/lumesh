// 主要框架
use ratatui::{prelude::*, widgets::*};

struct App {
    // 状态管理
    current_input: String,
    input_history: Vec<String>,
    output_history: Vec<(String, String)>,
    active_panel: PanelType,
    input_height: u16,
    // ...
}

enum PanelType {
    History,
    Favorites,
    Help,
    Settings,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;
    let app = App::new();
    run_app(&mut terminal, app)?;
    restore_terminal(&mut terminal)?;
    Ok(())
}

fn run_app(terminal: &mut Terminal, mut app: App) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if handle_events(&mut app)? {
            break;
        }
    }
    Ok(())
}

fn ui(frame: &mut Frame, app: &App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                // 顶部命令显示
            Constraint::Min(2),                   // 主区域
            Constraint::Length(app.input_height), // 底部输入区
        ]);

    let [top_panel, main_panel, bottom_panel] = main_layout.areas(frame.size());

    // 顶部面板 - 显示当前/历史命令
    frame.render_widget(build_top_panel(app), top_panel);

    // 主面板布局
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)]);

    let [side_panel, output_panel] = main_layout.areas(main_panel);

    // 左侧功能面板
    frame.render_widget(build_side_panel(app), side_panel);

    // 主输出区域
    frame.render_widget(build_output_panel(app), output_panel);

    // 底部输入区
    frame.render_widget(build_input_panel(app), bottom_panel);
}

// -------功能实现关键点-----
// 输入区扩展：
fn adjust_input_height(app: &mut App) {
    let line_count = app.current_input.lines().count();
    app.input_height = (line_count as u16).max(2);
}
// 语法高亮集成：
fn highlight_code(code: &str) -> Vec<Span> {
    // 复用现有高亮逻辑或使用syntect
    let highlighted = syntax_highlight(code);
    // 转换为ratatui的Span
    parse_ansi_to_spans(&highlighted)
}
// 历史命令导航：
fn navigate_history(app: &mut App, direction: Direction) {
    match direction {
        Direction::Up => {
            if app.history_index < app.input_history.len() {
                app.history_index += 1;
                app.current_input = app.input_history[app.history_index].clone();
            }
        }
        Direction::Down => {
            if app.history_index > 0 {
                app.history_index -= 1;
                app.current_input = app.input_history[app.history_index].clone();
            }
        }
    }
}
