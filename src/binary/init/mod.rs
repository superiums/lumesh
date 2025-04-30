use std::collections::HashMap;

use crate::{Environment, Expression, Int, LmError, RuntimeError};
use common_macros::hash_map;

#[cfg(feature = "chess-engine")]
mod chess_module;
mod console_module;
mod dict_module;
mod err_module;
mod fmt_module;
mod fn_module;
use fn_module::curry;
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
    // TODO tick out env
    let mut env = Environment::new();
    let fs = fs_module::get(&mut env);
    // let ops = operator_module::get(env);
    let math = math_module::get(&mut env);
    let standard_module = hash_map! {
      String::from("log") => log_module::get(),
        String::from("math") => math,
        String::from("dict") => dict_module::get(),
        String::from("version") => shell_module::get(),
        String::from("err") => err_module::get(),
        String::from("os") => os_module::get(),
        String::from("widget") => widget_module::get(),
        String::from("time") => time_module::get(),
        String::from("rand") => rand_module::get(),
        String::from("fn") => fn_module::get(),
        String::from("console") => console_module::get(),
        String::from("fmt") => fmt_module::get(),
        String::from("parse") => parse_module::get(),
        String::from("fs") => fs,
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
            String::from("debug") => Expression::builtin("debug", debug, "print the debug representation of the arguments and a newline"),
            String::from("println") => Expression::builtin("println", println, "print the arguments and a newline"),
            String::from("input") => Expression::builtin("input", input, "get user input"),
            String::from("str") => Expression::builtin("str", str, "format an expression to a string"),
            String::from("int") => Expression::builtin("int", int, "convert a float or string to an int"),
            String::from("insert") => Expression::builtin("insert", insert, "insert an item into a dictionary or list"),
            String::from("keys") => Expression::builtin("keys", keys, "get the list of keys in a table"),
            String::from("vals") => Expression::builtin("vals", vals, "get the list of values in a table"),
            String::from("vars") => Expression::builtin("vars", vars, "get a table of the defined variables"),
            String::from("len") => Expression::builtin("len", len, "get the length of an expression"),
            // String::from("chars") => Expression::builtin("chars", chars, "aaa"),
            // String::from("head") => Expression::builtin("head", head, "aaa"),
            // String::from("tail") => Expression::builtin("tail", tail, "aaa"),
            String::from("lines") => Expression::builtin("lines", lines, "get the list of lines in a string"),
            String::from("eval") => Expression::builtin("eval", eval, "evaluate an expression without changing the environment"),
            String::from("exec") => Expression::builtin("exec", exec, "evaluate an expression in the current environment"),
            // String::from("unbind") => Expression::builtin("unbind", unbind, "unbind a variable from the environment"),
            String::from("report") => Expression::builtin("report", report, "default function for reporting values"),



    };

    return standard_module;
}
fn exit(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    if args.is_empty() {
        std::process::exit(0);
    } else if let Expression::Integer(n) = args[0].clone().eval(env)? {
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

fn print(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    for (i, arg) in args.iter().enumerate() {
        let x = arg.clone().eval(env)?;
        if i < args.len() - 1 {
            print!("{} ", x)
        } else {
            print!("{}", x)
        }
    }

    Ok(Expression::None)
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
    for (i, arg) in args.iter().enumerate() {
        let x = arg.clone().eval(env)?;
        if i < args.len() - 1 {
            print!("{} ", x)
        } else {
            println!("{}", x)
        }
    }

    Ok(Expression::None)
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
    match (&mut arr, &idx) {
        (Expression::Map(exprs), Expression::String(key)) => {
            exprs.insert(key.clone(), val);
        }
        (Expression::List(exprs), Expression::Integer(i)) => {
            if *i as usize <= exprs.len() {
                exprs.insert(*i as usize, val);
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
    }

    Ok(arr)
}

fn keys(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    match args[0].eval(env)? {
        Expression::Map(m) => Ok(m.into_keys().collect::<Vec<_>>().into()),
        otherwise => Err(LmError::CustomError(format!(
            "cannot get the keys of {}",
            otherwise
        ))),
    }
}

fn vals(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    match args[0].eval(env)? {
        Expression::Map(m) => Ok(m.into_values().collect::<Vec<_>>().into()),
        otherwise => Err(LmError::CustomError(format!(
            "cannot get the values of {}",
            otherwise
        ))),
    }
}

fn vars(_: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    Ok(env.bindings.clone().into())
}

fn len(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    match args[0].eval(env)? {
        Expression::Map(m) => Ok(Expression::Integer(m.len() as Int)),
        Expression::List(list) => Ok(Expression::Integer(list.len() as Int)),
        Expression::Symbol(x) | Expression::String(x) => {
            Ok(Expression::Integer(x.chars().count() as Int))
        }
        otherwise => Err(LmError::CustomError(format!(
            "cannot get length of {}",
            otherwise
        ))),
    }
}

fn lines(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    match args[0].eval(env)? {
        Expression::String(x) => Ok(Expression::List(
            x.lines()
                .map(|ch| Expression::String(ch.to_string()))
                .collect::<Vec<Expression>>(),
        )),
        otherwise => Err(LmError::CustomError(format!(
            "cannot get lines of non-string {}",
            otherwise
        ))),
    }
}

fn eval(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut new_env = env.clone();
    Ok(args[0].clone().eval(env)?.eval(&mut new_env)?)
}

fn exec(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    Ok(args[0].clone().eval(env)?.eval(env)?)
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
