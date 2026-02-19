use crate::{
    Environment, Expression, Int, RuntimeError,
    libs::{
        BuiltinInfo,
        bin::colors::{COLOR_MAP, true_color_by_hex},
        helper::{
            check_args_len, check_exact_args_len, get_integer_arg, get_integer_ref, get_string_arg,
            get_string_ref,
        },
        lazy_module::LazyModule,
    },
    reg_info, reg_lazy,
};
use std::collections::BTreeMap;

use crate::libs::bin::into_lib::{
    filesize as to_filesize, float as to_float, int as to_int, striped as strip, table as to_table,
    time as to_time,
};

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // pprint,
        // 转换
        to_int, to_float, to_filesize, to_time, to_table,
        // 基础检查
        is_empty, is_whitespace, is_alpha, is_alphanumeric, is_numeric, is_lower, is_upper, is_title, len,
        // 子串检查
        starts_with, ends_with, contains,
        // 分割操作
        split, split_at, chars, words, words_quoted, lines, paragraphs, concat,
        // 修改操作
        repeat, replace, substring, remove_prefix, remove_suffix, trim, trim_start, trim_end, to_lower, to_upper, to_title,
        // 高级操作
        max_len, grep,
        caesar,
        strip,
        // 格式化
        pad_start, pad_end, center, wrap,
        // 样式
        href, bold, faint, italic, underline, blink, invert, strike,
        // 标准颜色
        black, red, green, yellow, blue, magenta, cyan, white,
        // 高级颜色
        color256, color256_bg, color, color_bg, colors,
    })
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
       // pprint => "convert to table and pretty print", "[headers|header...]"
       // 转换
       to_int => "convert a float or string to an int", "<value>"
       to_float => "convert an int or string to a float", "<value>"
       to_filesize => "parse a string representing a file size into bytes", "<size_str>"
       to_time => "convert a string to a datetime", "<datetime_str> [datetime_template]"
       to_table => "convert third-party command output to a table", "<command_output>"

       // 基础检查
       is_empty => "is this string empty?", "<string>"
       is_whitespace => "is this string whitespace?", "<string>"
       is_alpha => "is this string alphabetic?", "<string>"
       is_alphanumeric => "is this string alphanumeric?", "<string>"
       is_numeric => "is this string numeric?", "<string>"
       is_lower => "is this string lowercase?", "<string>"
       is_upper => "is this string uppercase?", "<string>"
       is_title => "is this string title case?", "<string>"
       len => "get length of string", "<string>"

       // 子串检查
       starts_with => "check if a string starts with a given substring", "<string> <substring>"
       ends_with => "check if a string ends with a given substring", "<string> <substring>"
       contains => "check if a string contains a given substring", "<string> <substring>"

       // 分割操作
       split => "split a string on a given character", "<string> [delimiter]"
       split_at => "split a string at a given index", "<string> <index>"
       chars => "split a string into characters", "<string>"
       words => "split a string into words", "<string>"
       words_quoted => "split a string into words,quoted as one", "<string>"
       lines => "split a string into lines", "<string>"
       paragraphs => "split a string into paragraphs", "<string>"
       concat => "concat strings", "<string>..."

       // 修改操作
       repeat => "repeat string specified number of times", "<string> <count>"
       replace => "replace all instances of a substring", "<string> <old> <new>"
       substring => "get substring from start to end indices", "<string> <start> <end>"
       remove_prefix => "remove prefix if present", "<string> <prefix>"
       remove_suffix => "remove suffix if present", "<string> <suffix>"
       trim => "trim whitespace from a string", "<string>"
       trim_start => "trim whitespace from the start", "<string>"
       trim_end => "trim whitespace from the end", "<string>"
       to_lower => "convert a string to lowercase", "<string>"
       to_upper => "convert a string to uppercase", "<string>"
       to_title => "convert a string to title case", "<string>"

       // 高级操作
       caesar => "encrypt a string using a caesar cipher", "<string> <shift>"
       max_len => "get max length of lines", "<string>"
       grep => "find lines which contains the substring", "<string> <substring>"
       strip => "remove all ANSI escape codes from string", "<string>"

       // 格式化
       pad_start => "pad string to specified length at start", "<string> <length> [pad_char]"
       pad_end => "pad string to specified length at end", "<string> <length> [pad_char]"
       center => "center string by padding both ends", "<string> <length> [pad_char]"
       wrap => "wrap text to fit in specific number of columns", "<string> <width>"

       // 样式
       href => "create terminal hyperlink", "<url> <text>"
       bold => "apply bold styling", "<string>"
       faint => "apply faint/dim styling", "<string>"
       italic => "apply italic styling", "<string>"
       underline => "apply underline styling", "<string>"
       blink => "apply blinking effect", "<string>"
       invert => "invert foreground/background colors", "<string>"
       strike => "apply strikethrough styling", "<string>"

       // 标准颜色
       black => "apply black foreground", "<string>"
       red => "apply red foreground", "<string>"
       green => "apply green foreground", "<string>"
       yellow => "apply yellow foreground", "<string>"
       blue => "apply blue foreground", "<string>"
       magenta => "apply magenta foreground", "<string>"
       cyan => "apply cyan foreground", "<string>"
       white => "apply white foreground", "<string>"

       // 高级颜色
       color256 => "apply color using 256-color code", "<string> <color_spec>"
       color256_bg => "apply background color using 256-color code", "<string> <color_spec>"
       color => "apply true color using RGB values or color_name", "<string> <hex_color|color_name|r,g,b>"
       color_bg => "apply True Color background using RGB values or color_name", "<string> <hex_color|color_name|r,g,b>"
       colors => "list all color_name for True Color", "[skip_colorized?]"
    })
}

// Basic Check Functions
fn is_empty(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_empty", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::Boolean(text.is_empty()))
}

fn is_whitespace(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_whitespace", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_whitespace())))
}

fn is_alpha(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_alpha", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_alphabetic())))
}

fn is_alphanumeric(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_alphanumeric", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::Boolean(
        text.chars().all(|c| c.is_alphanumeric()),
    ))
}

fn is_numeric(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_numeric", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_numeric())))
}

fn is_lower(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_lower", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_lowercase())))
}

fn is_upper(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_upper", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_uppercase())))
}

fn is_title(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_title", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let title = to_title_inner(&text);
    Ok(Expression::Boolean(text == &title))
}

fn len(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("len", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::Integer(text.chars().count() as Int))
}
// Substring Check Functions
fn starts_with(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("starts_with", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let prefix = get_string_ref(&args[1], ctx)?;

    Ok(Expression::Boolean(text.starts_with(prefix)))
}

fn ends_with(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("ends_with", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let suffix = get_string_ref(&args[1], ctx)?;

    Ok(Expression::Boolean(text.ends_with(suffix)))
}

fn contains(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("contains", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let substring = get_string_ref(&args[1], ctx)?;

    Ok(Expression::Boolean(text.contains(substring)))
}
// Splitting Operations
fn split(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("split", &args, 1..=2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;

    let ifs = env.get("IFS");
    let delimiter = if args.len() > 1 {
        let deli = get_string_ref(&args[1], ctx)?;
        Some(deli)
    } else {
        match (
            crate::runtime::ifs_contains(crate::runtime::IFS_STR, env),
            &ifs,
        ) {
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

fn split_at(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("split_at", &args, 2, ctx)?;
    let mut it = args.into_iter();
    let t_expr = it.next().unwrap();
    let i_expr = it.next().unwrap();
    let text = get_string_arg(t_expr, ctx)?;
    let index = get_integer_ref(&i_expr, ctx)? as usize;

    if index > text.len() {
        return Ok(Expression::from(vec![
            Expression::String(text),
            Expression::String(String::new()),
        ]));
    }

    let (left, right) = text.split_at(index);
    Ok(Expression::from(vec![
        Expression::String(left.to_string()),
        Expression::String(right.to_string()),
    ]))
}

fn chars(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("chars", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;

    let chars = text
        .chars()
        .map(|c| Expression::String(c.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(chars))
}

fn words(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("words", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let words = text
        .split_whitespace()
        .map(|word| Expression::String(word.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(words))
}

fn words_quoted(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("words_quoted", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;

    let re = regex_lite::Regex::new(r#""((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|(\S+)"#).unwrap();
    let words = re
        .captures_iter(&text)
        .filter_map(|cap| {
            cap.get(1)
                .or_else(|| cap.get(2))
                .or_else(|| cap.get(3))
                .map(|m| Expression::String(m.as_str().to_string()))
        })
        .collect::<Vec<Expression>>();

    Ok(Expression::from(words))
}

fn lines(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("lines", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;

    let lines = text
        .lines()
        .map(|line| Expression::String(line.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(lines))
}

fn paragraphs(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("paragraphs", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;

    let paragraphs = text
        .split("\n\n")
        .map(|para| Expression::String(para.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(paragraphs))
}
// Modification Operations
fn concat(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("concat", &args, 2.., ctx)?;

    let others = args
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .concat();

    Ok(Expression::from(others))
}

fn repeat(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("repeat", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let count = get_integer_ref(&args[1], ctx)?;

    Ok(Expression::String(
        text.repeat(count.max(0).min(1000) as usize),
    ))
}

fn replace(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("replace", &args, 3, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let from = get_string_ref(&args[1], ctx)?;
    let to = get_string_ref(&args[2], ctx)?;

    Ok(Expression::String(text.replace(from, to)))
}

fn substring(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("substring", &args, 2..=3, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let start = get_integer_ref(&args[1], ctx)?;

    let start_idx = if start < 0 {
        (text.len() as i64 + start).max(0) as usize
    } else {
        start.min(text.len() as i64) as usize
    };

    let end_idx = if args.len() == 3 {
        let end = get_integer_ref(&args[2], ctx)?;
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

fn remove_prefix(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("remove_prefix", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let prefix = get_string_ref(&args[1], ctx)?;

    Ok(Expression::String(
        text.strip_prefix(prefix).unwrap_or(text).to_string(),
    ))
}

fn remove_suffix(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("remove_suffix", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let suffix = get_string_ref(&args[1], ctx)?;

    Ok(Expression::String(
        text.strip_suffix(suffix).unwrap_or(text).to_string(),
    ))
}

fn trim(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("trim", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::String(text.trim().to_string()))
}

fn trim_start(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("trim_start", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::String(text.trim_start().to_string()))
}

fn trim_end(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("trim_end", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::String(text.trim_end().to_string()))
}

fn to_lower(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("to_lower", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::String(text.to_lowercase()))
}

fn to_upper(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("to_upper", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(Expression::String(text.to_uppercase()))
}

fn to_title_inner(text: &str) -> String {
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
    title
}

fn to_title(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("to_title", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let title = to_title_inner(&text);
    Ok(Expression::String(title))
}

// Formatting Operations (continued)
fn pad_start(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("pad_start", &args, 2..=3, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let length = get_integer_ref(&args[1], ctx)?;
    let char = if args.len() > 2 {
        let c = get_string_ref(&args[2], ctx)?;
        c.chars().next().unwrap_or(' ')
    } else {
        ' '
    };

    pad_start_impl(length.max(0) as usize, char, text.to_string())
}

fn pad_end(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("pad_end", &args, 2..=3, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let length = get_integer_ref(&args[1], ctx)?;
    let char = if args.len() > 2 {
        let c = get_string_ref(&args[2], ctx)?;
        c.chars().next().unwrap_or(' ')
    } else {
        ' '
    };

    pad_end_impl(length.max(0) as usize, char, text.to_string())
}

fn center(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("center", &args, 2..=3, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let length = get_integer_ref(&args[1], ctx)?;
    let char = if args.len() > 2 {
        let c = get_string_ref(&args[2], ctx)?;
        c.chars().next().unwrap_or(' ')
    } else {
        ' '
    };

    center_impl(length.max(0) as usize, char, text.to_string())
}

fn wrap(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("wrap", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let columns = get_integer_ref(&args[1], ctx)?;
    Ok(textwrap::fill(text, columns as usize).into())
}

// Style Functions
fn href(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("href", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let url = get_string_ref(&args[1], ctx)?;

    Ok(format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text).into())
}

fn bold(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("bold", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;

    Ok(format!("\x1b[1m{}\x1b[m\x1b[0m", text).into())
}

fn faint(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("faint", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;

    Ok(format!("\x1b[2m{}\x1b[m\x1b[0m", text).into())
}

fn italic(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("italics", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[3m{}\x1b[m\x1b[0m", text).into())
}

fn underline(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("underline", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[4m{}\x1b[m\x1b[0m", text).into())
}

fn blink(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("blink", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[5m{}\x1b[m\x1b[0m", text).into())
}

fn invert(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("invert", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[7m{}\x1b[m\x1b[0m", text).into())
}

fn strike(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("strike", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[9m{}\x1b[m\x1b[0m", text).into())
}
// Standard Color Functions
fn black(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("black", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[90m{}\x1b[m\x1b[0m", text).into())
}

fn red(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("red", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[91m{}\x1b[m\x1b[0m", text).into())
}

fn green(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("green", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[92m{}\x1b[m\x1b[0m", text).into())
}

fn yellow(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("yellow", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[93m{}\x1b[m\x1b[0m", text).into())
}

fn blue(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("blue", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[94m{}\x1b[m\x1b[0m", text).into())
}

fn magenta(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("magenta", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[95m{}\x1b[m\x1b[0m", text).into())
}

fn cyan(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cyan", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[96m{}\x1b[m\x1b[0m", text).into())
}

fn white(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("white", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    Ok(format!("\x1b[97m{}\x1b[m\x1b[0m", text).into())
}
// Advanced Color Functions
fn color256(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("color256", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let color = get_integer_ref(&args[1], ctx)? as usize;

    Ok(format!("\x1b[38;5;{}m{}\x1b[m\x1b[0m", color, text).into())
}

fn color256_bg(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("color256_bg", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let color = get_integer_ref(&args[1], ctx)? as usize;
    Ok(format!("\x1b[48;5;{}m{}\x1b[m\x1b[0m", color, text).into())
}

fn color(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    true_color(args, false, env, ctx)
}

fn color_bg(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    true_color(args, true, env, ctx)
}

fn colors(
    args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    if args.len() > 0 && !args[0].is_truthy() {
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
            ).unwrap();
            if i % 3 == 2 {
                writeln!(&mut stdout, "\n").unwrap();
            }
        }
        writeln!(&mut stdout).unwrap();
        Ok(Expression::None)
    }
}
// Helper Implementation Functions
fn pad_start_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, RuntimeError> {
    if s.len() >= len {
        return Ok(Expression::String(s));
    }
    let pad_len = len - s.len();
    let padding: String = std::iter::repeat_n(pad_ch, pad_len).collect();
    Ok(Expression::String(format!("{padding}{s}")))
}

fn pad_end_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, RuntimeError> {
    if s.len() >= len {
        return Ok(Expression::String(s));
    }
    let pad_len = len - s.len();
    let padding: String = std::iter::repeat_n(pad_ch, pad_len).collect();
    Ok(Expression::String(format!("{s}{padding}")))
}

fn center_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, RuntimeError> {
    let total_pad = len - s.len();
    let left_pad = total_pad / 2;
    let right_pad = total_pad - left_pad;
    let left: String = std::iter::repeat_n(pad_ch, left_pad).collect();
    let right: String = std::iter::repeat_n(pad_ch, right_pad).collect();
    Ok(Expression::String(format!("{left}{s}{right}")))
}

fn true_color(
    args: Vec<Expression>,
    is_bg: bool,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("true_color", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let color_spec = get_string_ref(&args[1], ctx)?;

    let color_code = if let Some(hex) = color_spec.strip_prefix('#') {
        // Parse hex color
        true_color_by_hex(hex, is_bg, ctx)?
    } else {
        // Parse RGB values
        let parts: Vec<&str> = color_spec.split(',').collect();
        match parts.len() {
            1 => {
                // Parse name
                if let Some((r, g, b)) = COLOR_MAP.get(&color_spec.as_str()) {
                    format!("{};{};{}", r, g, b)
                } else {
                    return Err(RuntimeError::common(
                        "invalid color name".into(),
                        ctx.clone(),
                        0,
                    ));
                }
            }
            // Parse RGB
            3 => format!("{}", parts.join(";")),
            _ => {
                return Err(RuntimeError::common(
                    "invalid color format, expected hex or r,g,b".into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
    };

    let prefix = if is_bg { "48" } else { "38" };
    Ok(format!("\x1b[{};2;{}m{}\x1b[m\x1b[0m", prefix, color_code, text).into())
}

// Additional Functions
fn caesar(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("caesar", &args, 1..=2, ctx)?;

    let text = get_string_ref(&args[0], ctx)?;
    let shift = if args.len() > 1 {
        get_integer_arg(args[1].eval(env)?, ctx)?
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

fn max_len(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("max_len", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;

    let max_width = text.lines().map(|line| line.len()).max().unwrap_or(0);

    Ok(Expression::Integer(max_width as Int))
}

fn grep(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("grep", &args, 2, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;
    let pat: &str = get_string_ref(&args[1], ctx)?;

    let lines = text
        .lines()
        .filter(|x| x.contains(pat))
        .map(|line| Expression::String(line.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(lines))
}

// fn table_pprint(
//     args: Vec<Expression>,
//     env: &mut Environment,
//     ctx: &Expression,
// ) -> Result<Expression, RuntimeError> {
//     check_args_len("pprint", &args, 1.., ctx)?;
//     let table = from_module::parse_command_output(args, env, ctx)?;
//     pprint::pretty_printer(&table)
// }
