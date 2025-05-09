use crate::{Environment, Expression, LmError};
use common_macros::hash_map;

pub fn get() -> Expression {
    (hash_map! {
        String::from("strip") => Expression::builtin("strip", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(crate::repl::strip_ansi_escapes(args[0].eval(env)?).into())
        }, "strips all colors and styling from a string"),

        String::from("wrap") => Expression::builtin("wrap", wrap,
            "wrap text such that it fits in a specific number of columns"),

        String::from("href") => Expression::builtin("href", href,
            "create a hyperlink on the console"),

        String::from("bold") => Expression::builtin("bold", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[1m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to bold on the console"),

        String::from("faint") => Expression::builtin("faint", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[2m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to italics on the console"),

        String::from("italics") => Expression::builtin("italics", |args, env| {
        super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[3m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to italics on the console"),

        String::from("underline") => Expression::builtin("underline", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[4m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "underline text on the console"),

        String::from("blink") => Expression::builtin("blink", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[5m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "blink text on the console"),

        String::from("invert") => Expression::builtin("invert", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[7m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "invert text on the console"),

        String::from("strike") => Expression::builtin("strike", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[9m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "strike out text on the console"),

        String::from("black") => Expression::builtin("black", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[90m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to black on the console"),

        String::from("red") => Expression::builtin("red", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[91m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to red on the console"),

        String::from("green") => Expression::builtin("green", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[92m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to green on the console"),

        String::from("yellow") => Expression::builtin("yellow", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[93m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to yellow on the console"),

        String::from("blue") => Expression::builtin("blue", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[94m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to blue on the console"),

        String::from("magenta") => Expression::builtin("magenta", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[95m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to magenta on the console"),

        String::from("cyan") => Expression::builtin("cyan", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[96m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to cyan on the console"),

        String::from("white") => Expression::builtin("white", |args, env| {
            super::check_exact_args_len("strip", &args, 1)?;
            Ok(format!("\x1b[97m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
        }, "convert text to white on the console"),

        String::from("dark") => hash_map! {
            String::from("black") => Expression::builtin("black", |args, env| {
                super::check_exact_args_len("strip", &args, 1)?;
                Ok(format!("\x1b[30m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
            }, "convert text to black on the console"),

            String::from("red") => Expression::builtin("red", |args, env| {
                super::check_exact_args_len("strip", &args, 1)?;
                Ok(format!("\x1b[31m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
            }, "convert text to red on the console"),

            String::from("green") => Expression::builtin("green", |args, env| {
                super::check_exact_args_len("strip", &args, 1)?;
                Ok(format!("\x1b[32m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
            }, "convert text to green on the console"),

            String::from("yellow") => Expression::builtin("yellow", |args, env| {
                super::check_exact_args_len("strip", &args, 1)?;
                Ok(format!("\x1b[33m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
            }, "convert text to yellow on the console"),

            String::from("blue") => Expression::builtin("blue", |args, env| {
                super::check_exact_args_len("strip", &args, 1)?;
                Ok(format!("\x1b[34m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
            }, "convert text to blue on the console"),

            String::from("magenta") => Expression::builtin("magenta", |args, env| {
                super::check_exact_args_len("strip", &args, 1)?;
                Ok(format!("\x1b[35m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
            }, "convert text to magenta on the console"),

            String::from("cyan") => Expression::builtin("cyan", |args, env| {
                super::check_exact_args_len("strip", &args, 1)?;
                Ok(format!("\x1b[36m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
            }, "convert text to cyan on the console"),

            String::from("white") => Expression::builtin("white", |args, env| {
                super::check_exact_args_len("strip", &args, 1)?;
                Ok(format!("\x1b[37m{}\x1b[m\x6b[0m", args[0].eval(env)?).into())
            }, "convert text to white on the console"),

            String::from("pad_start") => Expression::builtin("pad_start", |args, env| {
                       super::check_args_len("pad_start", &args, 2..3)?;

                       let (str_expr, length, pad_char) = match args.len() {
                           2 => (args[1].clone(), args[0].clone(), " ".to_string()),
                           3 => (args[2].clone(), args[0].clone(), args[1].clone().to_string()),
                           _ => unreachable!(),
                       };

                       let s = match str_expr.eval(env)? {
                           Expression::Symbol(x) | Expression::String(x) => x,
                           _ => return Err(LmError::CustomError("pad_start requires a string as last argument".to_string())),
                       };

                       let len = match length.eval(env)? {
                           Expression::Integer(n) => n.max(0) as usize,
                           _ => return Err(LmError::CustomError("pad_start requires an integer as length".to_string())),
                       };

                       let pad_ch = pad_char.chars().next().unwrap_or(' ');

                       if s.len() >= len {
                           return Ok(Expression::String(s));
                       }

                       let pad_len = len - s.len();
                       let padding: String = std::iter::repeat_n(pad_ch, pad_len).collect();
                       Ok(Expression::String(format!("{}{}", padding, s)))
                   }, "pad string to specified length at start, with optional pad character"),

                   String::from("pad_end") => Expression::builtin("pad_end", |args, env| {
                       super::check_args_len("pad_end", &args, 2..3)?;

                       let (str_expr, length, pad_char) = match args.len() {
                           2 => (args[1].clone(), args[0].clone(), " ".to_string()),
                           3 => (args[2].clone(), args[0].clone(), args[1].clone().to_string()),
                           _ => unreachable!(),
                       };

                       let s = match str_expr.eval(env)? {
                           Expression::Symbol(x) | Expression::String(x) => x,
                           _ => return Err(LmError::CustomError("pad_end requires a string as last argument".to_string())),
                       };

                       let len = match length.eval(env)? {
                           Expression::Integer(n) => n.max(0) as usize,
                           _ => return Err(LmError::CustomError("pad_end requires an integer as length".to_string())),
                       };

                       let pad_ch = pad_char.chars().next().unwrap_or(' ');

                       if s.len() >= len {
                           return Ok(Expression::String(s));
                       }

                       let pad_len = len - s.len();
                       let padding: String = std::iter::repeat_n(pad_ch, pad_len).collect();
                       Ok(Expression::String(format!("{}{}", s, padding)))
                   }, "pad string to specified length at end, with optional pad character"),

                   String::from("center") => Expression::builtin("center", |args, env| {
                       super::check_args_len("center", &args, 2..3)?;

                       let (str_expr, length, pad_char) = match args.len() {
                           2 => (args[1].clone(), args[0].clone(), " ".to_string()),
                           3 => (args[2].clone(), args[0].clone(), args[1].clone().to_string()),
                           _ => unreachable!(),
                       };

                       let s = match str_expr.eval(env)? {
                           Expression::Symbol(x) | Expression::String(x) => x,
                           _ => return Err(LmError::CustomError("center requires a string as last argument".to_string())),
                       };

                       let len = match length.eval(env)? {
                           Expression::Integer(n) => n.max(0) as usize,
                           _ => return Err(LmError::CustomError("center requires an integer as length".to_string())),
                       };

                       if s.len() >= len {
                           return Ok(Expression::String(s));
                       }

                       let pad_ch = pad_char.chars().next().unwrap_or(' ');
                       let total_pad = len - s.len();
                       let left_pad = total_pad / 2;
                       let right_pad = total_pad - left_pad;

                       let left: String = std::iter::repeat_n(pad_ch, left_pad).collect();
                       let right: String = std::iter::repeat_n(pad_ch, right_pad).collect();
                       Ok(Expression::String(format!("{}{}{}", left, s, right)))
                   }, "center string by padding both ends"),

                   String::from("format") => Expression::builtin("format", |args, env| {
                               // format template arg1 arg2 ... argN
                               if args.is_empty() {
                                   return Err(LmError::CustomError("format requires at least a template string".to_string()));
                               }

                               let template = match args.last().unwrap().eval(env)? {
                                   Expression::Symbol(x) | Expression::String(x) => x,
                                   _ => return Err(LmError::CustomError("format requires string template as last argument".to_string())),
                               };

                               let placeholders = template.matches("{}").count();
                               if args.len() - 1 < placeholders {
                                   return Err(LmError::CustomError(format!(
                                       "format requires {} arguments for {} placeholders",
                                       placeholders, placeholders
                                   )));
                               }

                               let mut result = template.clone();
                               for arg in args.iter().take(placeholders) {
                                   let value = arg.eval(env)?;
                                   result = result.replacen("{}", &value.to_string(), 1);
                               }

                               Ok(Expression::String(result))
                           }, "format string using {} placeholders"),

        }.into()
    })
    .into()
}

fn wrap(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("wrap", &args, 2)?;
    match args[1].eval(env)? {
        Expression::Integer(columns) => {
            Ok(textwrap::fill(&args[0].eval(env)?.to_string(), columns as usize).into())
        }
        otherwise => Err(LmError::CustomError(format!(
            "expected number of columns in wrap, but got {}",
            otherwise
        ))),
    }
}

fn href(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("href", &args, 2)?;
    Ok(format!(
        "\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\",
        url = args[0].eval(env)?,
        text = args[1].eval(env)?
    )
    .into())
}
