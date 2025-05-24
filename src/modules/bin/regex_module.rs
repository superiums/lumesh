use crate::{Expression, LmError};
use common_macros::hash_map;
use regex_lite::Regex;
use smallstr::SmallString;

pub fn get() -> Expression {
    (hash_map! {
        // 编译正则表达式（实际使用时直接传字符串更高效，这里提供兼容接口）
       SmallString::from("new") => Expression::builtin("new", |args, env| {
            super::check_exact_args_len("new", args, 1)?;
            let pattern = args[0].eval(env)?.to_string();
            Regex::new(&pattern)
                .map(|_| Expression::String(pattern)) // 仅验证正则有效性
                .map_err(|e| LmError::CustomError(e.to_string()))
        }, "compile regex pattern (validation only)"),

        // 检查是否匹配
       SmallString::from("match") => Expression::builtin("match", |args, env| {
            super::check_exact_args_len("match", args, 2)?;
            let pattern = args[0].eval(env)?.to_string();
            let text = args[1].eval(env)?.to_string();
            let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;
            Ok(Expression::Boolean(regex.is_match(&text)))
        }, "check if text matches pattern"),

        // 查找第一个匹配
       SmallString::from("find") => Expression::builtin("find", |args, env| {
            super::check_exact_args_len("find", args, 2)?;
            let pattern = args[0].eval(env)?.to_string();
            let text = args[1].eval(env)?.to_string();
            let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;

            regex.find(&text).map_or(
                Ok(Expression::None),
                |m| Ok(Expression::from(vec![
                    Expression::Integer(m.start() as i64),
                    Expression::Integer(m.end() as i64),
                    Expression::String(m.as_str().to_string())
                ]))
            )
        }, "find first match with [start, end, text]"),

        // 查找所有匹配
       SmallString::from("find_all") => Expression::builtin("find_all", |args, env| {
            super::check_exact_args_len("find_all", args, 2)?;
            let pattern = args[0].eval(env)?.to_string();
            let text = args[1].eval(env)?.to_string();
            let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;

            let matches: Vec<Expression> = regex.find_iter(&text).map(|m| {
                Expression::from(vec![
                    Expression::Integer(m.start() as i64),
                    Expression::Integer(m.end() as i64),
                    Expression::String(m.as_str().to_string())
                ])
            }).collect();

            Ok(Expression::from(matches))
        }, "find all matches as [[start, end, text], ...]"),

        // 获取第一个捕获组
       SmallString::from("capture") => Expression::builtin("capture", |args, env| {
            super::check_exact_args_len("capture", args, 2)?;
            let pattern = args[0].eval(env)?.to_string();
            let text = args[1].eval(env)?.to_string();
            let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;

            regex.captures(&text).map_or(
                Ok(Expression::None),
                |caps| {
                    let groups: Vec<Expression> = (0..caps.len()).map(|i| {
                        caps.get(i).map(|m| Expression::String(m.as_str().to_string()))
                            .unwrap_or(Expression::None)
                    }).collect();
                    Ok(Expression::from(groups))
                }
            )
        }, "get first capture groups as [full, group1, group2, ...]"),

        // 获取所有捕获组
       SmallString::from("captures") => Expression::builtin("captures", |args, env| {
            super::check_exact_args_len("captures", args, 2)?;
            let pattern = args[0].eval(env)?.to_string();
            let text = args[1].eval(env)?.to_string();
            let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;

            let all_caps: Vec<Expression> = regex.captures_iter(&text).map(|caps| {
                let groups: Vec<Expression> = (0..caps.len()).map(|i| {
                    caps.get(i).map(|m| Expression::String(m.as_str().to_string()))
                        .unwrap_or(Expression::None)
                }).collect();
                Expression::from(groups)
            }).collect();

            Ok(Expression::from(all_caps))
        }, "get all captures as [[full, group1, ...], ...]"),

        // 正则分割
       SmallString::from("split") => Expression::builtin("split", |args, env| {
            super::check_exact_args_len("split", args, 2)?;
            let pattern = args[0].eval(env)?.to_string();
            let text = args[1].eval(env)?.to_string();
            let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;

            let parts: Vec<Expression> = regex.split(&text)
                .map(|s| Expression::String(s.to_string()))
                .collect();

            Ok(Expression::from(parts))
        }, "split text by pattern"),

        // 替换所有匹配
       SmallString::from("replace") => Expression::builtin("replace", |args, env| {
            super::check_exact_args_len("replace", args, 3)?;
            let pattern = args[0].eval(env)?.to_string();
            let replacement = args[1].eval(env)?.to_string();
            let text = args[2].eval(env)?.to_string();
            let regex = Regex::new(&pattern).map_err(|e| LmError::CustomError(e.to_string()))?;

            Ok(Expression::String(
                regex.replace_all(&text, replacement.as_str()).to_string()
            ))
        }, "replace all matches in text"),

       SmallString::from("capture-name") => Expression::builtin("capture-name", |args, env| {
                    super::check_args_len("capture-name", args, 2..3)?;

                    let (pattern, s, group_names) = match args.len() {
                        2 => (args[0].clone(), args[1].clone(), false),
                        3 => (args[0].clone(), args[1].clone(), match args[2].eval(env)? {
                            Expression::Boolean(b) => b,
                            _ => return Err(LmError::CustomError("capture-name names parameter must be boolean".to_string())),
                        }),
                        _ => unreachable!(),
                    };

                    let pattern = match pattern.eval(env)? {
                        Expression::Symbol(x)  => x,
                        Expression::String(x) => SmallString::from(x),
                        _ => return Err(LmError::CustomError("capture-name requires string arguments".to_string())),
                    };

                    let s = match s.eval(env)? {
                        Expression::Symbol(x)  => x,
                        Expression::String(x) => SmallString::from(x),
                        _ => return Err(LmError::CustomError("capture-name requires string arguments".to_string())),
                    };

                    let re = Regex::new(&pattern)
                        .map_err(|e| LmError::CustomError(format!("invalid regex pattern: {}", e)))?;

                    if let Some(caps) = re.captures(&s) {
                        let mut result = Vec::new();

                        for (i, name) in re.capture_names().enumerate() {
                            if i == 0 { continue; } // Skip full match

                            let value = match (caps.get(i), group_names, name) {
                                (Some(mat), true, Some(n)) => Expression::from(vec![
                                    Expression::String(n.to_string()),
                                    Expression::String(mat.as_str().to_string())
                                ]),
                                (Some(mat), false, _) => Expression::String(mat.as_str().to_string()),
                                _ => Expression::None,
                            };

                            result.push(value);
                        }

                        return Ok(Expression::from(result));
                    }

                    Ok(Expression::None)
                }, "get regex capture groups, optionally with names"),

               SmallString::from("split") => Expression::builtin("split", |args, env| {
                    super::check_exact_args_len("split", args, 2)?;

                    let pattern = match args[0].eval(env)? {
                        Expression::Symbol(x)  => x,
                        Expression::String(x) => SmallString::from(x),
                        _ => return Err(LmError::CustomError("split requires string arguments".to_string())),
                    };

                    let s = match args[1].eval(env)? {
                        Expression::Symbol(x)  => x,
                        Expression::String(x) => SmallString::from(x),
                        _ => return Err(LmError::CustomError("split requires string arguments".to_string())),
                    };

                    let re = Regex::new(&pattern)
                        .map_err(|e| LmError::CustomError(format!("invalid regex pattern: {}", e)))?;

                    let parts: Vec<Expression> = re.split(&s)
                        .map(|part| Expression::String(part.to_string()))
                        .collect();

                    Ok(Expression::from(parts))
                }, "split string using regex delimiter"),

    })
    .into()
}
