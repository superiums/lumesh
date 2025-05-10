use crate::{Environment, Expression, LmError};
use common_macros::hash_map;

fn split(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 2 {
        return Err(LmError::CustomError(format!(
            "expected 2 arguments, got {}",
            args.len()
        )));
    }
    match (args[0].eval(env)?, args[1].eval(env)?) {
        (
            Expression::Symbol(x) | Expression::String(x),
            Expression::Symbol(y) | Expression::String(y),
        ) => {
            let mut v = Vec::new();
            for s in y.split(&x) {
                v.push(Expression::String(s.to_string()));
            }
            Ok(Expression::from(v))
        }
        (a, b) => Err(LmError::CustomError(format!(
            "expected string, got values {} and {}",
            a, b
        ))),
    }
}

pub fn get() -> Expression {
    (hash_map! {
        String::from("to_string") => Expression::builtin("to_string", |args, env| {
            super::check_exact_args_len("to_string", &args, 1)?;
            Ok(Expression::String(args[0].eval(env)?.to_string()))
        }, "convert a value to a string"),

        String::from("caesar") => Expression::builtin("caesar_cipher", |args, env| {
            super::check_args_len("caesar_cipher", &args, 1..=2)?;

            let expr = args[0].eval(env)?;
            let shift = if args.len() > 1 {
                args[1].eval(env)?
            } else {
                Expression::Integer(13)
            };
            Ok(match (expr, shift) {
                (Expression::Symbol(x) | Expression::String(x), Expression::Integer(i)) => {
                    let mut result = String::new();
                    for c in x.chars() {
                        // If the character is a letter, shift it
                        if c.is_ascii_alphabetic() {
                            // Get the base character code
                            let base = if c.is_ascii_lowercase() {
                                b'a'
                            } else {
                                b'A'
                            };
                            // Get the offset from the base
                            let offset = c as u8 - base;
                            // Shift the offset
                            let shifted_offset = (offset + (i as u8)) % 26;
                            // Get the shifted character
                            let shifted_char = (shifted_offset + base) as char;
                            // Add the shifted character to the result
                            result.push(shifted_char);
                        } else {
                            // If the character is not a letter, just add it to the result
                            result.push(c);
                        }
                    }
                    Expression::String(result)
                }
                _ => Expression::None,
            })
        }, "encrypt a string using a caesar cipher"),

        String::from("len") => Expression::builtin("len", super::len, "get the length of a string"),

        String::from("get_width") => Expression::builtin("get_width", |args, env| {
            super::check_exact_args_len("get_width", &args, 1)?;
            let expr = args[0].eval(env)?;
            Ok(Expression::Integer(match expr {
                Expression::Symbol(x) | Expression::String(x) => {
                    let mut width = 0;
                    let mut max_width = 0;
                    for c in x.chars() {
                        if c == '\n' {
                            if width > max_width {
                                max_width = width;
                            }
                            width = 0;
                        } else {
                            width += 1;
                        }
                    }

                    if width > max_width {
                        width
                    } else {
                        max_width
                    }
                },
                _ => 0
            }))
        }, "get the width of a string"),

        String::from("is_whitespace") => Expression::builtin("is_whitespace", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::Boolean(x.chars().all(|c| c.is_whitespace())))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "is this string whitespace?"),

        String::from("is_alpha") => Expression::builtin("is_alpha", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::Boolean(x.chars().all(|c| c.is_alphabetic())))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "is this string alphabetic?"),

        String::from("is_alphanumeric") => Expression::builtin("is_alphanumeric", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::Boolean(x.chars().all(|c| c.is_alphanumeric())))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "is this string alphanumeric?"),

        String::from("is_numeric") => Expression::builtin("is_numeric", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::Boolean(x.chars().all(|c| c.is_numeric())))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "is this string numeric?"),

        // TODO test, removed curry
        String::from("split") => Expression::builtin("split", split, "split a string on a given character"),

        String::from("to_lower") => Expression::builtin("to_lower", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::String(x.to_lowercase()))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "convert a string to lowercase"),

        String::from("to_upper") => Expression::builtin("to_upper", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::String(x.to_uppercase()))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "convert a string to uppercase"),

        String::from("to_title") => Expression::builtin("to_title", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    let mut title = String::new();
                    let mut capitalize = true;
                    for c in x.chars() {
                        if capitalize {
                            title.push(c.to_uppercase().next().unwrap());
                            capitalize = false;
                        } else {
                            title.push(c.to_lowercase().next().unwrap());
                        }
                        if c.is_whitespace() {
                            capitalize = true;
                        }
                    }
                    Ok(Expression::String(title))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "convert a string to title case"),

        String::from("is_lower") => Expression::builtin("is_lower", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::Boolean(x.chars().all(|c| c.is_lowercase())))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "is this string lowercase?"),

        String::from("is_upper") => Expression::builtin("is_upper", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::Boolean(x.chars().all(|c| c.is_uppercase())))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "is this string uppercase?"),

        String::from("is_title") => Expression::builtin("is_title", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    let mut title = String::new();
                    let mut capitalize = true;
                    for c in x.chars() {
                        if capitalize {
                            title.push(c.to_uppercase().next().unwrap());
                            capitalize = false;
                        } else {
                            title.push(c.to_lowercase().next().unwrap());
                        }
                        if c.is_whitespace() {
                            capitalize = true;
                        }
                    }
                    Ok(Expression::Boolean(x == title))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "is this string title case?"),

        String::from("rev") => Expression::builtin("rev", super::rev, "reverse a string"),

        String::from("join") => Expression::builtin("join", |args, env| {
            super::check_exact_args_len("join", &args, 2)?;
            let expr = args[0].eval(env)?;
            let separator = args[1].eval(env)?;
            Ok(match expr {
                Expression::List(list) => {
                    let mut joined = String::new();
                    for (i, item) in list.as_ref().iter().enumerate() {
                        if i != 0 {
                            joined.push_str(&separator.to_string());
                        }
                        joined.push_str(&item.to_string());
                    }
                    Expression::String(joined)
                }
                _ => Expression::None,
            })
        }, "join a list of strings with a separator"),

        String::from("lines") => Expression::builtin("lines", |args, env| {
            super::check_exact_args_len("lines", &args, 1)?;
            let expr = args[0].eval(env)?;
            Ok(match expr {
                Expression::Symbol(x) | Expression::String(x) => Expression::from(
                    x.lines()
                        .map(|line| Expression::String(line.to_string()))
                        .collect::<Vec<Expression>>(),
                ),
                _ => Expression::None,
            })
        }, "split a string into lines"),

        String::from("chars") => Expression::builtin("chars", |args, env| {
            super::check_exact_args_len("chars", &args, 1)?;
            // Ok(match expr {
            //     Expression::Symbol(x) | Expression::String(x) => Expression::List(
            //         x.chars()
            //             .map(|c| Expression::String(c.to_string()))
            //             .collect(),
            //     ),
            //     _ => Expression::None,
            // })

            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => Ok(Expression::from(
                    x.chars()
                        .map(|ch| Expression::String(ch.to_string()))
                        .collect::<Vec<Expression>>(),
                )),
                otherwise => Err(LmError::CustomError(format!(
                    "cannot get characters of non-string {}",
                    otherwise
                ))),
            }
        }, "split a string into characters"),

        String::from("words") => Expression::builtin("words", |args, env| {
            super::check_exact_args_len("words", &args, 1)?;
            let expr = args[0].eval(env)?;
            Ok(match expr {
                Expression::Symbol(x) | Expression::String(x) => Expression::from(
                    x.split_whitespace()
                        .map(|word| Expression::String(word.to_string()))
                        .collect::<Vec<Expression>>(),

                ),
                _ => Expression::None,
            })
        }, "split a string into words"),

        String::from("paragraphs") => Expression::builtin("paragraphs", |args, env| {
            super::check_exact_args_len("paragraphs", &args, 1)?;
            let expr = args[0].eval(env)?;
            Ok(match expr {
                Expression::Symbol(x) | Expression::String(x) => Expression::from(
                    x.split("\n\n")
                        .map(|paragraph| Expression::String(paragraph.to_string()))
                        .collect::<Vec<Expression>>(),

                ),
                _ => Expression::None,
            })
        }, "split a string into paragraphs"),

        String::from("split_at") => Expression::builtin("split_at", |args, env| {
            super::check_exact_args_len("split_at", &args, 2)?;
            let expr = args[0].eval(env)?;
            let index = args[1].eval(env)?;
            Ok(match (expr, index) {
                (Expression::Symbol(x) | Expression::String(x), Expression::Integer(i)) => {
                    Expression::from(vec![
                        Expression::String(x[..i as usize].to_string()),
                        Expression::String(x[i as usize..].to_string()),
                    ])
                }
                _ => Expression::None,
            })
        }, "split a string at a given index"),

        String::from("trim") => Expression::builtin("trim", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::String(x.trim().to_string()))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "trim whitespace from a string"),

        String::from("trim_start") => Expression::builtin("trim_start", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::String(x.trim_start().to_string()))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "trim whitespace from the start of a string"),

        String::from("trim_end") => Expression::builtin("trim_end", |args, env| {
            match args[0].eval(env)? {
                Expression::Symbol(x) | Expression::String(x) => {
                    Ok(Expression::String(x.trim_end().to_string()))
                }
                otherwise => Err(LmError::CustomError(format!(
                    "expected string, got value {}",
                    otherwise
                ))),
            }
        }, "trim whitespace from the end of a string"),

        String::from("replace") => Expression::builtin("replace", |args, env| {
            super::check_exact_args_len("replace", &args, 3)?;
            let expr = args[0].eval(env)?;
            let old = args[1].eval(env)?;
            let new = args[2].eval(env)?;
            Ok(match expr {
                Expression::Symbol(x) | Expression::String(x) => {
                    Expression::String(x.replace(&old.to_string(), &new.to_string()))
                }
                _ => Expression::None,
            })
        }, "replace all instances of a substring in a string with another string"),

        String::from("starts_with") => Expression::builtin("starts_with", |args, env| {
            super::check_exact_args_len("starts_with", &args, 2)?;
            let expr = args[0].eval(env)?;
            let prefix = args[1].eval(env)?;
            Ok(match expr {
                Expression::Symbol(x) | Expression::String(x) => {
                    Expression::Boolean(x.starts_with(&prefix.to_string()))
                }
                _ => Expression::None,
            })
        }, "check if a string starts with a given substring"),

        String::from("ends_with") => Expression::builtin("ends_with", |args, env| {
            super::check_exact_args_len("ends_with", &args, 2)?;
            let expr = args[0].eval(env)?;
            let suffix = args[1].eval(env)?;
            Ok(match expr {
                Expression::Symbol(x) | Expression::String(x) => {
                    Expression::Boolean(x.ends_with(&suffix.to_string()))
                }
                _ => Expression::None,
            })
        }, "check if a string ends with a given substring"),

        String::from("contains") => Expression::builtin("contains", |args, env| {
            super::check_exact_args_len("contains", &args, 2)?;
            let expr = args[0].eval(env)?;
            let substring = args[1].eval(env)?;
            Ok(match expr {
                Expression::Symbol(x) | Expression::String(x) => {
                    Expression::Boolean(x.contains(&substring.to_string()))
                }
                _ => Expression::None,
            })
        }, "check if a string contains a given substring"),

        String::from("repeat") => Expression::builtin("repeat", |args, env| {
                    super::check_exact_args_len("repeat", &args, 2)?;

                    let s = match args[1].eval(env)? {
                        Expression::Symbol(x) | Expression::String(x) => x,
                        _ => return Err(LmError::CustomError("repeat requires a string as last argument".to_string())),
                    };

                    let count = match args[0].eval(env)? {
                        Expression::Integer(n) => n.max(0) as usize,
                        _ => return Err(LmError::CustomError("repeat requires an integer as count".to_string())),
                    };

                    Ok(Expression::String(s.repeat(count)))
                }, "repeat string specified number of times"),

                String::from("substring") => Expression::builtin("substring", |args, env| {
                    super::check_args_len("substring", &args, 2..3)?;

                    let (str_expr, start, end) = match args.len() {
                        2 => (args[1].clone(), args[0].clone(), None),
                        3 => (args[2].clone(), args[0].clone(), Some(args[1].clone())),
                        _ => unreachable!(),
                    };

                    let s = match str_expr.eval(env)? {
                        Expression::Symbol(x) | Expression::String(x) => x,
                        _ => return Err(LmError::CustomError("substring requires a string as last argument".to_string())),
                    };

                    let start_idx = match start.eval(env)? {
                        Expression::Integer(n) => {
                            if n < 0 {
                                (s.len() as i64 + n).max(0) as usize
                            } else {
                                n.min(s.len() as i64) as usize
                            }
                        },
                        _ => return Err(LmError::CustomError("substring requires integer indices".to_string())),
                    };

                    let end_idx = if let Some(end_expr) = end {
                        match end_expr.eval(env)? {
                            Expression::Integer(n) => {
                                if n < 0 {
                                    (s.len() as i64 + n).max(0) as usize
                                } else {
                                    n.min(s.len() as i64) as usize
                                }
                            },
                            _ => return Err(LmError::CustomError("substring requires integer indices".to_string())),
                        }
                    } else {
                        s.len()
                    };

                    if start_idx > end_idx || start_idx >= s.len() {
                        return Ok(Expression::String(String::new()));
                    }

                    let result = s.chars()
                        .skip(start_idx)
                        .take(end_idx - start_idx)
                        .collect();
                    Ok(Expression::String(result))
                }, "get substring from start to end indices"),

                String::from("split_lines") => Expression::builtin("split_lines", |args, env| {
                    super::check_exact_args_len("split_lines", &args, 1)?;

                    let s = match args[0].eval(env)? {
                        Expression::Symbol(x) | Expression::String(x) => x,
                        _ => return Err(LmError::CustomError("split_lines requires a string".to_string())),
                    };

                    let lines: Vec<Expression> = s.lines()
                        .map(|line| Expression::String(line.to_string()))
                        .collect();

                    Ok(Expression::from(lines))
                }, "split string into lines, removing line endings"),

                String::from("remove_prefix") => Expression::builtin("remove_prefix", |args, env| {
                    super::check_exact_args_len("remove_prefix", &args, 2)?;

                    let prefix = match args[0].eval(env)? {
                        Expression::Symbol(x) | Expression::String(x) => x,
                        _ => return Err(LmError::CustomError("remove_prefix requires string arguments".to_string())),
                    };

                    let s = match args[1].eval(env)? {
                        Expression::Symbol(x) | Expression::String(x) => x,
                        _ => return Err(LmError::CustomError("remove_prefix requires string arguments".to_string())),
                    };

                    Ok(Expression::String(
                        s.strip_prefix(&prefix).unwrap_or(&s).to_string()
                    ))
                }, "remove prefix if present"),

                String::from("remove_suffix") => Expression::builtin("remove_suffix", |args, env| {
                    super::check_exact_args_len("remove_suffix", &args, 2)?;

                    let suffix = match args[0].eval(env)? {
                        Expression::Symbol(x) | Expression::String(x) => x,
                        _ => return Err(LmError::CustomError("remove_suffix requires string arguments".to_string())),
                    };

                    let s = match args[1].eval(env)? {
                        Expression::Symbol(x) | Expression::String(x) => x,
                        _ => return Err(LmError::CustomError("remove_suffix requires string arguments".to_string())),
                    };

                    Ok(Expression::String(
                        s.strip_suffix(&suffix).unwrap_or(&s).to_string()
                    ))
                }, "remove suffix if present"),
    })
    .into()
}
