use std::collections::BTreeMap;

use crate::{Environment, Expression, LmError};
use common_macros::{b_tree_map, hash_map};
use regex_lite::Regex;
// 注册所有正则表达式函数
pub fn get() -> Expression {
    (hash_map! {
    // 匹配定位
           String::from("find") => Expression::builtin("find", regex_find,
               "find first regex match with [start, end, text]", "<pattern> <text>"),

           String::from("find_all") => Expression::builtin("find_all", regex_find_all,
               "find all matches as [[start, end, text], ...]", "<pattern> <text>"),

           // 匹配验证
           String::from("match") => Expression::builtin("match", regex_match,
               "check if entire text matches pattern", "<pattern> <text>"),

           // 捕获组操作
           String::from("capture") => Expression::builtin("capture", regex_capture,
               "get first capture groups as [full, group1, group2, ...]", "<pattern> <text>"),

           String::from("captures") => Expression::builtin("captures", regex_captures,
               "get all captures as [[full, group1, ...], ...]", "<pattern> <text>"),

           String::from("capture_name") => Expression::builtin("capture_name", regex_capture_name,
               "get regex capture groups with names", "<pattern> <text>"),

           // 文本处理
           String::from("split") => Expression::builtin("split", regex_split,
               "split text by regex pattern", "<pattern> <text>"),

           String::from("replace") => Expression::builtin("replace", regex_replace,
               "replace all regex matches in text", "<pattern> <replacement> <text>"), })
    .into()
}

// 检查是否匹配
fn regex_match(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("match", args, 2)?;
    let (regex, text) = get_r_args(args, env)?;

    Ok(Expression::Boolean(regex.is_match(&text)))
}

// 查找第一个匹配
fn regex_find(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("find", args, 2)?;
    let (regex, text) = get_r_args(args, env)?;

    regex.find(&text).map_or(Ok(Expression::None), |m| {
        Ok(Expression::from(b_tree_map! {
          String::from("start") => Expression::Integer(m.start() as i64),
          String::from("end") => Expression::Integer(m.end() as i64),
          String::from("found") => Expression::String(m.as_str().to_string()),
        }))
    })
}

// 查找所有匹配
fn regex_find_all(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("find_all", args, 2)?;
    let (regex, text) = get_r_args(args, env)?;

    let matches: Vec<Expression> = regex
        .find_iter(&text)
        .map(|m| {
            Expression::from(b_tree_map! {
              String::from("start") => Expression::Integer(m.start() as i64),
              String::from("end") => Expression::Integer(m.end() as i64),
              String::from("found") => Expression::String(m.as_str().to_string()),
            })
        })
        .collect();
    Ok(Expression::from(matches))
}

// 获取第一个捕获组
fn regex_capture(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("capture", args, 2)?;
    let (regex, text) = get_r_args(args, env)?;

    regex.captures(&text).map_or(Ok(Expression::None), |caps| {
        let groups: Vec<Expression> = (0..caps.len())
            .map(|i| {
                caps.get(i)
                    .map(|m| Expression::String(m.as_str().to_string()))
                    .unwrap_or(Expression::None)
            })
            .collect();
        Ok(Expression::from(groups))
    })
}

// 获取所有捕获组
fn regex_captures(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("captures", args, 2)?;
    let (regex, text) = get_r_args(args, env)?;

    let all_caps: Vec<Expression> = regex
        .captures_iter(&text)
        .map(|caps| {
            let groups: Vec<Expression> = (0..caps.len())
                .map(|i| {
                    caps.get(i)
                        .map(|m| Expression::String(m.as_str().to_string()))
                        .unwrap_or(Expression::None)
                })
                .collect();
            Expression::from(groups)
        })
        .collect();
    Ok(Expression::from(all_caps))
}

// 正则分割
fn regex_split(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("split", args, 2)?;
    let (regex, text) = get_r_args(args, env)?;

    let parts: Vec<Expression> = regex
        .split(&text)
        .map(|s| Expression::String(s.to_string()))
        .collect();
    Ok(Expression::from(parts))
}

// 替换所有匹配
fn regex_replace(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("replace", args, 3)?;

    let first = args[0].eval(env)?;
    let second = args[1].eval(env)?;
    let last = args[2].eval(env)?;
    let (replacement, text, regex) = match last {
        Expression::Regex(regex) => (first.to_string(), second.to_string(), regex.regex),
        Expression::String(text) => {
            let (regex, replace) = get_r_args(args, env)?;
            (replace, text, regex)
        }
        _ => {
            return Err(LmError::CustomError(
                "regex option requires text as last argument".into(),
            ));
        }
    };

    Ok(Expression::String(
        regex.replace_all(&text, replacement.as_str()).to_string(),
    ))
}

// 获取命名捕获组
fn regex_capture_name(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, LmError> {
    super::check_exact_args_len("capture_name", args, 2)?;
    let (re, text) = get_r_args(args, env)?;

    if let Some(caps) = re.captures(&text) {
        // let mut result = Vec::new();
        let mut found = BTreeMap::new();
        for (i, name) in re.capture_names().enumerate().skip(1) {
            if let (Some(mat), Some(n)) = (caps.get(i), name) {
                found.insert(n.to_string(), Expression::String(mat.as_str().to_string()));
            }
        }
        return Ok(Expression::from(found));
    }
    Ok(Expression::None)
}

fn get_r_args(args: &Vec<Expression>, env: &mut Environment) -> Result<(Regex, String), LmError> {
    match (args[0].eval(env)?, args[1].eval(env)?) {
        (Expression::Regex(r), Expression::String(t) | Expression::Symbol(t)) => Ok((r.regex, t)),
        (Expression::String(t) | Expression::Symbol(t), Expression::Regex(r)) => Ok((r.regex, t)),
        (
            Expression::String(r) | Expression::Symbol(r),
            Expression::String(t) | Expression::Symbol(t),
        ) => Ok((
            Regex::new(&r)
                .map_err(|e| LmError::CustomError(format!("invalid regex pattern: {e}")))?,
            t,
        )),

        _ => Err(LmError::CustomError(
            "regex option requires Regex/pattern_string as first argument".into(),
        )),
    }
}
// fn get_regex(arg: Expression) -> Result<Regex, LmError> {
//     match arg {
//         Expression::String(pattern) | Expression::Symbol(pattern) => Ok(Regex::new(&pattern)
//             .map_err(|e| LmError::CustomError(format!("invalid regex pattern: {}", e)))?),
//         Expression::Regex(r) => Ok(r.regex),
//         _ => Err(LmError::CustomError(
//             "regex option requires Regex/pattern_string as first argument".into(),
//         )),
//     }
// }
