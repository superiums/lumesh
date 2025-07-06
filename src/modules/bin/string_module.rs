use super::{get_integer_arg, get_string_arg, get_string_args};
use crate::{
    Environment, Expression, Int, LmError,
    runtime::{IFS_STR, ifs_contains},
};
use common_macros::hash_map;

pub fn get() -> Expression {
    (hash_map! {
        // justify
        // 基础检查
        String::from("is_empty") => Expression::builtin("is_empty", is_empty, "is this string empty?", "<string>"),
        String::from("is_whitespace") => Expression::builtin("is_whitespace", is_whitespace, "is this string whitespace?", "<string>"),
        String::from("is_alpha") => Expression::builtin("is_alpha", is_alpha, "is this string alphabetic?", "<string>"),
        String::from("is_alphanumeric") => Expression::builtin("is_alphanumeric", is_alphanumeric, "is this string alphanumeric?", "<string>"),
        String::from("is_numeric") => Expression::builtin("is_numeric", is_numeric, "is this string numeric?", "<string>"),
        String::from("is_lower") => Expression::builtin("is_lower", is_lower, "is this string lowercase?", "<string>"),
        String::from("is_upper") => Expression::builtin("is_upper", is_upper, "is this string uppercase?", "<string>"),
        String::from("is_title") => Expression::builtin("is_title", is_title, "is this string title case?", "<string>"),

        // 子串检查
        String::from("starts_with") => Expression::builtin("starts_with", starts_with, "check if a string starts with a given substring", "<substring> <string>"),
        String::from("ends_with") => Expression::builtin("ends_with", ends_with, "check if a string ends with a given substring", "<substring> <string>"),
        String::from("contains") => Expression::builtin("contains", contains, "check if a string contains a given substring", "<substring> <string>"),

        // 分割操作
        String::from("split") => Expression::builtin("split", split, "split a string on a given character", "[delimiter] <string>"),
        String::from("split_at") => Expression::builtin("split_at", split_at, "split a string at a given index", "<index> <string>"),
        String::from("chars") => Expression::builtin("chars", chars, "split a string into characters", "<string>"),
        String::from("words") => Expression::builtin("words", words, "split a string into words", "<string>"),
        String::from("lines") => Expression::builtin("lines", lines, "split a string into lines", "<string>"),
        String::from("paragraphs") => Expression::builtin("paragraphs", paragraphs, "split a string into paragraphs", "<string>"),

        // 修改操作
        String::from("repeat") => Expression::builtin("repeat", repeat, "repeat string specified number of times", "<count> <string>"),
        String::from("replace") => Expression::builtin("replace", replace, "replace all instances of a substring", "<old> <new> <string>"),
        String::from("substring") => Expression::builtin("substring", substring, "get substring from start to end indices", "<start> <end> <string>"),
        String::from("remove_prefix") => Expression::builtin("remove_prefix", remove_prefix, "remove prefix if present", "<prefix> <string>"),
        String::from("remove_suffix") => Expression::builtin("remove_suffix", remove_suffix, "remove suffix if present", "<suffix> <string>"),
        String::from("trim") => Expression::builtin("trim", trim, "trim whitespace from a string", "<string>"),
        String::from("trim_start") => Expression::builtin("trim_start", trim_start, "trim whitespace from the start", "<string>"),
        String::from("trim_end") => Expression::builtin("trim_end", trim_end, "trim whitespace from the end", "<string>"),
        String::from("to_lower") => Expression::builtin("to_lower", to_lower, "convert a string to lowercase", "<string>"),
        String::from("to_upper") => Expression::builtin("to_upper", to_upper, "convert a string to uppercase", "<string>"),
        String::from("to_title") => Expression::builtin("to_title", to_title, "convert a string to title case", "<string>"),

        // 高级操作
        String::from("caesar") => Expression::builtin("caesar", caesar_cipher, "encrypt a string using a caesar cipher", "<shift> <string>"),
        String::from("get_width") => Expression::builtin("get_width", get_width, "get the width of a string", "<string>"),

        // 格式化
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

// String operation implementations

fn caesar_cipher(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("caesar_cipher", args, 1..=2)?;

    let text = get_string_arg(args.last().unwrap().eval(env)?)?;
    let shift = if args.len() > 1 {
        get_integer_arg(args[0].eval(env)?)?
    } else {
        13
    };

    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            let base = if c.is_ascii_lowercase() { b'a' } else { b'A' };
            let offset = (c as u8 - base) as i64;
            let shifted = ((offset + shift).rem_euclid(26) as u8 + base) as char;
            result.push(shifted);
        } else {
            result.push(c);
        }
    }
    Ok(Expression::String(result))
}

fn get_width(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("get_width", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;

    let max_width = text.lines().map(|line| line.len()).max().unwrap_or(0);

    Ok(Expression::Integer(max_width as Int))
}

fn is_empty(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("is_empty", args, 1)?;
    let text = super::get_exact_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.is_empty()))
}

fn is_whitespace(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("is_whitespace", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_whitespace())))
}

fn is_alpha(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("is_alpha", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_alphabetic())))
}

fn is_alphanumeric(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("is_alphanumeric", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(
        text.chars().all(|c| c.is_alphanumeric()),
    ))
}

fn is_numeric(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("is_numeric", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_numeric())))
}

fn split(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("split", args, 1..=2)?;

    let string_args = get_string_args(args, env)?;
    let text = string_args.last().unwrap().to_owned();

    let delimiter = if args.len() > 1 {
        string_args.first().unwrap().to_owned()
    } else {
        match env.get("IFS") {
            Some(Expression::String(fs)) => fs,
            _ => " ".to_string(), // 使用空格作为默认分隔符
        }
    };

    let parts: Vec<Expression> = match ifs_contains(IFS_STR, env) {
        true => text
            .split(&delimiter)
            .map(|s| Expression::String(s.to_string()))
            .collect(),
        false => text
            .split_whitespace()
            .map(|s| Expression::String(s.to_string()))
            .collect(),
    };

    Ok(Expression::from(parts))
}

fn to_lower(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("to_lower", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::String(text.to_lowercase()))
}

fn to_upper(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("to_upper", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::String(text.to_uppercase()))
}

fn to_title(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("to_title", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;

    let mut title = String::with_capacity(text.len());
    let mut capitalize = true;

    for c in text.chars() {
        if capitalize {
            title.extend(c.to_uppercase());
            capitalize = false;
        } else {
            title.extend(c.to_lowercase());
        }
        if c.is_whitespace() {
            capitalize = true;
        }
    }
    Ok(Expression::String(title))
}

fn is_lower(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("is_lower", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_lowercase())))
}

fn is_upper(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("is_upper", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_uppercase())))
}

fn is_title(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("is_title", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    let title = to_title(&vec![args[0].clone()], env)?;
    Ok(Expression::Boolean(text == title.to_string()))
}

fn lines(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("lines", args, 1)?;
    let text = get_string_arg(args[0].eval_in_pipe(env)?)?;

    let lines = text
        .lines()
        .map(|line| Expression::String(line.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(lines))
}

fn chars(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("chars", args, 1)?;
    let text = get_string_arg(args[0].eval_in_pipe(env)?)?;

    let chars = text
        .chars()
        .map(|c| Expression::String(c.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(chars))
}

fn words(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("words", args, 1)?;
    let text = get_string_arg(args[0].eval_in_pipe(env)?)?;

    let words = text
        .split_whitespace()
        .map(|word| Expression::String(word.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(words))
}

fn paragraphs(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("paragraphs", args, 1)?;
    let text = get_string_arg(args[0].eval_in_pipe(env)?)?;

    let paragraphs = text
        .split("\n\n")
        .map(|para| Expression::String(para.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(paragraphs))
}

fn split_at(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("split_at", args, 2)?;
    let text = get_string_arg(args[1].eval_in_pipe(env)?)?;
    let index = get_integer_arg(args[0].eval(env)?)? as usize;

    if index > text.len() {
        return Ok(Expression::from(vec![
            Expression::String(text.clone()),
            Expression::String(String::new()),
        ]));
    }

    let (left, right) = text.split_at(index);
    Ok(Expression::from(vec![
        Expression::String(left.to_string()),
        Expression::String(right.to_string()),
    ]))
}

fn trim(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("trim", args, 1)?;
    let text = get_string_arg(args[0].eval_in_pipe(env)?)?;
    Ok(Expression::String(text.trim().to_string()))
}

fn trim_start(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("trim_start", args, 1)?;
    let text = get_string_arg(args[0].eval_in_pipe(env)?)?;
    Ok(Expression::String(text.trim_start().to_string()))
}

fn trim_end(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("trim_end", args, 1)?;
    let text = get_string_arg(args[0].eval_in_pipe(env)?)?;
    Ok(Expression::String(text.trim_end().to_string()))
}

fn replace(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("replace", args, 3)?;
    let string_args = get_string_args(args, env)?;
    let [from, to, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::String(text.replace(from, to)))
}

fn starts_with(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("starts_with", args, 2)?;
    let string_args = get_string_args(args, env)?;
    let [prefix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::Boolean(text.starts_with(prefix)))
}

fn ends_with(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("ends_with", args, 2)?;
    let string_args = get_string_args(args, env)?;
    let [suffix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::Boolean(text.ends_with(suffix)))
}

fn contains(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("contains", args, 2)?;
    let string_args = get_string_args(args, env)?;
    let [substring, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::Boolean(text.contains(substring)))
}

fn repeat(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("repeat", args, 2)?;
    let count = get_integer_arg(args[0].eval(env)?)?;
    let text = get_string_arg(args[1].eval(env)?)?;

    Ok(Expression::String(text.repeat(count.max(0) as usize)))
}

fn substring(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("substring", args, 2..3)?;
    let text = get_string_arg(args.last().unwrap().eval_in_pipe(env)?)?;

    let start = get_integer_arg(args[0].eval(env)?)?;
    let start_idx = if start < 0 {
        (text.len() as i64 + start).max(0) as usize
    } else {
        start.min(text.len() as i64) as usize
    };

    let end_idx = if args.len() == 3 {
        let end = get_integer_arg(args[1].eval(env)?)?;
        if end < 0 {
            (text.len() as i64 + end).max(0) as usize
        } else {
            end.min(text.len() as i64) as usize
        }
    } else {
        text.len()
    };

    if start_idx >= end_idx || start_idx >= text.len() {
        return Ok(Expression::String(String::new()));
    }

    let result: String = text
        .chars()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .collect();
    Ok(Expression::String(result))
}

fn remove_prefix(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("remove_prefix", args, 2)?;
    let string_args = get_string_args(args, env)?;
    let [prefix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::String(
        text.strip_prefix(prefix).unwrap_or(text).to_string(),
    ))
}

fn remove_suffix(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("remove_suffix", args, 2)?;
    let string_args = get_string_args(args, env)?;
    let [suffix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::String(
        text.strip_suffix(suffix).unwrap_or(text).to_string(),
    ))
}

// ================== fmt ====================
fn pad_start(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("pad_start", args, 2..=3)?;
    let (length, pad_char) = match args.len() {
        2 => (args[0].clone(), " ".to_string()),
        3 => (args[0].clone(), args[1].clone().to_string()),
        _ => unreachable!(),
    };
    let s_val = match args.last().unwrap().eval_in_pipe(env)? {
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

    let s_val = match args.last().unwrap().eval_in_pipe(env)? {
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

    let s_val = match args.last().unwrap().eval_in_pipe(env)? {
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

    let template = match args.first().unwrap().eval_in_pipe(env)? {
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
        let value = arg.eval_in_pipe(env)?;
        result = result.replacen("{}", &value.to_string(), 1);
    }

    Ok(Expression::String(result))
}

// 单参数函数（字符串作为最后一个参数）
fn strip(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("strip", args, 1)?;
    Ok(strip_ansi_escapes(args[0].eval_in_pipe(env)?.to_string().as_str()).into())
}

fn bold(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("bold", args, 1)?;
    Ok(format!("\x1b[1m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

// 其他样式函数采用相同模式...
fn faint(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("faint", args, 1)?;
    Ok(format!("\x1b[2m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn italics(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("italics", args, 1)?;
    Ok(format!("\x1b[3m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

// 颜色函数采用相同模式...
fn black(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("black", args, 1)?;
    Ok(format!("\x1b[90m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn dark_black(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_black", args, 1)?;
    Ok(format!("\x1b[30m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
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

fn wrap(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("wrap", args, 2)?;
    match args[0].eval(env)? {
        Expression::Integer(columns) => {
            Ok(textwrap::fill(&args[1].eval_in_pipe(env)?.to_string(), columns as usize).into())
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
        url = args[0].eval_in_pipe(env)?,
        text = args[1].eval_in_pipe(env)?
    )
    .into())
}

// 继续实现剩余的单参数样式函数
fn underline(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("underline", args, 1)?;
    Ok(format!("\x1b[4m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn blink(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("blink", args, 1)?;
    Ok(format!("\x1b[5m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn invert(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("invert", args, 1)?;
    Ok(format!("\x1b[7m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn strike(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("strike", args, 1)?;
    Ok(format!("\x1b[9m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

// 实现所有颜色函数
fn red(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("red", args, 1)?;
    Ok(format!("\x1b[91m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn green(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("green", args, 1)?;
    Ok(format!("\x1b[92m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn yellow(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("yellow", args, 1)?;
    Ok(format!("\x1b[93m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn blue(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("blue", args, 1)?;
    Ok(format!("\x1b[94m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn magenta(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("magenta", args, 1)?;
    Ok(format!("\x1b[95m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn cyan(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("cyan", args, 1)?;
    Ok(format!("\x1b[96m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn white(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("white", args, 1)?;
    Ok(format!("\x1b[97m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

// 实现dark命名空间下的颜色函数
fn dark_red(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_red", args, 1)?;
    Ok(format!("\x1b[31m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn dark_green(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_green", args, 1)?;
    Ok(format!("\x1b[32m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn dark_yellow(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_yellow", args, 1)?;
    Ok(format!("\x1b[33m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn dark_blue(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_blue", args, 1)?;
    Ok(format!("\x1b[34m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn dark_magenta(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_magenta", args, 1)?;
    Ok(format!("\x1b[35m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn dark_cyan(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_cyan", args, 1)?;
    Ok(format!("\x1b[36m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
}

fn dark_white(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dark_white", args, 1)?;
    // 修正原始代码中的转义序列错误
    Ok(format!("\x1b[37m{}\x1b[m\x1b[0m", args[0].eval_in_pipe(env)?).into())
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
