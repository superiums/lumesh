use crate::{Environment, Expression, Int, LmError};
use common_macros::hash_map;
use std::io::Write;

pub fn get() -> Expression {
    (hash_map! {
        String::from("width") => Expression::builtin("width", width, "get the width of the console"),
        String::from("height") => Expression::builtin("height", height, "get the height of the console"),
        String::from("write") => Expression::builtin("write", write, "write text to a specific position in the console"),
        String::from("title") => Expression::builtin("title", title, "set the title of the console"),
        String::from("clear") => Expression::builtin("clear", clear, "clear the console"),
        String::from("flush") => Expression::builtin("flush", |_, _| {
            std::io::stdout().flush().unwrap();
            Ok(Expression::None)
        }, "flush the console"),
        String::from("mode") => Expression::from(hash_map! {
            String::from("raw") => Expression::builtin("raw", |_, _| {
                match crossterm::terminal::enable_raw_mode() {
                    Ok(_) => Ok(Expression::None),
                    Err(_) => Err(LmError::CustomError("could not enable raw mode".to_string()))
                }
            }, "enable raw mode"),
            String::from("cooked") => Expression::builtin("cooked", |_, _| {
                match crossterm::terminal::disable_raw_mode() {
                    Ok(_) => Ok(Expression::None),
                    Err(_) => Err(LmError::CustomError("could not disable raw mode".to_string()))
                }
            }, "disable raw mode"),
            String::from("alternate") => Expression::builtin("alternate", |_, _| {
                print!("\x1b[?1049h");
                Ok(Expression::None)
            }, "enable alternate screen"),
            String::from("normal") => Expression::builtin("normal", |_, _| {
                print!("\x1b[?1049l");
                Ok(Expression::None)
            }, "disable alternate screen"),
        }),
        String::from("cursor") => Expression::from(hash_map! {
            String::from("to") => Expression::builtin("to", |args, env| {
                super::check_exact_args_len("to", args, 2)?;
                let x = args[0].eval(env)?;
                let y = args[1].eval(env)?;
                match (x, y) {
                    (Expression::Integer(x), Expression::Integer(y)) => {
                        print!("\x1b[{row};{column}f", column = x, row = y);
                    }
                    (x, y) => return Err(LmError::CustomError(format!("expected first two arguments to be integers, but got: `{:?}`, `{:?}`", x, y)))
                }
                Ok(Expression::None)
            }, "move the cursor to a specific position in the console"),

            String::from("up") => Expression::builtin("up", |args, env| {
                super::check_exact_args_len("up", args, 1)?;
                let y = args[0].eval(env)?;
                if let Expression::Integer(y) = &y {
                    print!("\x1b[{y}A", y = y);
                } else {
                    return Err(LmError::CustomError(format!("expected first argument to be an integer, but got: `{:?}`", y)));
                }
                Ok(Expression::None)
            }, "move the cursor up a specific number of lines"),

            String::from("down") => Expression::builtin("down", |args, env| {
                super::check_exact_args_len("down", args, 1)?;
                let y = args[0].eval(env)?;
                if let Expression::Integer(y) = &y {
                    print!("\x1b[{y}B", y = y);
                } else {
                    return Err(LmError::CustomError(format!("expected first argument to be an integer, but got: `{:?}`", y)));
                }
                Ok(Expression::None)
            }, "move the cursor down a specific number of lines"),

            String::from("left") => Expression::builtin("left", |args, env| {
                super::check_exact_args_len("left", args, 1)?;
                let x = args[0].eval(env)?;
                if let Expression::Integer(x) = &x {
                    print!("\x1b[{x}D", x = x);
                } else {
                    return Err(LmError::CustomError(format!("expected first argument to be an integer, but got: `{:?}`", x)));
                }
                Ok(Expression::None)
            }, "move the cursor left a specific number of columns"),

            String::from("right") => Expression::builtin("right", |args, env| {
                super::check_exact_args_len("right", args, 1)?;
                let x = args[0].eval(env)?;
                if let Expression::Integer(x) = &x {
                    print!("\x1b[{x}C", x = x);
                } else {
                    return Err(LmError::CustomError(format!("expected first argument to be an integer, but got: `{:?}`", x)));
                }
                Ok(Expression::None)
            }, "move the cursor right a specific number of columns"),

            String::from("save") => Expression::builtin("save", |_, _| {
                print!("\x1b[s");
                Ok(Expression::None)
            }, "save the current cursor position"),

            String::from("restore") => Expression::builtin("restore", |_, _| {
                print!("\x1b[u");
                Ok(Expression::None)
            }, "restore the last saved cursor position"),

            String::from("hide") => Expression::builtin("hide", |_, _| {
                print!("\x1b[?25l");
                Ok(Expression::None)
            }, "hide the cursor"),

            String::from("show") => Expression::builtin("show", |_, _| {
                print!("\x1b[?25h");
                Ok(Expression::None)
            }, "show the cursor"),
        }),

        String::from("keyboard") => Expression::from(hash_map! {
            String::from("read_line") => Expression::builtin("read_line", |_, _| {
                let mut buffer = String::new();
                std::io::stdin().read_line(&mut buffer).unwrap();
                Ok(Expression::String(buffer))
            }, "read a line from the keyboard"),
            String::from("read_password") => Expression::builtin("read_password", |_, _| {
                let password = rpassword::read_password().unwrap();
                Ok(Expression::String(password))
            }, "read a password from the keyboard"),
            String::from("read_key") => Expression::builtin("read_key", |_, _| {
                let key = crossterm::event::read().unwrap();
                // Get the key as a string.
                let key = match key {
                    crossterm::event::Event::Key(key) => key,
                    _ => return Ok(Expression::None)
                };
                let code = key.code;
                use crossterm::event::KeyCode::*;
                Ok(match code {
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
                    F(i) => Expression::String(format!("\x1b[{}~", i)),
                    Null => Expression::String("\x00".to_string()),
                    BackTab => Expression::String("\x1b[Z".to_string()),
                    _ => Expression::None
                })
            }, "read a key from the keyboard"),
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
    })
    .into()
}

fn width(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    Ok(match crossterm::terminal::size() {
        Ok((w, _)) => (w as Int).into(),
        _ => Expression::None,
    })
}

fn height(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    Ok(match crossterm::terminal::size() {
        Ok((_, h)) => (h as Int).into(),
        _ => Expression::None,
    })
}

fn write(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("write", args, 3)?;
    match (args[0].eval(env)?, args[1].eval(env)?, args[2].eval(env)?) {
        (Expression::Integer(x), Expression::Integer(y), content) => {
            let content = content.to_string();
            for (y_offset, line) in content.lines().enumerate() {
                print!(
                    "\x1b[s\x1b[{row};{column}H\x1b[{row};{column}f{content}\x1b[u",
                    column = x,
                    row = y + y_offset as Int,
                    content = line
                );
            }
        }
        (x, y, _) => {
            return Err(LmError::CustomError(format!(
                "expected first two arguments to be integers, but got: `{:?}`, `{:?}`",
                x, y
            )));
        }
    }
    Ok(Expression::None)
}

fn title(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("title", args, 1)?;
    print!("\x1b]2;{}\x1b[0m", args[0].eval(env)?);
    Ok(Expression::None)
}

fn clear(args: &Vec<Expression>, _env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("clear", args, 1)?;
    print!("\x1b[2J\x1b[H");
    Ok(Expression::None)
}
