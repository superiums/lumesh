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
        String::from("get_width") => Expression::builtin("get_width", get_width, "get the width of a string", "<string>")

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
