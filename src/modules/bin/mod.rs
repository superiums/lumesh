use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
};

use crate::{Environment, Expression, Int, LmError, RuntimeError, parse_and_eval};
use common_macros::hash_map;
use pprint::pretty_printer;

#[cfg(feature = "chess-engine")]
mod chess_module;
mod console_module;
mod fmt_module;
mod map_module;
// mod fn_module;
// use fn_module::curry;
mod fs_module;
mod list_module;
mod log_module;
mod math_module;
// mod operator_module;
mod about_module;
mod fs_ls;
mod into_module;
mod os_module;
mod parse_module;
mod pprint;
mod rand_module;
mod regex_module;
mod string_module;
mod sys_module;
mod time_module;
mod ui_module;
mod widget_module;

pub fn get_module_map() -> HashMap<String, Expression> {
    hash_map! {
        String::from("log") => log_module::get(),
        String::from("math") => math_module::get(),
        String::from("map") => map_module::get(),
        String::from("about") => about_module::get(),
        // String::from("err") => err_module::get(),
        String::from("os") => os_module::get(),
        String::from("ui") => ui_module::get(),
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
        String::from("into") => into_module::get(),

        // console control
        // Shell control
                String::from("exit") => Expression::builtin("exit", exit, "exit the shell", "[status]"),
                String::from("cd") => Expression::builtin("cd", cd, "change directories", "[path]"),
                String::from("pwd") => Expression::builtin("pwd", pwd, "print working directory", ""),

                // I/O operations
                String::from("tap") => Expression::builtin("tap", tap, "print and return result", "<args>..."),
                String::from("print") => Expression::builtin("print", print, "print arguments without newline", "<args>..."),
                String::from("pprint") => Expression::builtin("pprint", pretty_print, "pretty print", "<list>|<map>"),
                String::from("println") => Expression::builtin("println", println, "print arguments with newline", "<args>..."),
                String::from("eprint") => Expression::builtin("eprint", eprint, "print to stderr without newline", "<args>..."),
                String::from("eprintln") => Expression::builtin("eprintln", eprintln, "print to stderr with newline", "<args>..."),
                String::from("debug") => Expression::builtin("debug", debug, "print debug representation", "<args>..."),
                String::from("report") => Expression::builtin("report", report, "default value reporting", "<value>"),
                String::from("read") => Expression::builtin("read", read, "get user input", "[prompt]"),

                // Data manipulation
                String::from("get") => Expression::builtin("get", get, "get value from nested map/list/range using dot notation path", "<path> <map|list|range>"),
                String::from("type") => Expression::builtin("type", get_type, "get data type", "<value>"),
                String::from("len") => Expression::builtin("len", len, "get length of expression", "<collection>"),
                String::from("insert") => Expression::builtin("insert", insert, "insert item into collection", "<key/index> <value> <collection>"),
                String::from("rev") => Expression::builtin("rev", rev, "reverse sequence", "<string|list|bytes>"),
                String::from("flatten") => Expression::builtin("flatten", flatten_wrapper, "flatten nested structure", "<collection>"),
                String::from("where") => Expression::builtin("where", filter_rows, "filter rows by condition", "<condition> <list[map]> "),
                String::from("select") => Expression::builtin("select", select_columns, "select columns from list of maps", "<columns>...<list[map]>"),

                // Execution control
                String::from("repeat") => Expression::builtin("repeat", repeat, "evaluate without env change", "<expr>"),
                String::from("eval") => Expression::builtin("eval", eval, "evaluate expression", "<expr>"),
                String::from("exec_str") => Expression::builtin("exec_str", exec_str, "evaluate string", "<string>"),
                String::from("exec") => Expression::builtin("exec", exec, "evaluate in current env", "<expr>"),
                String::from("include") => Expression::builtin("include", include, "evaluate file in current env", "<path>"),
                String::from("import") => Expression::builtin("import", import, "evaluate file in new env", "<path>"),

                // Help system
                String::from("help") => Expression::builtin("help", help, "display lib modules", "[module]")
    }
}
fn help(args: &Vec<Expression>, _: &mut Environment) -> Result<Expression, crate::LmError> {
    if !args.is_empty() {
        match super::get_builtin(&args[0].to_string()) {
            Some(m) => {
                pretty_printer(m)?;
            }
            _ => return Err(LmError::CustomError("no lib found".into())),
        }
    } else {
        let m = super::get_builtin_map()
            .iter()
            .map(|item| match item.1 {
                Expression::HMap(_) => (item.0.clone(), Expression::String("module".to_string())),
                Expression::Map(_) => (item.0.clone(), Expression::String("Module".to_string())),
                other => (item.0.clone(), other.clone()),
            })
            .collect::<HashMap<String, Expression>>();
        pretty_printer(&Expression::from(m))?;
    }
    Ok(Expression::None)
}
fn import(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("import", args, 1)?;
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
fn include(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("include", args, 1)?;

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
fn exit(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    if args.is_empty() {
        std::process::exit(0);
    } else if let Expression::Integer(n) = args[0].eval(env)? {
        std::process::exit(n as i32);
    } else {
        Err(LmError::TypeError {
            expected: "Integer".to_string(),
            found: args[0].type_name(),
            sym: args[0].to_string(),
        })
    }
}
fn cd(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("cd", args, 1)?;

    match args[0].eval(env)? {
        Expression::Symbol(mut path) | Expression::String(mut path) => {
            if path.starts_with("~") {
                if let Some(home_dir) = dirs::home_dir() {
                    path = path.replace("~", home_dir.to_string_lossy().as_ref());
                }
            }
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
            cd(&vec![Expression::String(path)], env)
        }
    }
}
fn pwd(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, crate::LmError> {
    let path = std::env::current_dir()?;
    // println!("{}", path.display());
    Ok(Expression::String(path.to_string_lossy().into_owned()))
}

fn get_type(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("type", args, 1)?;
    let x_type = args[0].type_name();
    let rs = if &x_type == "Symbol" {
        args[0].eval(env)?.type_name()
    } else {
        x_type
    };
    Ok(Expression::String(rs))
}

fn debug(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    for (i, arg) in args.iter().enumerate() {
        let x = arg.eval(env)?;
        if i < args.len() - 1 {
            print!("{:?} ", x)
        } else {
            println!("{:?}", x)
        }
    }
    Ok(Expression::None)
}

fn tap(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut stdout = std::io::stdout().lock();
    let mut result: Vec<Expression> = Vec::with_capacity(args.len());
    for (i, arg) in args.iter().enumerate() {
        let x = arg.eval(env)?;
        if i < args.len() - 1 {
            write!(&mut stdout, "{} ", x)?;
        } else {
            writeln!(&mut stdout, "{}", x)?;
        }
        result.push(x)
    }
    stdout.flush()?;
    if result.len() == 1 {
        return Ok(result[0].clone());
    }
    Ok(Expression::from(result))
}

fn print(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut stdout = std::io::stdout().lock();
    for arg in args.iter() {
        let x = arg.eval(env)?;
        write!(&mut stdout, "{} ", x)?;
    }
    writeln!(&mut stdout, "")?;
    stdout.flush()?;
    Ok(Expression::None)
}
fn println(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut stdout = std::io::stdout().lock();
    for arg in args.iter() {
        let x = arg.eval(env)?;
        // println!("{}", x);
        writeln!(&mut stdout, "{}", x)?;
    }
    stdout.flush()?;
    Ok(Expression::None)
}
fn pretty_print(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    check_args_len("pprint", args, 1..)?;
    // let _ = args.iter().map(|a| pretty_printer(&a.eval(env)?));
    for arg in args.iter() {
        let r = arg.eval(env)?;
        pretty_printer(&r)?;
    }
    Ok(Expression::None)
}
fn eprint(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut stderr = std::io::stderr().lock();
    for (i, arg) in args.iter().enumerate() {
        let x = arg.eval(env)?;
        if i < args.len() - 1 {
            write!(&mut stderr, "\x1b[38;5;9m{} \x1b[m\x1b[0m", x)?;
        } else {
            writeln!(&mut stderr, "\x1b[38;5;9m{}\x1b[m\x1b[0m", x)?;
        }
    }
    stderr.flush()?;
    Ok(Expression::None)
}
fn eprintln(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    let mut stderr = std::io::stderr().lock();
    for arg in args.iter() {
        let x = arg.eval(env)?;
        writeln!(&mut stderr, "\x1b[38;5;9m{}\x1b[m\x1b[0m", x)?;
    }
    stderr.flush()?;
    Ok(Expression::None)
}

fn read(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    print(args, env)?;
    let _ = std::io::stdout().flush();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(Expression::String(input.trim().to_owned()))
    // let mut prompt = String::new();
    // for (i, arg) in args.iter().enumerate() {
    //     let x = arg.eval(env)?;
    //     if i < args.len() - 1 {
    //         prompt += &format!("{} ", x)
    //     } else {
    //         prompt += &format!("{}", x)
    //     }
    // }
    // Ok(Expression::String(crate::repl::read_user_input(&prompt)))
}

fn insert(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("insert", args, 3)?;
    let mut arr = args[2].eval(env)?;
    let idx = args[0].eval(env)?;
    let val = args[1].eval(env)?;
    match (&mut arr, &idx) {
        (Expression::HMap(exprs), Expression::String(key)) => {
            let mut result = exprs.as_ref().clone();
            result.insert(key.clone(), val);
            Ok(Expression::from(result))
        }
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
                Err(LmError::CustomError(format!(
                    "index {} out of bounds for {:?}",
                    idx, arr
                )))
            }
        }
        (Expression::String(s), Expression::Integer(i)) => {
            if *i as usize <= s.len() {
                s.insert_str(*i as usize, &val.to_string());
                Ok(Expression::String(s.clone()))
            } else {
                Err(LmError::CustomError(format!(
                    "index {} out of bounds for {:?}",
                    idx, arr
                )))
            }
        }
        _ => Err(LmError::CustomError(format!(
            "cannot insert {:?} into {:?} with index {:?}",
            val, arr, idx
        ))),
    }

    // Ok(arr)
}

fn len(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("len", args, 1)?;
    match args[0].eval(env)? {
        Expression::HMap(m) => Ok(Expression::Integer(m.as_ref().len() as Int)),
        Expression::Map(m) => Ok(Expression::Integer(m.as_ref().len() as Int)),
        Expression::List(list) => Ok(Expression::Integer(list.as_ref().len() as Int)),
        Expression::Symbol(x) | Expression::String(x) => {
            Ok(Expression::Integer(x.chars().count() as Int))
        }
        Expression::Bytes(bytes) => Ok(Expression::Integer(bytes.len() as Int)),

        otherwise => Err(LmError::CustomError(format!(
            "cannot get length of {}:{}",
            otherwise,
            otherwise.type_name()
        ))),
    }
}

fn rev(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("rev", args, 1)?;
    match args[0].eval(env)? {
        Expression::List(list) => {
            let mut reversed = list.as_ref().to_vec();
            reversed.reverse();
            Ok(Expression::from(reversed))
        }
        Expression::String(s) => Ok(Expression::String(s.chars().rev().collect())),
        Expression::Symbol(s) => Ok(Expression::Symbol(s.chars().rev().collect())),
        Expression::Bytes(b) => Ok(Expression::Bytes(b.into_iter().rev().collect())),
        _ => Err(LmError::CustomError(
            "rev requires list or string as argument".to_string(),
        )),
    }
}

fn exec_str(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("exec_str", args, 1)?;
    match &args[0] {
        Expression::String(cmd) => {
            if !cmd.is_empty() {
                println!("\n  >> Excuting: \x1b[38;5;208m\x1b[1m{}\x1b[m\x1b[0m", cmd);
                parse_and_eval(cmd, env);
            }
            Ok(Expression::None)
        }
        _ => Err(LmError::CustomError(
            "only String acceptable to exec_str".to_owned(),
        )),
    }
}
fn repeat(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("repeat", args, 2)?;
    let n = get_integer_arg(args[0].eval(env)?)?;
    let r = (0..n)
        .map(|_| args[1].eval(env))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expression::from(r))
}
fn eval(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("eval", args, 1)?;
    let mut new_env = env.clone();
    Ok(args[0].eval(env)?.eval(&mut new_env)?)
}

fn exec(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("exec", args, 1)?;
    Ok(args[0].eval(env)?.eval(env)?)
}

fn report(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("report", args, 1)?;
    let val = args[0].eval(env)?;
    match val {
        // Expression::HMap(_) => println!("{}", val),
        // Expression::Map(_) => println!("{}", val),
        // Expression::String(s) => println!("{}", s),
        Expression::None => {}
        otherwise => println!("{}", otherwise),
    }

    Ok(Expression::None)
}

fn flatten(expr: Expression) -> Vec<Expression> {
    match expr {
        Expression::List(list) => list
            .as_ref()
            .iter()
            .flat_map(|item| flatten(item.clone()))
            .collect(),
        Expression::HMap(map) => map
            .as_ref()
            .values()
            .flat_map(|item| flatten(item.clone()))
            .collect(),
        Expression::Map(map) => map
            .as_ref()
            .values()
            .flat_map(|item| flatten(item.clone()))
            .collect(),
        expr => vec![expr],
    }
}

fn flatten_wrapper(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("flatten", args, 1)?;
    Ok(Expression::from(flatten(args[0].eval(env)?)))
}

fn filter_rows(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    // dbg!(&args);
    check_exact_args_len("where", args, 2)?;

    let data = if let Expression::List(list) = args[1].eval(env)? {
        list
    } else {
        return Err(LmError::CustomError("Expected list for filtering".into()));
    };

    let mut filtered = Vec::new();

    let mut row_env = Environment::new();
    row_env.define("LINES", Expression::Integer(data.len() as i64));
    for (i, row) in data.as_ref().iter().enumerate() {
        row_env.define("LINENO", Expression::Integer(i as i64));

        // dbg!(row_env.get("LINENO"));
        if let Expression::HMap(row_map) = row {
            for (k, v) in row_map.as_ref() {
                row_env.define(k, v.clone());
            }
            if let Expression::Boolean(true) = args[0].eval(&mut row_env)? {
                filtered.push(row.clone());
            }
        } else if let Expression::Map(row_map) = row {
            for (k, v) in row_map.as_ref() {
                row_env.define(k, v.clone());
            }

            let c = args[0].eval(&mut row_env)?;
            // dbg!(&c);
            if let Expression::Boolean(true) = c {
                filtered.push(row.clone());
            }
        }
    }

    Ok(Expression::from(filtered))
}

fn select_columns(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    // check_exact_args_len("select", args, 2)?;

    let headers = match args.len() {
        3.. => args[..args.len() - 1]
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>(),
        2 => {
            let a = args[0].eval(env)?;
            if let Expression::List(list) = a {
                list.as_ref()
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
            } else {
                vec![a.to_string()]
            }
        }
        0..2 => {
            return Err(LmError::CustomError(
                "select required 2 or more args".into(),
            ));
        }
    };

    let data = if let Expression::List(list) = args.last().unwrap().eval(env)? {
        list
    } else {
        return Err(LmError::CustomError(
            "Expected list for column selection".into(),
        ));
    };

    // dbg!(&data, &headers);
    let result = data
        .as_ref()
        .iter()
        .filter_map(|row| {
            // dbg!(&row, &row.type_name());
            if let Expression::Map(row_map) = row {
                // dbg!(&row_map);
                let selected = headers
                    .iter()
                    .filter_map(|col| {
                        // dbg!(&col, &row_map.get(col));
                        row_map
                            .as_ref()
                            .get(col)
                            .map(|val| (col.clone(), val.clone()))
                    })
                    .collect::<BTreeMap<_, _>>();

                Some(Expression::from(selected))
            } else {
                // dbg!("Not Map");
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(Expression::from(result))
}

fn get(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("get", args, 2)?;

    let index = args[0].eval(env)?;
    let mut current = args[1].eval(env)?;

    // let path = get_string_arg(index)?;
    let path = match index {
        Expression::Symbol(s) | Expression::String(s) => s,
        Expression::Integer(i) => i.to_string(),
        _ => {
            return Err(LmError::TypeError {
                expected: "symbol/string/integer as path".to_owned(),
                found: index.type_name(),
                sym: index.to_string(),
            });
        }
    };
    let path_segments: Vec<&str> = path.split('.').collect();
    if path_segments.is_empty() {
        return Ok(current);
    }

    for segment in path_segments {
        match current {
            Expression::Map(m) => {
                current = m
                    .as_ref()
                    .get(segment)
                    .ok_or_else(|| {
                        LmError::CustomError(format!(
                            "path segment '{}' not found in Map `{:?}`",
                            segment,
                            m.as_ref()
                        ))
                    })?
                    .clone();
            }
            Expression::List(m) => match segment.parse::<usize>() {
                Ok(key) => {
                    current = m
                        .as_ref()
                        .get(key)
                        .ok_or_else(|| {
                            LmError::CustomError(format!(
                                "path index '{}' not found in List `{:?}`",
                                segment,
                                m.as_ref()
                            ))
                        })?
                        .clone();
                }
                _ => {
                    return Err(LmError::CustomError(format!(
                        "path index '{}' is not valid for List",
                        segment
                    )));
                }
            },
            Expression::Range(m, step) => match segment.parse::<usize>() {
                Ok(key) => {
                    current = m
                        .step_by(step)
                        .skip(key)
                        .next()
                        .map(Expression::Integer)
                        .ok_or_else(|| {
                            LmError::CustomError(format!(
                                "path index '{}' not found in Range",
                                segment
                            ))
                        })?
                        .clone();
                }
                _ => {
                    return Err(LmError::CustomError(format!(
                        "path index '{}' is not valid for Range",
                        segment
                    )));
                }
            },
            _ => {
                return Err(LmError::CustomError(format!(
                    "path segment '{}' attempt to access on non-indexable type: {}",
                    segment,
                    current.type_name()
                )));
            }
        }
    }

    Ok(current)

    // _ => {
    //     return Err(LmError::CustomError(
    //         "get requires a map as last argument".to_string(),
    //     ));
    // }
}

// Helper functions

fn check_args_len(
    name: impl ToString,
    args: &[Expression],
    expected_len: impl std::ops::RangeBounds<usize>,
) -> Result<(), LmError> {
    if expected_len.contains(&args.len()) {
        Ok(())
    } else {
        Err(LmError::CustomError(format!(
            "mismatched count of arguments for function {}",
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

// pub fn get_list_arg(expr: Expression) -> Result<Rc<Vec<Expression>>, LmError> {
//     match expr {
//         Expression::List(s) => Ok(s),
//         _ => Err(LmError::CustomError("expected string".to_string())),
//     }
// }

// pub fn get_list_args(
//     args: &[Expression],
//     env: &mut Environment,
// ) -> Result<Vec<Rc<Vec<Expression>>>, LmError> {
//     args.iter()
//         .map(|arg| get_list_arg(arg.eval(env)?))
//         .collect()
// }

pub fn get_exact_string_arg(expr: Expression) -> Result<String, LmError> {
    match expr {
        Expression::String(s) => Ok(s),
        e => Err(LmError::TypeError {
            expected: "String".to_string(),
            found: e.type_name(),
            sym: e.to_string(),
        }),
    }
}
pub fn get_string_arg(expr: Expression) -> Result<String, LmError> {
    match expr {
        Expression::Symbol(s) | Expression::String(s) => Ok(s),
        e => Err(LmError::TypeError {
            expected: "String".to_string(),
            found: e.type_name(),
            sym: e.to_string(),
        }),
    }
}

pub fn get_string_args(args: &[Expression], env: &mut Environment) -> Result<Vec<String>, LmError> {
    args.iter()
        .map(|arg| get_string_arg(arg.eval(env)?))
        .collect()
}

pub fn get_integer_arg(expr: Expression) -> Result<i64, LmError> {
    match expr {
        Expression::Integer(i) => Ok(i),
        e => Err(LmError::TypeError {
            expected: "Integer".to_string(),
            found: e.type_name(),
            sym: e.to_string(),
        }),
    }
}
