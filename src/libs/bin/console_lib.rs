use common_macros::hash_map;

use crate::{Environment, Expression};
use std::collections::BTreeMap;

use crate::libs::BuiltinInfo;
use crate::libs::helper::{check_args_len, check_exact_args_len};
use crate::libs::lazy_module::LazyModule;
use crate::{Int, RuntimeError, reg_info, reg_lazy};
use std::io::Write;

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
        write => "write text to a specific position in the console", "<x> <y> <text>"
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
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    crossterm::terminal::size()
        .map(|(w, _)| Expression::Integer(w as Int))
        .or(Ok(Expression::None))
}

fn height(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    crossterm::terminal::size()
        .map(|(_, h)| Expression::Integer(h as Int))
        .or(Ok(Expression::None))
}
// Text Output Functions
fn write(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("write", args, 3, ctx)?;

    let x = &args[0];
    let y = &args[1];
    let content = &args[2];

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
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("title", args, 1, ctx)?;
    let title = &args[0];
    print!("\x1b]2;{title}\x07");
    Ok(Expression::None)
}

fn clear(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    print!("\x1b[2J\x1b[H");
    Ok(Expression::None)
}

fn flush(
    _args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    std::io::stdout()
        .flush()
        .map_err(|e| RuntimeError::common(format!("Flush failed: {e}").into(), ctx.clone(), 0))?;
    Ok(Expression::None)
}
// Console Mode Functions
fn mode_raw(
    _args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    crossterm::terminal::enable_raw_mode()
        .map(|_| Expression::None)
        .map_err(|_| RuntimeError::common("Failed to enable raw mode".into(), ctx.clone(), 0))
}

fn mode_normal(
    _args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    crossterm::terminal::disable_raw_mode()
        .map(|_| Expression::None)
        .map_err(|_| RuntimeError::common("Failed to disable raw mode".into(), ctx.clone(), 0))
}

fn screen_alternate(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    print!("\x1b[?1049h");
    Ok(Expression::None)
}

fn screen_normal(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    print!("\x1b[?1049l");
    Ok(Expression::None)
}
// Cursor Control Functions
fn cursor_to(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cursor_to", args, 2, ctx)?;

    let x = &args[0];
    let y = &args[1];

    match (x, y) {
        (Expression::Integer(x), Expression::Integer(y)) => {
            print!("\x1b[{y};{x}H");
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

macro_rules! cursor_move_fn {
    ($name:ident, $code:literal, $doc:literal) => {
        fn $name(
            args: &[Expression],
            _env: &mut Environment,
            ctx: &Expression,
        ) -> Result<Expression, RuntimeError> {
            check_exact_args_len(stringify!($name), args, 1, ctx)?;

            if let Expression::Integer(n) = args[0] {
                print!(concat!("\x1b[", $code, "{}"), n);
                Ok(Expression::None)
            } else {
                Err(RuntimeError::common(
                    format!("Expected integer for movement amount, got {:?}", args[0]).into(),
                    ctx.clone(),
                    0,
                ))
            }
        }
    };
}

cursor_move_fn!(cursor_up, "A", "Move cursor up");
cursor_move_fn!(cursor_down, "B", "Move cursor down");
cursor_move_fn!(cursor_left, "D", "Move cursor left");
cursor_move_fn!(cursor_right, "C", "Move cursor right");

fn cursor_save(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    print!("\x1b[s");
    Ok(Expression::None)
}

fn cursor_restore(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    print!("\x1b[u");
    Ok(Expression::None)
}

fn cursor_hide(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    print!("\x1b[?25l");
    Ok(Expression::None)
}

fn cursor_show(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    print!("\x1b[?25h");
    Ok(Expression::None)
}
// Input Functions
fn read_line(
    args: &[Expression],
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
            std::io::stdout().flush().map_err(|e| {
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

fn keys(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(Expression::from(hash_map! {
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
    }))
}
fn read_password(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("read_password", args, 0..1, ctx)?;
    let rst = if args.len() > 0 {
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
    _args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    crossterm::terminal::enable_raw_mode()
        .map_err(|_| RuntimeError::common("Failed to enable raw mode".into(), ctx.clone(), 0))?;

    let key = crossterm::event::read().map_err(|e| {
        RuntimeError::common(format!("Failed to read key: {e}").into(), ctx.clone(), 0)
    })?;

    crossterm::terminal::disable_raw_mode()
        .map_err(|_| RuntimeError::common("Failed to disable raw mode".into(), ctx.clone(), 0))?;

    match key {
        crossterm::event::Event::Key(event) => {
            let key_str = match event.code {
                crossterm::event::KeyCode::Enter => "\n".to_string(),
                crossterm::event::KeyCode::Backspace => "\x08".to_string(),
                crossterm::event::KeyCode::Delete => "\x7f".to_string(),
                crossterm::event::KeyCode::Left => "\x1b[D".to_string(),
                crossterm::event::KeyCode::Right => "\x1b[C".to_string(),
                crossterm::event::KeyCode::Up => "\x1b[A".to_string(),
                crossterm::event::KeyCode::Down => "\x1b[B".to_string(),
                crossterm::event::KeyCode::Home => "\x1b[H".to_string(),
                crossterm::event::KeyCode::End => "\x1b[F".to_string(),
                crossterm::event::KeyCode::PageUp => "\x1b[5~".to_string(),
                crossterm::event::KeyCode::PageDown => "\x1b[6~".to_string(),
                crossterm::event::KeyCode::Tab => "\t".to_string(),
                crossterm::event::KeyCode::Esc => "\x1b".to_string(),
                crossterm::event::KeyCode::Insert => "\x1b[2~".to_string(),
                crossterm::event::KeyCode::F(1) => "\x1b[11~".to_string(),
                crossterm::event::KeyCode::F(2) => "\x1b[12~".to_string(),
                crossterm::event::KeyCode::F(3) => "\x1b[13~".to_string(),
                crossterm::event::KeyCode::F(4) => "\x1b[14~".to_string(),
                crossterm::event::KeyCode::F(5) => "\x1b[15~".to_string(),
                crossterm::event::KeyCode::F(6) => "\x1b[17~".to_string(),
                crossterm::event::KeyCode::F(7) => "\x1b[18~".to_string(),
                crossterm::event::KeyCode::F(8) => "\x1b[19~".to_string(),
                crossterm::event::KeyCode::F(9) => "\x1b[20~".to_string(),
                crossterm::event::KeyCode::F(10) => "\x1b[21~".to_string(),
                crossterm::event::KeyCode::F(11) => "\x1b[23~".to_string(),
                crossterm::event::KeyCode::F(12) => "\x1b[24~".to_string(),
                crossterm::event::KeyCode::Null => "\x00".to_string(),
                crossterm::event::KeyCode::BackTab => "\x1b[Z".to_string(),
                crossterm::event::KeyCode::Char(c) => c.to_string(),
                _ => format!("{:?}", event.code),
            };
            Ok(Expression::String(key_str))
        }
        _ => Err(RuntimeError::common(
            "Expected key event".into(),
            ctx.clone(),
            0,
        )),
    }
}
