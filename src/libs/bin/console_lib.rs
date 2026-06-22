use crate::{Environment, Expression};
use std::collections::BTreeMap;

use crate::libs::BuiltinInfo;
use crate::libs::helper::{check_args_len, check_exact_args_len};
use crate::libs::lazy_module::LazyModule;
use crate::{Int, RuntimeError, reg_info, reg_lazy};

use crossterm::cursor::{
    Hide, MoveDown, MoveLeft, MoveRight, MoveTo, MoveUp, RestorePosition, SavePosition, Show,
};
use crossterm::event::{Event, KeyCode, read};
use crossterm::style::Print;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
    disable_raw_mode, enable_raw_mode, size,
};
use crossterm::{execute, queue};
use std::io::{stdout, Write};

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // Console information
        width, height,
        // Output control
        write, title, clear, flush,
        // Mode control
        mode_raw, mode_normal, screen_alternate, screen_normal,
        // Cursor control
        cursor_to, cursor_up, cursor_down, cursor_left, cursor_right, cursor_save, cursor_restore, cursor_hide, cursor_show,
        // Input control
        read_line, read_password, read_key,
        keys
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        // Console information
        width => "get the width of the console", ""
        height => "get the height of the console", ""

        // Output control
        write => "write text to a specific position in the console", "<text> <x> <y>"
        title => "set the title of the console", "<string>"
        clear => "clear the console", ""
        flush => "flush the console", ""

        // Mode control
        mode_raw => "enable raw mode", ""
        mode_normal => "disable raw mode", ""
        screen_alternate => "enable alternate screen", ""
        screen_normal => "disable alternate screen", ""

        // Cursor control
        cursor_to => "move the cursor to a specific position", "<x> <y>"
        cursor_up => "move the cursor up", "<n>"
        cursor_down => "move the cursor down", "<n>"
        cursor_left => "move the cursor left", "<n>"
        cursor_right => "move the cursor right", "<n>"
        cursor_save => "save cursor position", ""
        cursor_restore => "restore cursor position", ""
        cursor_hide => "hide cursor", ""
        cursor_show => "show cursor", ""

        // Input control
        read_line => "read line from keyboard", "[prompt]"
        read_password => "read password from keyboard", "[prompt]"
        read_key => "read key from keyboard", ""


        keys => "list keys", ""
    })
}

// Console Information Functions
fn width(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    size()
        .map(|(w, _)| Expression::Integer(w as Int))
        .or(Ok(Expression::None))
}

fn height(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    size()
        .map(|(_, h)| Expression::Integer(h as Int))
        .or(Ok(Expression::None))
}
// Text Output Functions
fn write(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("write", &args, 3, ctx)?;

    let x = &args[1];
    let y = &args[2];

    match (x, y) {
        (Expression::Integer(x), Expression::Integer(y)) => {
            let content_str = args[0].to_string();
            let mut out = stdout();
            for (y_offset, line) in content_str.lines().enumerate() {
                queue!(
                    out,
                    SavePosition,
                    MoveTo(*x as u16, (*y + y_offset as Int) as u16),
                    Print(line),
                    RestorePosition,
                )
                .map_err(|e| {
                    RuntimeError::common(format!("Write failed: {e}").into(), ctx.clone(), 0)
                })?;
            }
            out.flush()
                .map_err(|e| RuntimeError::common(format!("Flush failed: {e}").into(), ctx.clone(), 0))?;
            Ok(Expression::None)
        }
        (m, n) => Err(RuntimeError::common(
            format!(
                "Expected integers for position, got ({} {:?}, {} {:?})",
                m.type_name(),
                m,
                n.type_name(),
                n
            )
            .into(),
            ctx.clone(),
            0,
        )),
    }
}

fn title(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("title", &args, 1, ctx)?;
    execute!(stdout(), SetTitle(args[0].to_string()))
        .map_err(|e| RuntimeError::common(format!("Failed to set title: {e}").into(), ctx.clone(), 0))?;
    Ok(Expression::None)
}

fn clear(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))
        .map_err(|_| RuntimeError::common("Clear failed".into(), _ctx.clone(), 0))?;
    Ok(Expression::None)
}

fn flush(
    _args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    stdout()
        .flush()
        .map_err(|e| RuntimeError::common(format!("Flush failed: {e}").into(), ctx.clone(), 0))?;
    Ok(Expression::None)
}
// Console Mode Functions
fn mode_raw(
    _args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    enable_raw_mode()
        .map(|_| Expression::None)
        .map_err(|_| RuntimeError::common("Failed to enable raw mode".into(), ctx.clone(), 0))
}

fn mode_normal(
    _args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    disable_raw_mode()
        .map(|_| Expression::None)
        .map_err(|_| RuntimeError::common("Failed to disable raw mode".into(), ctx.clone(), 0))
}

fn screen_alternate(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    execute!(stdout(), EnterAlternateScreen)
        .map_err(|_| RuntimeError::common("Failed to enter alternate screen".into(), _ctx.clone(), 0))?;
    Ok(Expression::None)
}

fn screen_normal(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    execute!(stdout(), LeaveAlternateScreen)
        .map_err(|_| RuntimeError::common("Failed to leave alternate screen".into(), _ctx.clone(), 0))?;
    Ok(Expression::None)
}
// Cursor Control Functions
fn cursor_to(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cursor_to", &args, 2, ctx)?;

    match (&args[0], &args[1]) {
        (Expression::Integer(x), Expression::Integer(y)) => {
            execute!(stdout(), MoveTo(*x as u16, *y as u16))
                .map_err(|e| RuntimeError::common(format!("Failed to move cursor: {e}").into(), ctx.clone(), 0))?;
            Ok(Expression::None)
        }
        (m, n) => Err(RuntimeError::common(
            format!(
                "Expected integers for position, got ({} {:?}, {} {:?})",
                m.type_name(),
                m,
                n.type_name(),
                n
            )
            .into(),
            ctx.clone(),
            0,
        )),
    }
}

fn cursor_up(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cursor_up", &args, 1, ctx)?;
    if let Expression::Integer(n) = args[0] {
        execute!(stdout(), MoveUp(n as u16))
            .map_err(|e| RuntimeError::common(format!("Failed to move cursor: {e}").into(), ctx.clone(), 0))?;
        Ok(Expression::None)
    } else {
        Err(RuntimeError::common(
            format!("Expected integer for movement amount, got {:?}", args[0]).into(),
            ctx.clone(),
            0,
        ))
    }
}

fn cursor_down(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cursor_down", &args, 1, ctx)?;
    if let Expression::Integer(n) = args[0] {
        execute!(stdout(), MoveDown(n as u16))
            .map_err(|e| RuntimeError::common(format!("Failed to move cursor: {e}").into(), ctx.clone(), 0))?;
        Ok(Expression::None)
    } else {
        Err(RuntimeError::common(
            format!("Expected integer for movement amount, got {:?}", args[0]).into(),
            ctx.clone(),
            0,
        ))
    }
}

fn cursor_left(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cursor_left", &args, 1, ctx)?;
    if let Expression::Integer(n) = args[0] {
        execute!(stdout(), MoveLeft(n as u16))
            .map_err(|e| RuntimeError::common(format!("Failed to move cursor: {e}").into(), ctx.clone(), 0))?;
        Ok(Expression::None)
    } else {
        Err(RuntimeError::common(
            format!("Expected integer for movement amount, got {:?}", args[0]).into(),
            ctx.clone(),
            0,
        ))
    }
}

fn cursor_right(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cursor_right", &args, 1, ctx)?;
    if let Expression::Integer(n) = args[0] {
        execute!(stdout(), MoveRight(n as u16))
            .map_err(|e| RuntimeError::common(format!("Failed to move cursor: {e}").into(), ctx.clone(), 0))?;
        Ok(Expression::None)
    } else {
        Err(RuntimeError::common(
            format!("Expected integer for movement amount, got {:?}", args[0]).into(),
            ctx.clone(),
            0,
        ))
    }
}

fn cursor_save(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    execute!(stdout(), SavePosition)
        .map_err(|_| RuntimeError::common("Failed to save cursor position".into(), _ctx.clone(), 0))?;
    Ok(Expression::None)
}

fn cursor_restore(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    execute!(stdout(), RestorePosition)
        .map_err(|_| RuntimeError::common("Failed to restore cursor position".into(), _ctx.clone(), 0))?;
    Ok(Expression::None)
}

fn cursor_hide(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    execute!(stdout(), Hide)
        .map_err(|_| RuntimeError::common("Failed to hide cursor".into(), _ctx.clone(), 0))?;
    Ok(Expression::None)
}

fn cursor_show(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    execute!(stdout(), Show)
        .map_err(|_| RuntimeError::common("Failed to show cursor".into(), _ctx.clone(), 0))?;
    Ok(Expression::None)
}
// Input Functions
fn read_line(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    match args.len() {
        0 => {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).map_err(|e| {
                RuntimeError::common(format!("Failed to read line: {e}").into(), ctx.clone(), 0)
            })?;
            Ok(Expression::String(input.trim_end().to_string()))
        }
        1 => {
            let prompt = args[0].to_string();
            print!("{prompt}");
            stdout().flush().map_err(|e| {
                RuntimeError::common(format!("Flush failed: {e}").into(), ctx.clone(), 0)
            })?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).map_err(|e| {
                RuntimeError::common(format!("Failed to read line: {e}").into(), ctx.clone(), 0)
            })?;
            Ok(Expression::String(input.trim_end().to_string()))
        }
        _ => Err(RuntimeError::common(
            "read_line expects 0 or 1 arguments".into(),
            ctx.clone(),
            0,
        )),
    }
}

// Key mapping constants shared between read_key and keys
const SPECIAL_KEY_MAPPINGS: &[(&str, KeyCode, &str)] = &[
    ("enter", KeyCode::Enter, "\n"),
    ("backspace", KeyCode::Backspace, "\x08"),
    ("delete", KeyCode::Delete, "\x7f"),
    ("left", KeyCode::Left, "\x1b[D"),
    ("right", KeyCode::Right, "\x1b[C"),
    ("up", KeyCode::Up, "\x1b[A"),
    ("down", KeyCode::Down, "\x1b[B"),
    ("home", KeyCode::Home, "\x1b[H"),
    ("end", KeyCode::End, "\x1b[F"),
    ("page_up", KeyCode::PageUp, "\x1b[5~"),
    ("page_down", KeyCode::PageDown, "\x1b[6~"),
    ("tab", KeyCode::Tab, "\t"),
    ("esc", KeyCode::Esc, "\x1b"),
    ("insert", KeyCode::Insert, "\x1b[2~"),
    ("f1", KeyCode::F(1), "\x1b[11~"),
    ("f2", KeyCode::F(2), "\x1b[12~"),
    ("f3", KeyCode::F(3), "\x1b[13~"),
    ("f4", KeyCode::F(4), "\x1b[14~"),
    ("f5", KeyCode::F(5), "\x1b[15~"),
    ("f6", KeyCode::F(6), "\x1b[17~"),
    ("f7", KeyCode::F(7), "\x1b[18~"),
    ("f8", KeyCode::F(8), "\x1b[19~"),
    ("f9", KeyCode::F(9), "\x1b[20~"),
    ("f10", KeyCode::F(10), "\x1b[21~"),
    ("f11", KeyCode::F(11), "\x1b[23~"),
    ("f12", KeyCode::F(12), "\x1b[24~"),
    ("null", KeyCode::Null, "\x00"),
    ("back_tab", KeyCode::BackTab, "\x1b[Z"),
];

fn key_code_str(code: KeyCode) -> Option<&'static str> {
    SPECIAL_KEY_MAPPINGS
        .iter()
        .find(|(_, k, _)| *k == code)
        .map(|(_, _, s)| *s)
}

fn keys(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(Expression::from(
        SPECIAL_KEY_MAPPINGS
            .iter()
            .map(|(name, _, s)| (name.to_string(), Expression::String(s.to_string())))
            .collect::<BTreeMap<_, _>>(),
    ))
}

fn read_password(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("read_password", &args, 0..1, ctx)?;
    let rst = if !args.is_empty() {
        rpassword::prompt_password(args[0].to_string())
    } else {
        rpassword::prompt_password("")
    };
    let r = rst.map_err(|e| {
        RuntimeError::common(
            format!("Failed to read password: {e}").into(),
            ctx.clone(),
            0,
        )
    })?;
    Ok(Expression::String(r))
}

fn read_key(
    _args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    enable_raw_mode()
        .map_err(|_| RuntimeError::common("Failed to enable raw mode".into(), ctx.clone(), 0))?;

    let key = read().map_err(|e| {
        RuntimeError::common(format!("Failed to read key: {e}").into(), ctx.clone(), 0)
    });

    disable_raw_mode()
        .map_err(|_| RuntimeError::common("Failed to disable raw mode".into(), ctx.clone(), 0))?;

    let key = key?;

    match key {
        Event::Key(event) => {
            let key_str = key_code_str(event.code)
                .map(|s| s.to_string())
                .unwrap_or_else(|| match event.code {
                    KeyCode::Char(c) => c.to_string(),
                    _ => format!("{:?}", event.code),
                });
            Ok(Expression::String(key_str))
        }
        _ => Err(RuntimeError::common(
            "Expected key event".into(),
            ctx.clone(),
            0,
        )),
    }
}
