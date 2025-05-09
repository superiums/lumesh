use std::collections::HashMap;

use crate::{Environment, Expression, Int, LmError, RuntimeError};
use common_macros::hash_map;

#[cfg(feature = "chess-engine")]
mod chess_module;
mod console_module;
mod dict_module;
mod err_module;
mod fmt_module;
// mod fn_module;
// use fn_module::curry;
mod fs_module;
mod list_module;
use list_module::*;
mod log_module;
mod math_module;
// mod operator_module;
mod os_module;
mod parse_module;
mod rand_module;
mod regex_module;
mod shell_module;
mod string_module;
mod sys_module;
mod time_module;
mod widget_module;

pub fn get_module_map() -> HashMap<String, Expression> {
    hash_map! {
      String::from("log") => log_module::get(),
        String::from("math") => math_module::get(),
        String::from("dict") => dict_module::get(),
        String::from("version") => shell_module::get(),
        String::from("err") => err_module::get(),
        String::from("os") => os_module::get(),
        String::from("widget") => widget_module::get(),
        String::from("time") => time_module::get(),
        String::from("rand") => rand_module::get(),
        // String::from("fn") => fn_module::get(),
        String::from("console") => console_module::get(),
        String::from("fmt") => fmt_module::get(),
        String::from("parse") => parse_module::get(),
        String::from("fs") => fs_module::get(),
            String::from("string") => string_module::get(),
            String::from("regex") => regex_module::get(),
            String::from("list") => list_module::get(),
            String::from("sys") => sys_module::get(),
            String::from("exit") => Expression::builtin(
                "exit",
                exit,
                "exit the shell",
            ),
            String::from("cd") => Expression::builtin("cd", cd, "change directories"),
            String::from("print") => Expression::builtin("print", print,"print the arguments without a newline"),
            String::from("println") => Expression::builtin("println", println, "print the arguments and a newline"),
            String::from("eprint") => Expression::builtin("eprintln", eprint, "print to stderr"),
            String::from("eprintln") => Expression::builtin("eprintln", eprintln, "print to stderr"),
            String::from("debug") => Expression::builtin("debug", debug, "print the debug representation of the arguments and a newline"),
            String::from("input") => Expression::builtin("input", input, "get user input"),

            String::from("type") => Expression::builtin("type", get_type, "get type of data"),
            String::from("str") => Expression::builtin("str", str, "format an expression to a string"),
            String::from("int") => Expression::builtin("int", int, "convert a float or string to an int"),
            String::from("insert") => Expression::builtin("insert", insert, "insert an item into a dictionary or list"),
            String::from("len") => Expression::builtin("len", len, "get the length of an expression"),
            // String::from("chars") => Expression::builtin("chars", chars, "aaa"),
            // String::from("head") => Expression::builtin("head", head, "aaa"),
            // String::from("tail") => Expression::builtin("tail", tail, "aaa"),
            // String::from("lines") => Expression::builtin("lines", lines, "get the list of lines in a string"),
            String::from("eval") => Expression::builtin("eval", eval, "evaluate an expression without changing the environment"),
            String::from("exec") => Expression::builtin("exec", exec, "evaluate an expression in the current environment"),
            // String::from("unbind") => Expression::builtin("unbind", unbind, "unbind a variable from the environment"),
            String::from("report") => Expression::builtin("report", report, "default function for reporting values"),

            String::from("include") => Expression::builtin("include", include, "evaluate a file in the current environment"),

            String::from("import") => Expression::builtin("import", import, "import a file (evaluate it in a new environment)"),


    }
}
fn import(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("import", &args, 1)?;
    let cwd = std::env::current_dir()?;
    let path = cwd.join(args[0].eval(env)?.to_string());

    if let Ok(canon_path) = dunce::canonicalize(&path) {
        // Read the file.
        let contents = std::fs::read_to_string(canon_path.clone()).map_err(|e| {
            LmError::CustomError(format!(
                "could not read file {}: {}",
                canon_path.display(),
                e
            ))
        })?;
        // Evaluate the file.
        if let Ok(expr) = crate::parse(&contents) {
            let mut new_env = env.clone();
            Ok(expr.eval(&mut new_env)?)
        } else {
            Err(LmError::CustomError(format!(
                "could not parse file {}",
                canon_path.display()
            )))
        }
    } else {
        Err(LmError::CustomError(format!(
            "could not canonicalize path {}",
            path.display()
        )))
    }
}
fn include(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("include", &args, 1)?;

    let cwd = std::env::current_dir()?;
    let path = cwd.join(args[0].eval(env)?.to_string());

    if let Ok(canon_path) = dunce::canonicalize(&path) {
        // Read the file.
        let contents = std::fs::read_to_string(canon_path.clone()).map_err(|e| {
            LmError::CustomError(format!(
                "could not read file {}: {}",
                canon_path.display(),
                e
            ))
        })?;
        // Evaluate the file.
        if let Ok(expr) = crate::parse(&contents) {
            Ok(expr.eval(env)?)
        } else {
            Err(LmError::CustomError(format!(
                "could not parse file {}",
                canon_path.display()
            )))
        }
    } else {
        Err(LmError::CustomError(format!(
            "could not canonicalize path {}",
            path.display()
        )))
    }
}
fn exit(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    if args.is_empty() {
        std::process::exit(0);
    } else if let Expression::Integer(n) = args[0].eval(env)? {
        std::process::exit(n as i32);
    } else {
        Err(LmError::CustomError(format!(
            "expected integer but got `{:?}`",
            args[0]
        )))
    }
}
fn cd(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("cd", &args, 1)?;

    match args[0].eval(env)? {
        Expression::Symbol(path) | Expression::String(path) => {
            std::env::set_current_dir(&path).map_err(|e| {
                crate::LmError::CustomError(match format!("{:?}", e.kind()).as_str() {
                    "PermissionDenied" => {
                        format!("you don't have permission to read directory {:?}", &path)
                    }
                    "NotADirectory" => {
                        format!("{:?} is not a directory", &path)
                    }
                    _ => format!("could not change directory to {:?}\n  reason: {}", &path, e),
                })
            })?;

            // env.set_cwd(new_cwd.into_os_string().into_string().unwrap());
            Ok(Expression::None)
        }

        other => {
            // Try to convert the argument to a string
            let path = other.to_string();
            cd(vec![Expression::String(path)], env)
        }
    }
}

fn get_type(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("type", &args, 1)?;
    let x_type = args[0].type_name();
    let rs = if &x_type == "Symbol" {
        args[0].eval(env)?.type_name()
    } else {
        x_type
    };
    Ok(Expression::String(rs))
}
fn print(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut result: Vec<Expression> = Vec::with_capacity(args.len());

    for (i, arg) in args.iter().enumerate() {
        let x = arg.clone().eval(env)?;
        if i < args.len() - 1 {
            print!("{} ", x)
        } else {
            println!("{}", x)
        }
        result.push(x)
    }
    if result.len() == 1 {
        return Ok(result[0].clone());
    }
    Ok(Expression::from(result))
}

fn debug(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    for (i, arg) in args.iter().enumerate() {
        let x = arg.clone().eval(env)?;
        if i < args.len() - 1 {
            print!("{:?} ", x)
        } else {
            println!("{:?}", x)
        }
    }

    Ok(Expression::None)
}

fn println(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut result: Vec<Expression> = Vec::with_capacity(args.len());
    for (_, arg) in args.iter().enumerate() {
        let x = arg.clone().eval(env)?;
        println!("{}", x);
        result.push(x);
    }
    if result.len() == 1 {
        return Ok(result[0].clone());
    }
    Ok(Expression::from(result))
}

fn eprint(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut result: Vec<Expression> = Vec::with_capacity(args.len());

    for (i, arg) in args.iter().enumerate() {
        let x = arg.clone().eval(env)?;
        if i < args.len() - 1 {
            eprint!("\x1b[38;5;9m{} \x1b[m\x1b[0m", x)
        } else {
            eprintln!("\x1b[38;5;9m{}\x1b[m\x1b[0m", x)
        }
        result.push(x)
    }
    if result.len() == 1 {
        return Ok(result[0].clone());
    }
    Ok(Expression::from(result))
}
fn eprintln(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut result: Vec<Expression> = Vec::with_capacity(args.len());
    for (_, arg) in args.iter().enumerate() {
        let x = arg.clone().eval(env)?;
        eprintln!("\x1b[38;5;9m{}\x1b[m\x1b[0m", x);
        result.push(x);
    }
    if result.len() == 1 {
        return Ok(result[0].clone());
    }
    Ok(Expression::from(result))
}

fn input(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut prompt = String::new();
    for (i, arg) in args.iter().enumerate() {
        let x = arg.clone().eval(env)?;
        if i < args.len() - 1 {
            prompt += &format!("{} ", x)
        } else {
            prompt += &format!("{}", x)
        }
    }
    Ok(Expression::String(crate::repl::read_user_input(&prompt)))
}

fn str(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    Ok(Expression::String(args[0].eval(env)?.to_string()))
}

fn int(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    match args[0].eval(env)? {
        Expression::Integer(x) => Ok(Expression::Integer(x)),
        Expression::Float(x) => Ok(Expression::Integer(x as Int)),
        Expression::String(x) => {
            if let Ok(n) = x.parse::<Int>() {
                Ok(Expression::Integer(n))
            } else {
                Err(LmError::CustomError(format!(
                    "could not convert {:?} to an integer",
                    x
                )))
            }
        }
        otherwise => Err(LmError::CustomError(format!(
            "could not convert {:?} to an integer",
            otherwise
        ))),
    }
}
fn insert(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("insert", &args, 3)?;
    let mut arr = args[0].eval(env)?;
    let idx = args[1].eval(env)?;
    let val = args[2].eval(env)?;
    return match (&mut arr, &idx) {
        (Expression::Map(exprs), Expression::String(key)) => {
            let mut result = exprs.as_ref().clone();
            result.insert(key.clone(), val);
            Ok(Expression::from(result))
        }
        (Expression::List(exprs), Expression::Integer(i)) => {
            if *i as usize <= exprs.as_ref().len() {
                let mut result = exprs.as_ref().clone();
                result.insert(*i as usize, val);
                Ok(Expression::from(result))
            } else {
                return Err(LmError::CustomError(format!(
                    "index {} out of bounds for {:?}",
                    idx, arr
                )));
            }
        }
        (Expression::String(s), Expression::Integer(i)) => {
            if *i as usize <= s.len() {
                s.insert_str(*i as usize, &val.to_string());
                Ok(Expression::String(s.clone()))
            } else {
                return Err(LmError::CustomError(format!(
                    "index {} out of bounds for {:?}",
                    idx, arr
                )));
            }
        }
        _ => {
            return Err(LmError::CustomError(format!(
                "cannot insert {:?} into {:?} with index {:?}",
                val, arr, idx
            )));
        }
    };

    // Ok(arr)
}

fn len(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("len", &args, 1)?;
    match args[0].eval(env)? {
        Expression::Map(m) => Ok(Expression::Integer(m.as_ref().len() as Int)),
        Expression::List(list) => Ok(Expression::Integer(list.as_ref().len() as Int)),
        Expression::Symbol(x) | Expression::String(x) => {
            Ok(Expression::Integer(x.chars().count() as Int))
        }
        Expression::Bytes(bytes) => Ok(Expression::Integer(bytes.len() as Int)),

        otherwise => Err(LmError::CustomError(format!(
            "cannot get length of {}",
            otherwise
        ))),
    }
}

// fn lines(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
//     match args[0].eval(env)? {
//         Expression::String(x) => Ok(Expression::List(
//             x.lines()
//                 .map(|ch| Expression::String(ch.to_string()))
//                 .collect::<Vec<Expression>>(),
//         )),
//         otherwise => Err(LmError::CustomError(format!(
//             "cannot get lines of non-string {}",
//             otherwise
//         ))),
//     }
// }

fn eval(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut new_env = env.clone();
    Ok(args[0].eval(env)?.eval(&mut new_env)?)
}

fn exec(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    Ok(args[0].eval(env)?.eval(env)?)
}

fn report(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let val = args[0].eval(env)?;
    match val {
        Expression::Map(_) => println!("{}", val),
        Expression::String(s) => println!("{}", s),
        Expression::None => {}
        otherwise => println!("{}", otherwise),
    }

    Ok(Expression::None)
}

fn check_args_len(
    name: impl ToString,
    args: &[Expression],
    expected_len: impl std::ops::RangeBounds<usize>,
) -> Result<(), LmError> {
    if expected_len.contains(&args.len()) {
        Ok(())
    } else {
        Err(LmError::CustomError(format!(
            "too few arguments to function {}",
            name.to_string()
        )))
    }
}

fn check_exact_args_len(
    name: impl ToString,
    args: &[Expression],
    expected_len: usize,
) -> Result<(), RuntimeError> {
    if args.len() == expected_len {
        Ok(())
    } else {
        // SyntaxError::new(
        //     "",
        //     lumesh::SyntaxErrorKind::ArgumentMismatch {
        //         name: name,
        //         expected: expected_len,
        //         received: args.len(),
        //     },
        // )
        Err(RuntimeError::ArgumentMismatch {
            name: name.to_string(),
            expected: expected_len,
            received: args.len(),
        })

        // Err(RuntimeError::ArgumentMismatch(if args.len() > expected_len {
        //     format!("too many arguments to function {}", name.to_string())
        // } else {
        //     format!("too few arguments to function {}", name.to_string())
        // }))
    }
}
