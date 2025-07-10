use crate::{Environment, Expression, Int, LmError};
use common_macros::hash_map;
use std::io::Write;

pub fn get() -> Expression {
    (hash_map! {

            // Console information
            String::from("width") => Expression::builtin("width", width, "get the width of the console", ""),
            String::from("height") => Expression::builtin("height", height, "get the height of the console", ""),

            // Output control
            String::from("write") => Expression::builtin("write", write, "write text to a specific position in the console", "<x> <y> <text>"),
            String::from("title") => Expression::builtin("title", title, "set the title of the console", "<string>"),
            String::from("clear") => Expression::builtin("clear", clear, "clear the console", ""),
            String::from("flush") => Expression::builtin("flush", flush, "flush the console", ""),

            // Mode control
            String::from("mode_raw") => Expression::builtin("mode_raw", enable_raw_mode, "enable raw mode", ""),
            String::from("mode_normal") => Expression::builtin("mode_normal", disable_raw_mode, "disable raw mode", ""),
            String::from("screen_alternate") => Expression::builtin("screen_alternate", enable_alternate_screen, "enable alternate screen", ""),
            String::from("screen_normal") => Expression::builtin("screen_normal", disable_alternate_screen, "disable alternate screen", ""),

            // Cursor control
            String::from("cursor_to") => Expression::builtin("cursor_to", cursor_to, "move the cursor to a specific position", "<x> <y>"),
            String::from("cursor_up") => Expression::builtin("cursor_up", cursor_up, "move the cursor up", "<n>"),
            String::from("cursor_down") => Expression::builtin("cursor_down", cursor_down, "move the cursor down", "<n>"),
            String::from("cursor_left") => Expression::builtin("cursor_left", cursor_left, "move the cursor left", "<n>"),
            String::from("cursor_right") => Expression::builtin("cursor_right", cursor_right, "move the cursor right", "<n>"),
            String::from("cursor_save") => Expression::builtin("cursor_save", cursor_save, "save cursor position", ""),
            String::from("cursor_restore") => Expression::builtin("cursor_restore", cursor_restore, "restore cursor position", ""),
            String::from("cursor_hide") => Expression::builtin("cursor_hide", cursor_hide, "hide cursor", ""),
            String::from("cursor_show") => Expression::builtin("cursor_show", cursor_show, "show cursor", ""),

            // Input control
            String::from("read_line") => Expression::builtin("read_line", keyboard_read_line, "read line from keyboard", "[prompt]"),
            String::from("read_password") => Expression::builtin("read_password", keyboard_read_password, "read password from keyboard", "[prompt]"),
            String::from("read_key") => Expression::builtin("read_key", keyboard_read_key, "read key from keyboard", ""),


            String::from("keys") => Expression::from(hash_map! {
                String::from("enter") => Expression::String("\n".to_string()),
                String::from("backspace") => Expression::String("\x08".to_string()),
                String::from("delete") => Expression::String("\x7f".to_string()),
                String::from("left") => Expression::String("\x1b[D".to_string()),
                String::from("right") => Expression::String("\x1b[C".to_string()),
                String::from("up") => Expression::String("\x1b[A".to_string()),
                String::from("down") => Expression::String("\x1b[B".to_string()),
                String::from("home") => Expression::String("\x1b[H".to_string()),
                String::from("end") => Expression::String("\x1b[F".to_string()),
                String::from("page_up") => Expression::String("\x1b[5~".to_string()),
                String::from("page_down") => Expression::String("\x1b[6~".to_string()),
                String::from("tab") => Expression::String("\t".to_string()),
                String::from("esc") => Expression::String("\x1b".to_string()),
                String::from("insert") => Expression::String("\x1b[2~".to_string()),
                String::from("f1") => Expression::String("\x1b[11~".to_string()),
                String::from("f2") => Expression::String("\x1b[12~".to_string()),
                String::from("f3") => Expression::String("\x1b[13~".to_string()),
                String::from("f4") => Expression::String("\x1b[14~".to_string()),
                String::from("f5") => Expression::String("\x1b[15~".to_string()),
                String::from("f6") => Expression::String("\x1b[17~".to_string()),
                String::from("f7") => Expression::String("\x1b[18~".to_string()),
                String::from("f8") => Expression::String("\x1b[19~".to_string()),
                String::from("f9") => Expression::String("\x1b[20~".to_string()),
                String::from("f10") => Expression::String("\x1b[21~".to_string()),
                String::from("f11") => Expression::String("\x1b[23~".to_string()),
                String::from("f12") => Expression::String("\x1b[24~".to_string()),
                String::from("null") => Expression::String("\x00".to_string()),
                String::from("back_tab") => Expression::String("\x1b[Z".to_string()),
            })
    })
    .into()
}

// 控制台尺寸函数
fn width(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    crossterm::terminal::size()
        .map(|(w, _)| Expression::Integer(w as Int))
        .or(Ok(Expression::None))
}

fn height(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    crossterm::terminal::size()
        .map(|(_, h)| Expression::Integer(h as Int))
        .or(Ok(Expression::None))
}

// 文本输出函数
fn write(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("write", args, 3)?;

    let x = args[0].eval(env)?;
    let y = args[1].eval(env)?;
    let content = args[2].eval(env)?;

    match (x, y) {
        (Expression::Integer(x), Expression::Integer(y)) => {
            let content_str = content.to_string();
            for (y_offset, line) in content_str.lines().enumerate() {
                print!(
                    "\x1b[s\x1b[{row};{column}H{line}\x1b[u",
                    column = x,
                    row = y + y_offset as Int,
                    line = line
                );
            }
            Ok(Expression::None)
        }
        (m, n) => Err(LmError::CustomError(format!(
            "Expected integers for position, got ({} {:?}, {} {:?})",
            m.type_name(),
            m,
            n.type_name(),
            n
        ))),
    }
}

fn title(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("title", args, 1)?;
    let title = args[0].eval(env)?.to_string();
    print!("\x1b]2;{title}\x07");
    Ok(Expression::None)
}

fn clear(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    print!("\x1b[2J\x1b[H");
    Ok(Expression::None)
}

fn flush(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    std::io::stdout()
        .flush()
        .map_err(|e| LmError::CustomError(format!("Flush failed: {e}")))?;
    Ok(Expression::None)
}

// 控制台模式函数
fn enable_raw_mode(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    crossterm::terminal::enable_raw_mode()
        .map(|_| Expression::None)
        .map_err(|_| LmError::CustomError("Failed to enable raw mode".into()))
}

fn disable_raw_mode(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    crossterm::terminal::disable_raw_mode()
        .map(|_| Expression::None)
        .map_err(|_| LmError::CustomError("Failed to disable raw mode".into()))
}

fn enable_alternate_screen(
    _: &Vec<Expression>,
    _: &mut Environment,
) -> Result<Expression, LmError> {
    print!("\x1b[?1049h");
    Ok(Expression::None)
}

fn disable_alternate_screen(
    _: &Vec<Expression>,
    _: &mut Environment,
) -> Result<Expression, LmError> {
    print!("\x1b[?1049l");
    Ok(Expression::None)
}

// 光标控制函数
fn cursor_to(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("cursor_to", args, 2)?;

    let x = args[0].eval(env)?;
    let y = args[1].eval(env)?;

    match (x, y) {
        (Expression::Integer(x), Expression::Integer(y)) => {
            print!("\x1b[{y};{x}H");
            Ok(Expression::None)
        }
        (m, n) => Err(LmError::CustomError(format!(
            "Expected integers for position, got ({} {:?}, {} {:?})",
            m.type_name(),
            m,
            n.type_name(),
            n
        ))),
    }
}

macro_rules! cursor_move_fn {
    ($name:ident, $code:literal, $doc:literal) => {
        fn $name(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
            super::check_exact_args_len(stringify!($name), args, 1)?;

            if let Expression::Integer(n) = args[0].eval(env)? {
                print!(concat!("\x1b[", $code, "{}"), n);
                Ok(Expression::None)
            } else {
                Err(LmError::CustomError(format!(
                    "Expected integer for movement amount, got {:?}",
                    args[0]
                )))
            }
        }
    };
}

cursor_move_fn!(cursor_up, "A", "Move cursor up");
cursor_move_fn!(cursor_down, "B", "Move cursor down");
cursor_move_fn!(cursor_left, "D", "Move cursor left");
cursor_move_fn!(cursor_right, "C", "Move cursor right");

fn cursor_save(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    print!("\x1b[s");
    Ok(Expression::None)
}

fn cursor_restore(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    print!("\x1b[u");
    Ok(Expression::None)
}

fn cursor_hide(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    print!("\x1b[?25l");
    Ok(Expression::None)
}

fn cursor_show(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    print!("\x1b[?25h");
    Ok(Expression::None)
}

// 键盘输入函数
fn keyboard_read_line(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    let mut buffer = String::new();
    std::io::stdin()
        .read_line(&mut buffer)
        .map_err(|e| LmError::CustomError(format!("Read failed: {e}")))?;
    Ok(Expression::String(buffer))
}

fn keyboard_read_password(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    rpassword::read_password()
        .map(Expression::String)
        .map_err(|e| LmError::CustomError(format!("Password read failed: {e}")))
}

fn keyboard_read_key(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    let event = crossterm::event::read()
        .map_err(|e| LmError::CustomError(format!("Key read failed: {e}")))?;

    if let crossterm::event::Event::Key(key) = event {
        use crossterm::event::KeyCode::*;
        Ok(match key.code {
            Char(c) => Expression::String(c.to_string()),
            Enter => Expression::String("\n".to_string()),
            Backspace => Expression::String("\x08".to_string()),
            Delete => Expression::String("\x7f".to_string()),
            Left => Expression::String("\x1b[D".to_string()),
            Right => Expression::String("\x1b[C".to_string()),
            Up => Expression::String("\x1b[A".to_string()),
            Down => Expression::String("\x1b[B".to_string()),
            Home => Expression::String("\x1b[H".to_string()),
            End => Expression::String("\x1b[F".to_string()),
            PageUp => Expression::String("\x1b[5~".to_string()),
            PageDown => Expression::String("\x1b[6~".to_string()),
            Tab => Expression::String("\t".to_string()),
            Esc => Expression::String("\x1b".to_string()),
            Insert => Expression::String("\x1b[2~".to_string()),
            F(i) => Expression::String(format!("\x1b[{i}~")),
            Null => Expression::String("\x00".to_string()),
            BackTab => Expression::String("\x1b[Z".to_string()),
            _ => Expression::None,
        })
    } else {
        Ok(Expression::None)
    }
}
