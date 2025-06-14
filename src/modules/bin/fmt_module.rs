use crate::{Environment, Expression, LmError};
use common_macros::hash_map;

pub fn get() -> Expression {
    (hash_map! {
            // 排列
            String::from("pad_start") => Expression::builtin("pad_start", pad_start, "pad string to specified length at start", "<length> [pad_char] <string>"),
            String::from("pad_end") => Expression::builtin("pad_end", pad_end, "pad string to specified length at end", "<length> [pad_char] <string>"),
            String::from("center") => Expression::builtin("center", center, "center string by padding both ends", "<length> [pad_char] <string>"),
            String::from("wrap") => Expression::builtin("wrap", wrap, "wrap text to fit in specific number of columns", "<width> <string>"),
            String::from("format") => Expression::builtin("format", format, "format string using {} placeholders", "<format_string> <args>..."),

            // 样式
            String::from("strip") => Expression::builtin("strip", strip, "remove all ANSI escape codes from string", "<string>"),
            String::from("href") => Expression::builtin("href", href, "create terminal hyperlink", "<url> <text>"),
            String::from("bold") => Expression::builtin("bold", bold, "apply bold styling", "<string>"),
            String::from("faint") => Expression::builtin("faint", faint, "apply faint/dim styling", "<string>"),
            String::from("italics") => Expression::builtin("italics", italics, "apply italic styling", "<string>"),
            String::from("underline") => Expression::builtin("underline", underline, "apply underline styling", "<string>"),
            String::from("blink") => Expression::builtin("blink", blink, "apply blinking effect", "<string>"),
            String::from("invert") => Expression::builtin("invert", invert, "invert foreground/background colors", "<string>"),
            String::from("strike") => Expression::builtin("strike", strike, "apply strikethrough styling", "<string>"),

            // 标准颜色
            String::from("black") => Expression::builtin("black", black, "apply black foreground", "<string>"),
            String::from("red") => Expression::builtin("red", red, "apply red foreground", "<string>"),
            String::from("green") => Expression::builtin("green", green, "apply green foreground", "<string>"),
            String::from("yellow") => Expression::builtin("yellow", yellow, "apply yellow foreground", "<string>"),
            String::from("blue") => Expression::builtin("blue", blue, "apply blue foreground", "<string>"),
            String::from("magenta") => Expression::builtin("magenta", magenta, "apply magenta foreground", "<string>"),
            String::from("cyan") => Expression::builtin("cyan", cyan, "apply cyan foreground", "<string>"),
            String::from("white") => Expression::builtin("white", white, "apply white foreground", "<string>"),

            // 暗色
            String::from("dark_black") => Expression::builtin("dark_black", dark_black, "apply dark black foreground", "<string>"),
            String::from("dark_red") => Expression::builtin("dark_red", dark_red, "apply dark red foreground", "<string>"),
            String::from("dark_green") => Expression::builtin("dark_green", dark_green, "apply dark green foreground", "<string>"),
            String::from("dark_yellow") => Expression::builtin("dark_yellow", dark_yellow, "apply dark yellow foreground", "<string>"),
            String::from("dark_blue") => Expression::builtin("dark_blue", dark_blue, "apply dark blue foreground", "<string>"),
            String::from("dark_magenta") => Expression::builtin("dark_magenta", dark_magenta, "apply dark magenta foreground", "<string>"),
            String::from("dark_cyan") => Expression::builtin("dark_cyan", dark_cyan, "apply dark cyan foreground", "<string>"),
            String::from("dark_white") => Expression::builtin("dark_white", dark_white, "apply dark white foreground", "<string>")

        })
        .into()
}

// 提取的独立函数 (字符串参数作为最后一个参数)
fn pad_start(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("pad_start", args, 2..=3)?;
    let (length, pad_char) = match args.len() {
        2 => (args[0].clone(), " ".to_string()),
        3 => (args[0].clone(), args[1].clone().to_string()),
        _ => unreachable!(),
    };
    let s = args.last().unwrap().clone();

    let s_val = match s.eval(env)? {
        Expression::Symbol(x) | Expression::String(x) => x,
        _ => {
            return Err(LmError::CustomError(
                "pad_start requires a string as last argument".to_string(),
            ));
        }
    };

    let len = match length.eval(env)? {
        Expression::Integer(n) => n.max(0) as usize,
        _ => {
            return Err(LmError::CustomError(
                "pad_start requires an integer as length".to_string(),
            ));
        }
    };

    let pad_ch = pad_char.chars().next().unwrap_or(' ');
    pad_start_impl(len, pad_ch, s_val)
}

fn pad_end(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("pad_end", args, 2..=3)?;
    let (length, pad_char) = match args.len() {
        2 => (args[0].clone(), " ".to_string()),
        3 => (args[0].clone(), args[1].clone().to_string()),
        _ => unreachable!(),
    };
    let s = args.last().unwrap().clone();

    let s_val = match s.eval(env)? {
        Expression::Symbol(x) | Expression::String(x) => x,
        _ => {
            return Err(LmError::CustomError(
                "pad_end requires a string as last argument".to_string(),
            ));
        }
    };

    let len = match length.eval(env)? {
        Expression::Integer(n) => n.max(0) as usize,
        _ => {
            return Err(LmError::CustomError(
                "pad_end requires an integer as length".to_string(),
            ));
        }
    };

    let pad_ch = pad_char.chars().next().unwrap_or(' ');
    pad_end_impl(len, pad_ch, s_val)
}

fn center(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("center", args, 2..=3)?;
    let (length, pad_char) = match args.len() {
        2 => (args[0].clone(), " ".to_string()),
        3 => (args[0].clone(), args[1].clone().to_string()),
        _ => unreachable!(),
    };
    let s = args.last().unwrap().clone();

    let s_val = match s.eval(env)? {
        Expression::Symbol(x) | Expression::String(x) => x,
        _ => {
            return Err(LmError::CustomError(
                "center requires a string as last argument".to_string(),
            ));
        }
    };

    let len = match length.eval(env)? {
        Expression::Integer(n) => n.max(0) as usize,
        _ => {
            return Err(LmError::CustomError(
                "center requires an integer as length".to_string(),
            ));
        }
    };

    if s_val.len() >= len {
        return Ok(Expression::String(s_val));
    }

    let pad_ch = pad_char.chars().next().unwrap_or(' ');
    center_impl(len, pad_ch, s_val)
}

// 模板字符串作为第一个参数（特殊处理）
fn format(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.is_empty() {
        return Err(LmError::CustomError(
            "format requires at least a template string".to_string(),
        ));
    }

    let template = match args.first().unwrap().eval(env)? {
        Expression::Symbol(x) | Expression::String(x) => x,
        _ => {
            return Err(LmError::CustomError(
                "format requires string template as first argument".to_string(),
            ));
        }
    };

    let placeholders = template.matches("{}").count();
    if args.len() - 1 < placeholders {
        return Err(LmError::CustomError(format!(
            "format requires {} arguments for {} placeholders",
            placeholders, placeholders
        )));
    }

    let mut result = template.clone();
    for arg in args.iter().skip(1).take(placeholders) {
        let value = arg.eval(env)?;
        result = result.replacen("{}", &value.to_string(), 1);
    }

    Ok(Expression::String(result))
}

// 单参数函数（字符串作为最后一个参数）
fn strip(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("strip", args, 1)?;
    Ok(strip_ansi_escapes(args[0].eval(env)?.to_string().as_str()).into())
}

fn bold(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("bold", args, 1)?;
    Ok(format!("\x1b[1m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

// 其他样式函数采用相同模式...
fn faint(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("faint", args, 1)?;
    Ok(format!("\x1b[2m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn italics(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("italics", args, 1)?;
    Ok(format!("\x1b[3m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

// 颜色函数采用相同模式...
fn black(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("black", args, 1)?;
    Ok(format!("\x1b[90m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn dark_black(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_black", args, 1)?;
    Ok(format!("\x1b[30m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

// 其他颜色函数类似实现...

// 原始实现函数保持不变
fn pad_start_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, LmError> {
    if s.len() >= len {
        return Ok(Expression::String(s));
    }
    let pad_len = len - s.len();
    let padding: String = std::iter::repeat_n(pad_ch, pad_len).collect();
    Ok(Expression::String(format!("{}{}", padding, s)))
}

fn pad_end_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, LmError> {
    if s.len() >= len {
        return Ok(Expression::String(s));
    }
    let pad_len = len - s.len();
    let padding: String = std::iter::repeat_n(pad_ch, pad_len).collect();
    Ok(Expression::String(format!("{}{}", s, padding)))
}

fn center_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, LmError> {
    let total_pad = len - s.len();
    let left_pad = total_pad / 2;
    let right_pad = total_pad - left_pad;
    let left: String = std::iter::repeat_n(pad_ch, left_pad).collect();
    let right: String = std::iter::repeat_n(pad_ch, right_pad).collect();
    Ok(Expression::String(format!("{}{}{}", left, s, right)))
}

// 已存在的独立函数
fn wrap(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("wrap", args, 2)?;
    match args[0].eval(env)? {
        Expression::Integer(columns) => {
            Ok(textwrap::fill(&args[1].eval(env)?.to_string(), columns as usize).into())
        }
        otherwise => Err(LmError::CustomError(format!(
            "expected number of columns in wrap, but got `{}`",
            otherwise
        ))),
    }
}

fn href(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("href", args, 2)?;
    Ok(format!(
        "\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\",
        url = args[0].eval(env)?,
        text = args[1].eval(env)?
    )
    .into())
}

// 继续实现剩余的单参数样式函数
fn underline(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("underline", args, 1)?;
    Ok(format!("\x1b[4m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn blink(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("blink", args, 1)?;
    Ok(format!("\x1b[5m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn invert(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("invert", args, 1)?;
    Ok(format!("\x1b[7m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn strike(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("strike", args, 1)?;
    Ok(format!("\x1b[9m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

// 实现所有颜色函数
fn red(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("red", args, 1)?;
    Ok(format!("\x1b[91m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn green(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("green", args, 1)?;
    Ok(format!("\x1b[92m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn yellow(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("yellow", args, 1)?;
    Ok(format!("\x1b[93m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn blue(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("blue", args, 1)?;
    Ok(format!("\x1b[94m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn magenta(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("magenta", args, 1)?;
    Ok(format!("\x1b[95m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn cyan(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("cyan", args, 1)?;
    Ok(format!("\x1b[96m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn white(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("white", args, 1)?;
    Ok(format!("\x1b[97m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

// 实现dark命名空间下的颜色函数
fn dark_red(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_red", args, 1)?;
    Ok(format!("\x1b[31m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn dark_green(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_green", args, 1)?;
    Ok(format!("\x1b[32m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn dark_yellow(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_yellow", args, 1)?;
    Ok(format!("\x1b[33m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn dark_blue(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_blue", args, 1)?;
    Ok(format!("\x1b[34m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn dark_magenta(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_magenta", args, 1)?;
    Ok(format!("\x1b[35m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn dark_cyan(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_cyan", args, 1)?;
    Ok(format!("\x1b[36m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn dark_white(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_white", args, 1)?;
    // 修正原始代码中的转义序列错误
    Ok(format!("\x1b[37m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

// pub fn strip_ansi_escapes(text: impl ToString) -> String {
//     let text = text.to_string();
//     let mut result = String::new();
//     let mut is_in_escape = false;
//     for ch in text.chars() {
//         if ch == '\x1b' {
//             is_in_escape = true;
//         } else if is_in_escape && ch == 'm' {
//             is_in_escape = false;
//         } else if !is_in_escape {
//             result.push(ch);
//         }
//     }
//     result
// }
use regex_lite::Regex;
pub fn strip_ansi_escapes(text: &str) -> String {
    // 更全面的正则表达式，匹配大多数常见的 ANSI 转义序列
    let ansi_escape_pattern = Regex::new(r"(?:\\x1b[@-_]|[\x80-\x9F])[0-?]*[ -/]*[@-~]").unwrap();
    ansi_escape_pattern.replace_all(text, "").into_owned()
    // (?:\\x1b[@-_]|[\x80-\x9F]):

    // (?: ... )：这是一个非捕获组，表示匹配其中的内容但不捕获它。
    // \\x1b[@-_]：匹配 \x1b 后面跟着 @ 到 _ 的字符。\x1b 是 ASCII 中的 ESC 字符（即转义字符），表示 ANSI 转义序列的开始。
    // |：逻辑或操作符，表示匹配左边或右边的内容。
    // [\x80-\x9F]：匹配从 \x80 到 \x9F 的字符范围。这些字符也是 ANSI 转义序列的一部分。
    // [0-?]*:

    // [0-?]：匹配从 0 到 ? 的字符范围。? 是 ASCII 中的一个特殊字符。
    // *：表示前面的字符范围可以出现零次或多次。
    // [ -/]*:

    // [ -/]：匹配从空格到 / 的字符范围。
    // *：表示前面的字符范围可以出现零次或多次。
    // [@-~]:

    // [@-~]：匹配从 @ 到 ~ 的字符范围。
    // 这个范围包括了常见的控制字符，如 A-Z, a-z, 0-9, 和一些符号。
}
