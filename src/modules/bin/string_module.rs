use std::collections::HashMap;

use super::from_module::parse_command_output;
use super::into_module::{filesize, float, int};
use super::time_module::parse_time;

use super::{
    check_args_len, check_exact_args_len, get_exact_string_arg, get_integer_arg, get_string_arg,
    get_string_args,
};
use crate::modules::bin::into_module::strip_str;

use crate::modules::pprint::pretty_printer;
use crate::{
    Environment, Expression, Int, LmError,
    runtime::{IFS_STR, ifs_contains},
};
use common_macros::hash_map;
use lazy_static::lazy_static;

pub fn get() -> Expression {
    (hash_map! {
        String::from("pprint") => Expression::builtin("pprint", table_pprint, "convert to table and pretty print", "[headers|header...]"),

        // 转换
        String::from("to_int") => Expression::builtin("int", int, "convert a float or string to an int", "<value>"),
        String::from("to_float") => Expression::builtin("float", float, "convert an int or string to a float", "<value>"),
        String::from("to_filesize") => Expression::builtin("filesize", filesize, "parse a string representing a file size into bytes", "<size_str>"),
        String::from("to_time") => Expression::builtin("time", parse_time, "convert a string to a datetime", "<datetime_str> [datetime_template]"),
        String::from("to_table") => Expression::builtin("table", parse_command_output, "convert third-party command output to a table", "<command_output>"),

        // 基础检查
        String::from("is_empty") => Expression::builtin("is_empty", is_empty, "is this string empty?", "<string>"),
        String::from("is_whitespace") => Expression::builtin("is_whitespace", is_whitespace, "is this string whitespace?", "<string>"),
        String::from("is_alpha") => Expression::builtin("is_alpha", is_alpha, "is this string alphabetic?", "<string>"),
        String::from("is_alphanumeric") => Expression::builtin("is_alphanumeric", is_alphanumeric, "is this string alphanumeric?", "<string>"),
        String::from("is_numeric") => Expression::builtin("is_numeric", is_numeric, "is this string numeric?", "<string>"),
        String::from("is_lower") => Expression::builtin("is_lower", is_lower, "is this string lowercase?", "<string>"),
        String::from("is_upper") => Expression::builtin("is_upper", is_upper, "is this string uppercase?", "<string>"),
        String::from("is_title") => Expression::builtin("is_title", is_title, "is this string title case?", "<string>"),
        String::from("len") => Expression::builtin("len", len, "get length of string", "<string>"),

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
        String::from("concat") => Expression::builtin("concat", concat, "concat strings", "<string>..."),

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
        String::from("grep") => Expression::builtin("grep", grep, "find lines which contains the substring", "<substring> <string>"),
        String::from("strip") => Expression::builtin("strip", strip_str, "remove all ANSI escape codes from string", "<string>"),

        // 格式化
        String::from("pad_start") => Expression::builtin("pad_start", pad_start, "pad string to specified length at start", "<length> [pad_char] <string>"),
        String::from("pad_end") => Expression::builtin("pad_end", pad_end, "pad string to specified length at end", "<length> [pad_char] <string>"),
        String::from("center") => Expression::builtin("center", center, "center string by padding both ends", "<length> [pad_char] <string>"),
        String::from("wrap") => Expression::builtin("wrap", wrap, "wrap text to fit in specific number of columns", "<width> <string>"),
        String::from("format") => Expression::builtin("format", format, "format string using {} placeholders", "<format_string> <args>..."),

        // 样式
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

        // 高级颜色
        String::from("color256") => Expression::builtin("color256", color256, "apply color using 256-color code", "<color_spec> <string>"),
        String::from("color256_bg") => Expression::builtin("color256_bg", color256_bg, "apply background color using 256-color code", "<color_spec> <string>"),
        String::from("color") => Expression::builtin("color", color, "apply true color using RGB values or color_name", "<hex_color|color_name|r,g,b> <string>"),
        String::from("color_bg") => Expression::builtin("color_bg", color_bg, "apply True Color background using RGB values or color_name", "<hex_color|color_name|r,g,b> <string>"),
        String::from("colors") => Expression::builtin("colors", colors, "list all color_name for True Color", "[skip_colorized?]"),

    })
    .into()
}

// String operation implementations
fn table_pprint(args: &[Expression], env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_args_len("len", args, 1..)?;
    let table = parse_command_output(args, env)?;
    pretty_printer(&table)
}
fn len(args: &[Expression], env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("len", args, 1)?;
    match args[0].eval(env)? {
        Expression::Symbol(x) | Expression::String(x) => {
            Ok(Expression::Integer(x.chars().count() as Int))
        }
        otherwise => Err(LmError::TypeError {
            expected: "List".into(),
            found: otherwise.type_name(),
            sym: otherwise.to_string(),
        }),
    }
}
fn caesar_cipher(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_args_len("caesar_cipher", args, 1..=2)?;

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

fn get_width(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("get_width", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;

    let max_width = text.lines().map(|line| line.len()).max().unwrap_or(0);

    Ok(Expression::Integer(max_width as Int))
}

fn grep(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("grep", args, 2)?;
    let pat = get_string_arg(args[0].eval_in_assign(env)?)?;
    let text = get_string_arg(args[1].eval_in_assign(env)?)?;

    let lines = text
        .lines()
        .filter(|x| x.contains(&pat))
        .map(|line| Expression::String(line.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(lines))
}

fn is_empty(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("is_empty", args, 1)?;
    let text = get_exact_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.is_empty()))
}

fn is_whitespace(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("is_whitespace", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_whitespace())))
}

fn is_alpha(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("is_alpha", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_alphabetic())))
}

fn is_alphanumeric(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("is_alphanumeric", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(
        text.chars().all(|c| c.is_alphanumeric()),
    ))
}

fn is_numeric(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("is_numeric", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_numeric())))
}

fn split(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_args_len("split", args, 1..=2)?;

    let string_args = get_string_args(args, env)?;
    let text = string_args.last().unwrap().to_owned();

    let ifs = env.get("IFS");
    let delimiter = if args.len() > 1 {
        string_args.first()
    } else {
        match (ifs_contains(IFS_STR, env), &ifs) {
            (true, Some(Expression::String(fs))) => Some(fs),
            _ => None,
        }
    };

    let parts: Vec<Expression> = match delimiter {
        Some(sep) => text
            .split(sep)
            .map(|s| Expression::String(s.to_string()))
            .collect(),
        _ => text
            .split_whitespace()
            .map(|s| Expression::String(s.to_string()))
            .collect(),
    };

    Ok(Expression::from(parts))
}

fn to_lower(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("to_lower", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::String(text.to_lowercase()))
}

fn to_upper(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("to_upper", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::String(text.to_uppercase()))
}

fn to_title(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("to_title", args, 1)?;
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

fn is_lower(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("is_lower", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_lowercase())))
}

fn is_upper(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("is_upper", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_uppercase())))
}

fn is_title(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("is_title", args, 1)?;
    let text = get_string_arg(args[0].eval(env)?)?;
    let title = to_title(&[args[0].clone()], env)?;
    Ok(Expression::Boolean(text == title.to_string()))
}

fn lines(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("lines", args, 1)?;
    let text = get_string_arg(args[0].eval_in_assign(env)?)?;

    let lines = text
        .lines()
        .map(|line| Expression::String(line.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(lines))
}

fn chars(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("chars", args, 1)?;
    let text = get_string_arg(args[0].eval_in_assign(env)?)?;

    let chars = text
        .chars()
        .map(|c| Expression::String(c.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(chars))
}

fn words(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("words", args, 1)?;
    let text = get_string_arg(args[0].eval_in_assign(env)?)?;

    let words = text
        .split_whitespace()
        .map(|word| Expression::String(word.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(words))
}

fn paragraphs(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("paragraphs", args, 1)?;
    let text = get_string_arg(args[0].eval_in_assign(env)?)?;

    let paragraphs = text
        .split("\n\n")
        .map(|para| Expression::String(para.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(paragraphs))
}

fn concat(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_args_len("concat", args, 2..)?;
    let text = get_string_arg(args.last().unwrap().eval_in_assign(env)?)?;
    let others = args[..args.len() - 1]
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .concat();

    Ok(Expression::from(text + &others))
}

fn split_at(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("split_at", args, 2)?;
    let text = get_string_arg(args[1].eval_in_assign(env)?)?;
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

fn trim(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("trim", args, 1)?;
    let text = get_string_arg(args[0].eval_in_assign(env)?)?;
    Ok(Expression::String(text.trim().to_string()))
}

fn trim_start(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("trim_start", args, 1)?;
    let text = get_string_arg(args[0].eval_in_assign(env)?)?;
    Ok(Expression::String(text.trim_start().to_string()))
}

fn trim_end(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("trim_end", args, 1)?;
    let text = get_string_arg(args[0].eval_in_assign(env)?)?;
    Ok(Expression::String(text.trim_end().to_string()))
}

fn replace(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("replace", args, 3)?;
    let string_args = get_string_args(args, env)?;
    let [from, to, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::String(text.replace(from, to)))
}

fn starts_with(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("starts_with", args, 2)?;
    let string_args = get_string_args(args, env)?;
    let [prefix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::Boolean(text.starts_with(prefix)))
}

fn ends_with(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("ends_with", args, 2)?;
    let string_args = get_string_args(args, env)?;
    let [suffix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::Boolean(text.ends_with(suffix)))
}

fn contains(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("contains", args, 2)?;
    let string_args = get_string_args(args, env)?;
    let [substring, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::Boolean(text.contains(substring)))
}

fn repeat(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("repeat", args, 2)?;
    let count = get_integer_arg(args[0].eval(env)?)?;
    let text = get_string_arg(args[1].eval(env)?)?;

    Ok(Expression::String(text.repeat(count.max(0) as usize)))
}

fn substring(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_args_len("substring", args, 2..3)?;
    let text = get_string_arg(args.last().unwrap().eval_in_assign(env)?)?;

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

fn remove_prefix(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("remove_prefix", args, 2)?;
    let string_args = get_string_args(args, env)?;
    let [prefix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::String(
        text.strip_prefix(prefix).unwrap_or(text).to_string(),
    ))
}

fn remove_suffix(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("remove_suffix", args, 2)?;
    let string_args = get_string_args(args, env)?;
    let [suffix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::String(
        text.strip_suffix(suffix).unwrap_or(text).to_string(),
    ))
}

// ================== fmt ====================
fn pad_start(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_args_len("pad_start", args, 2..=3)?;
    let (length, pad_char) = match args.len() {
        2 => (args[0].clone(), " ".to_string()),
        3 => (args[0].clone(), args[1].clone().to_string()),
        _ => unreachable!(),
    };
    let s_val = match args.last().unwrap().eval_in_assign(env)? {
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

fn pad_end(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_args_len("pad_end", args, 2..=3)?;
    let (length, pad_char) = match args.len() {
        2 => (args[0].clone(), " ".to_string()),
        3 => (args[0].clone(), args[1].clone().to_string()),
        _ => unreachable!(),
    };

    let s_val = match args.last().unwrap().eval_in_assign(env)? {
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

fn center(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_args_len("center", args, 2..=3)?;
    let (length, pad_char) = match args.len() {
        2 => (args[0].clone(), " ".to_string()),
        3 => (args[0].clone(), args[1].clone().to_string()),
        _ => unreachable!(),
    };

    let s_val = match args.last().unwrap().eval_in_assign(env)? {
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
fn format(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    if args.is_empty() {
        return Err(LmError::CustomError(
            "format requires at least a template string".to_string(),
        ));
    }

    let template = match args.first().unwrap().eval_in_assign(env)? {
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
            "format requires {placeholders} arguments for {placeholders} placeholders"
        )));
    }

    let mut result = template.clone();
    for arg in args.iter().skip(1).take(placeholders) {
        let value = arg.eval_in_assign(env)?;
        result = result.replacen("{}", &value.to_string(), 1);
    }

    Ok(Expression::String(result))
}

fn bold(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("bold", args, 1)?;
    Ok(format!("\x1b[1m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

// 其他样式函数采用相同模式...
fn faint(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("faint", args, 1)?;
    Ok(format!("\x1b[2m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

fn italics(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("italics", args, 1)?;
    Ok(format!("\x1b[3m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

// 颜色函数采用相同模式...
fn black(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("black", args, 1)?;
    Ok(format!("\x1b[90m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

// 其他颜色函数类似实现...

// 原始实现函数保持不变
fn pad_start_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, LmError> {
    if s.len() >= len {
        return Ok(Expression::String(s));
    }
    let pad_len = len - s.len();
    let padding: String = std::iter::repeat_n(pad_ch, pad_len).collect();
    Ok(Expression::String(format!("{padding}{s}")))
}

fn pad_end_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, LmError> {
    if s.len() >= len {
        return Ok(Expression::String(s));
    }
    let pad_len = len - s.len();
    let padding: String = std::iter::repeat_n(pad_ch, pad_len).collect();
    Ok(Expression::String(format!("{s}{padding}")))
}

fn center_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, LmError> {
    let total_pad = len - s.len();
    let left_pad = total_pad / 2;
    let right_pad = total_pad - left_pad;
    let left: String = std::iter::repeat_n(pad_ch, left_pad).collect();
    let right: String = std::iter::repeat_n(pad_ch, right_pad).collect();
    Ok(Expression::String(format!("{left}{s}{right}")))
}

fn wrap(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("wrap", args, 2)?;
    match args[0].eval(env)? {
        Expression::Integer(columns) => {
            Ok(textwrap::fill(&args[1].eval_in_assign(env)?.to_string(), columns as usize).into())
        }
        otherwise => Err(LmError::CustomError(format!(
            "expected number of columns in wrap, but got `{otherwise}`"
        ))),
    }
}

fn href(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("href", args, 2)?;
    Ok(format!(
        "\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\",
        url = args[0].eval_in_assign(env)?,
        text = args[1].eval_in_assign(env)?
    )
    .into())
}

// 继续实现剩余的单参数样式函数
fn underline(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("underline", args, 1)?;
    Ok(format!("\x1b[4m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

fn blink(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("blink", args, 1)?;
    Ok(format!("\x1b[5m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

fn invert(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("invert", args, 1)?;
    Ok(format!("\x1b[7m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

fn strike(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("strike", args, 1)?;
    Ok(format!("\x1b[9m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

// 实现所有颜色函数
fn red(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("red", args, 1)?;
    Ok(format!("\x1b[91m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

fn green(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("green", args, 1)?;
    Ok(format!("\x1b[92m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

fn yellow(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("yellow", args, 1)?;
    Ok(format!("\x1b[93m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

fn blue(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("blue", args, 1)?;
    Ok(format!("\x1b[94m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

fn magenta(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("magenta", args, 1)?;
    Ok(format!("\x1b[95m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

fn cyan(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("cyan", args, 1)?;
    Ok(format!("\x1b[96m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

fn white(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("white", args, 1)?;
    Ok(format!("\x1b[97m{}\x1b[m\x1b[0m", args[0].eval_in_assign(env)?).into())
}

// 实现dark命名空间下的颜色函数

fn color_256(args: &[Expression], bg: bool, env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("color256", args, 2)?;
    let color_spec = get_integer_arg(args[0].eval_in_assign(env)?)?;
    let text = get_string_arg(args[1].eval_in_assign(env)?)?;

    if !(0..=255).contains(&color_spec) {
        return Err(LmError::CustomError(
            "color values must between 0-255".into(),
        ));
    }

    if bg {
        Ok(format!("\x1b[48;5;{color_spec}m{text}\x1b[m\x1b[0m").into())
    } else {
        Ok(format!("\x1b[38;5;{color_spec}m{text}\x1b[m\x1b[0m").into())
    }
}

fn color256(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    color_256(args, false, env)
}
fn color256_bg(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    color_256(args, true, env)
}

fn colors(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() > 0 && !args[0].eval(env)?.is_truthy() {
        Ok(Expression::from(
            COLOR_MAP
                .iter()
                .map(|(&k, _)| Expression::String(k.to_owned()))
                .collect::<Vec<_>>(),
        ))
    } else {
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();

        for (i, (text, (r, g, b))) in COLOR_MAP.iter().enumerate() {
            let pad_len = 20 - text.len();
            let padding: String = std::iter::repeat_n(" ", pad_len).collect();
            write!(
                &mut stdout,
                "\x1b[48;2;{r};{g};{b}m     \x1b[m\x1b[0m \x1b[38;2;{r};{g};{b}m{text}\x1b[m\x1b[0m{padding}"
            )?;
            if i % 3 == 2 {
                writeln!(&mut stdout, "\n")?;
            }
        }
        writeln!(&mut stdout)?;
        Ok(Expression::None)
    }
}
fn color(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    true_color(args, false, env)
}
fn color_bg(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    true_color(args, true, env)
}

fn true_color(args: &[Expression], bg: bool, env: &mut Environment) -> Result<Expression, LmError> {
    check_args_len("color", args, 2..=4)?;

    let (r, g, b) = match args.len() {
        2 => match args[0].eval_in_assign(env)? {
            // 十六进制颜色代码，如 #FF0000 或 #ff0000
            Expression::String(hex_color) if hex_color.starts_with('#') => {
                parse_hex_color(&hex_color)?
            }
            // 颜色名称，如 "red", "green", "blue"
            Expression::String(color_name) | Expression::Symbol(color_name) => {
                parse_color_name(color_name.as_str())?
            }
            _ => {
                return Err(LmError::CustomError(
                    "Color spec must be hex color (#RRGGBB), color name, or RGB integer".into(),
                ));
            }
        },
        4 => {
            let r = get_integer_arg(args[0].eval_in_assign(env)?)?;
            let g = get_integer_arg(args[1].eval_in_assign(env)?)?;
            let b = get_integer_arg(args[2].eval_in_assign(env)?)?;
            (r, g, b)
        }
        _ => return Err(LmError::CustomError("Args mismatch".into())),
    };
    let range = 0..=255;
    if !range.contains(&r) || !range.contains(&g) || !range.contains(&b) {
        return Err(LmError::CustomError("RGB values must be 0-255".into()));
    }

    let text = get_string_arg(args.last().unwrap().eval_in_assign(env)?)?;
    if bg {
        Ok(format!("\x1b[48;2;{r};{g};{b}m{text}\x1b[m\x1b[0m").into())
    } else {
        Ok(format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[m\x1b[0m").into())
    }
}

// 解析十六进制颜色代码
fn parse_hex_color(hex: &str) -> Result<(i64, i64, i64), LmError> {
    if hex.len() != 7 {
        return Err(LmError::CustomError(
            "Hex color must be in format #RRGGBB".into(),
        ));
    }

    let hex_digits = &hex[1..]; // 去掉 # 前缀

    let r = i64::from_str_radix(&hex_digits[0..2], 16)
        .map_err(|_| LmError::CustomError("Invalid hex color format".into()))?;
    let g = i64::from_str_radix(&hex_digits[2..4], 16)
        .map_err(|_| LmError::CustomError("Invalid hex color format".into()))?;
    let b = i64::from_str_radix(&hex_digits[4..6], 16)
        .map_err(|_| LmError::CustomError("Invalid hex color format".into()))?;

    Ok((r, g, b))
}

fn parse_color_name(color: &str) -> Result<(i64, i64, i64), LmError> {
    COLOR_MAP
        .get(color)
        .cloned()
        .ok_or(LmError::CustomError("Invalid color name".into()))
}

lazy_static! {
    static ref COLOR_MAP: HashMap<&'static str, (i64, i64, i64)> = hash_map! {
        "aliceblue" => (240, 248, 255),
        "antiquewhite" => (250, 235, 215),
        "aqua" => (0, 255, 255),
        "aquamarine" => (127, 255, 212),
        "azure" => (240, 255, 255),
        "beige" => (245, 245, 220),
        "bisque" => (255, 228, 196),
        "black" => (0, 0, 0),
        "blanchedalmond" => (255, 235, 205),
        "blue" => (0, 0, 255),
        "blueviolet" => (138, 43, 226),
        "brown" => (165, 42, 42),
        "burlywood" => (222, 184, 135),
        "cadetblue" => (95, 158, 160),
        "chartreuse" => (127, 255, 0),
        "chocolate" => (210, 105, 30),
        "coral" => (255, 127, 80),
        "cornflowerblue" => (100, 149, 237),
        "cornsilk" => (255, 248, 220),
        "crimson" => (220, 20, 60),
        "cyan" => (0, 255, 255),
        "darkblue" => (0, 0, 139),
        "darkcyan" => (0, 139, 139),
        "darkgoldenrod" => (184, 134, 11),
        "darkgray" => (169, 169, 169),
        "darkgreen" => (0, 100, 0),
        "darkgrey" => (169, 169, 169),
        "darkkhaki" => (189, 183, 107),
        "darkmagenta" => (139, 0, 139),
        "darkolivegreen" => (85, 107, 47),
        "darkorange" => (255, 140, 0),
        "darkorchid" => (153, 50, 204),
        "darkred" => (139, 0, 0),
        "darksalmon" => (233, 150, 122),
        "darkseagreen" => (143, 188, 143),
        "darkslateblue" => (72, 61, 139),
        "darkslategrey" => (47, 79, 79),
        "darkturquoise" => (0, 206, 209),
        "darkviolet" => (148, 0, 211),
        "deeppink" => (255, 20, 147),
        "deepskyblue" => (0, 191, 255),
        "dimgray" => (105, 105, 105),
        "dodgerblue" => (30, 144, 255),
        "firebrick" => (178, 34, 34),
        "floralwhite" => (255, 250, 240),
        "forestgreen" => (34, 139, 34),
        "fuchsia" => (255, 0, 255),
        "gainsboro" => (221, 221, 221),
        "ghostwhite" => (248, 248, 255),
        "gold" => (255, 215, 0),
        "goldenrod" => (218, 165, 32),
        "gray" => (128, 128, 128),
        "green" => (0, 255, 0),
        "greenyellow" => (173, 255, 47),
        "honeydew" => (240, 255, 240),
        "hotpink" => (255, 105, 180),
        "indianred" => (205, 92, 92),
        "indigo" => (75, 0, 130),
        "ivory" => (255, 255, 240),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "lavenderblush" => (255, 245, 245),
        "lawngreen" => (124, 252, 0),
        "lemonchiffon" => (255, 250, 205),
        "lightblue" => (173, 216, 230),
        "lightcoral" => (240, 128, 128),
        "lightcyan" => (224, 255, 255),
        "lightgoldenrodyellow" => (250, 250, 210),
        "lightgray" => (211, 211, 211),
        "lightgreen" => (144, 238, 144),
        "lightgrey" => (211, 211, 211),
        "lightpink" => (255, 182, 193),
        "lightsalmon" => (255, 160, 122),
        "lightseagreen" => (32, 178, 170),
        "lightskyblue" => (135, 206, 250),
        "lightslategray" => (119, 136, 153),
        "lightsteelblue" => (176, 196, 222),
        "lightyellow" => (255, 255, 224),
        "lime" => (0, 255, 0),
        "limegreen" => (50, 205, 50),
        "linen" => (250, 240, 230),
        "magenta" => (255, 0, 255),
        "maroon" => (128, 0, 0),
        "mediumaquamarine" => (102, 209, 209),
        "mediumblue" => (0, 0, 205),
        "mediumorchid" => (183, 105, 224),
        "mediumpurple" => (147, 112, 219),
        "mediumseagreen" => (60, 179, 113),
        "mediumslateblue" => (123, 104, 238),
        "mediumspringgreen" => (0, 250, 150),
        "mediumturquoise" => (72, 209, 204),
        "mediumvioletred" => (199, 21, 133),
        "midnightblue" => (25, 25, 112),
        "mintcream" => (245, 255, 250),
        "mistyrose" => (255, 228, 225),
        "moccasin" => (255, 228, 181),
        "navajowhite" => (255, 222, 173),
        "navy" => (0, 0, 128),
        "oldlace" => (253, 245, 230),
        "olive" => (128, 128, 0),
        "olivedrab" => (107, 142, 35),
        "orange" => (255, 165, 0),
        "orangered" => (255, 69, 0),
        "orchid" => (218, 112, 214),
        "palegoldenrod" => (238, 232, 170),
        "palegreen" => (152, 251, 152),
        "paleturquoise" => (175, 238, 238),
        "palevioletred" => (238, 130, 238),
        "papayawhip" => (255, 239, 213),
        "peachpuff" => (255, 218, 185),
        "peru" => (205, 133, 63),
        "pink" => (255, 192, 203),
        "plum" => (221, 160, 221),
        "powderblue" => (176, 224, 230),
        "purple" => (128, 0, 128),
        "rebeccapurple" => (102, 51, 153),
        "red" => (255, 0, 0),
        "rosybrown" => (188, 143, 143),
        "royalblue" => (65, 105, 225),
        "saddlebrown" => (139, 69, 19),
        "salmon" => (250, 128, 114),
        "sandybrown" => (244, 164, 96),
        "seagreen" => (46, 139, 87),
        "seashell" => (255, 245, 238),
        "sienna" => (160, 82, 45),
        "silver" => (192, 192, 192),
        "skyblue" => (135, 206, 235),
        "slateblue" => (106, 90, 205),
        "slategray" => (112, 128, 144),
        "snow" => (255, 250, 250),
        "springgreen" => (0, 255, 128),
        "steelblue" => (70, 130, 180),
        "tan" => (210, 180, 140),
        "teal" => (0, 128, 128),
        "thistle" => (216, 191, 216),
        "tomato" => (255, 99, 71),
        "turquoise" => (64, 224, 208),
        "violet" => (238, 130, 238),
        "wheat" => (245, 222, 179),
        "white" => (255, 255, 255),
        "whitesmoke" => (245, 245, 245),
        "yellow" => (255, 255, 0),
        "yellowgreen" => (154, 255, 50),
    };
}
