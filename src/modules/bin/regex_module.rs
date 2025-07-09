use std::collections::BTreeMap;

use crate::{Environment, Expression, LmError};
use common_macros::hash_map;
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

// 辅助函数：验证并获取字符串参数
fn get_string_arg(
    args: &[Expression],
    index: usize,
    func_name: &str,
    env: &mut Environment,
) -> Result<String, LmError> {
    match args
        .get(index)
        .ok_or(LmError::CustomError(format!(
            "regex.{} requires argument at index {}",
            func_name, index
        )))?
        .eval(env)?
    {
        Expression::Symbol(s) | Expression::String(s) => Ok(s.clone()),
        _ => Err(LmError::CustomError(format!(
            "regex.{} requires string argument at index {}",
            func_name, index
        ))),
    }
}

// 检查是否匹配
fn regex_match(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("match", args, 2)?;
    let pattern = get_string_arg(args, 0, "match", env)?;
    let text = get_string_arg(args, 1, "match", env)?;
    let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;
    Ok(Expression::Boolean(regex.is_match(&text)))
}

// 查找第一个匹配
fn regex_find(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("find", args, 2)?;
    let pattern = get_string_arg(args, 0, "find", env)?;
    let text = get_string_arg(args, 1, "find", env)?;
    let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;
    regex.find(&text).map_or(Ok(Expression::None), |m| {
        Ok(Expression::from(vec![
            Expression::Integer(m.start() as i64),
            Expression::Integer(m.end() as i64),
            Expression::String(m.as_str().to_string()),
        ]))
    })
}

// 查找所有匹配
fn regex_find_all(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("find_all", args, 2)?;
    let pattern = get_string_arg(args, 0, "find_all", env)?;
    let text = get_string_arg(args, 1, "find_all", env)?;
    let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;
    let matches: Vec<Expression> = regex
        .find_iter(&text)
        .map(|m| {
            Expression::from(vec![
                Expression::Integer(m.start() as i64),
                Expression::Integer(m.end() as i64),
                Expression::String(m.as_str().to_string()),
            ])
        })
        .collect();
    Ok(Expression::from(matches))
}

// 获取第一个捕获组
fn regex_capture(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("capture", args, 2)?;
    let pattern = get_string_arg(args, 0, "capture", env)?;
    let text = get_string_arg(args, 1, "capture", env)?;
    let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;
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
    let pattern = get_string_arg(args, 0, "captures", env)?;
    let text = get_string_arg(args, 1, "captures", env)?;
    let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;
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
    let pattern = get_string_arg(args, 0, "split", env)?;
    let text = get_string_arg(args, 1, "split", env)?;
    let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;
    let parts: Vec<Expression> = regex
        .split(&text)
        .map(|s| Expression::String(s.to_string()))
        .collect();
    Ok(Expression::from(parts))
}

// 替换所有匹配
fn regex_replace(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("replace", args, 3)?;
    let pattern = get_string_arg(args, 0, "replace", env)?;
    let replacement = get_string_arg(args, 1, "replace", env)?;
    let text = get_string_arg(args, 2, "replace", env)?;
    let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;
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
    let (pattern, text) = match args.len() {
        2 => (args[0].clone(), args[1].clone()),
        _ => unreachable!(),
    };

    let pattern = get_string_arg(&[pattern], 0, "capture_name", env)?;
    let text = get_string_arg(&[text], 0, "capture_name", env)?;

    let re = Regex::new(&pattern)
        .map_err(|e| LmError::CustomError(format!("invalid regex pattern: {}", e)))?;

    if let Some(caps) = re.captures(&text) {
        // let mut result = Vec::new();
        let mut found = BTreeMap::new();
        for (i, name) in re.capture_names().enumerate().skip(1) {
            match (caps.get(i), name) {
                (Some(mat), Some(n)) => {
                    found.insert(n.to_string(), Expression::String(mat.as_str().to_string()));
                }
                _ => {}
            }
        }
        return Ok(Expression::from(found));
    }
    Ok(Expression::None)
}
