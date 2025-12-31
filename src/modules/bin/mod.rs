use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
};

use crate::{
    Environment, Expression, Int, LmError,
    modules::{
        bin::sys_module::{set_builtin, throw, unset_builtin},
        // helper::{check_args_len, check_exact_args_len},
        pretty_printer,
    },
    parse_and_eval,
};
use common_macros::hash_map;

mod console_module;
mod map_module;
// mod fn_module;
// use fn_module::curry;
mod boolean_module;
mod filesize_module;
mod list_module;
mod log_module;
mod math_module;
// mod operator_module;
mod about_module;
mod from_module;
mod fs_ls;
mod fs_module;
mod helper;
mod into_module;
mod rand_module;
mod regex_module;
mod string_module;
mod sys_module;
pub mod time_module;
mod ui_module;
pub use helper::{
    check_args_len, check_exact_args_len, get_exact_string_arg, get_integer_arg, get_string_arg,
    get_string_args,
};
pub fn get_module_map() -> HashMap<String, Expression> {
    hash_map! {
        String::from("Log") => log_module::get(),
        String::from("Math") => math_module::get(),
        String::from("Map") => map_module::get(),
        String::from("About") => about_module::get(),
        String::from("Ui") => ui_module::get(),
        String::from("Time") => time_module::get(),
        String::from("Rand") => rand_module::get(),
        String::from("Console") => console_module::get(),
        String::from("From") => from_module::get(),
        String::from("Fs") => fs_module::get(),
        String::from("String") => string_module::get(),
        String::from("Regex") => regex_module::get(),
        String::from("List") => list_module::get(),
        String::from("Sys") => sys_module::get(),
        String::from("Into") => into_module::get(),
        String::from("Boolean") => boolean_module::get(),
        String::from("Filesize") => filesize_module::get(),

        // console control
        // Shell control
        String::from("exit") => Expression::builtin("exit", exit, "exit the shell", "[status]"),
        String::from("cd") => Expression::builtin("cd", cd, "change directories", "[path]"),
        String::from("pwd") => Expression::builtin("pwd", pwd, "print working directory", ""),
        // env control
        String::from("set") => Expression::builtin("set", set_builtin, "define a variable in root environment", "<var> <val>"),
        String::from("unset") => Expression::builtin("unset", unset_builtin, "undefine a variable in root environment", "<var>"),

        // I/O operations
        String::from("tap") => Expression::builtin("tap", tap, "print and return result", "<args>..."),
        String::from("print") => Expression::builtin("print", print, "print arguments without newline", "<args>..."),
        String::from("pprint") => Expression::builtin("pprint", pretty_print, "pretty print", "<list>|<map>"),
        String::from("println") => Expression::builtin("println", println, "print arguments with newline", "<args>..."),
        String::from("eprint") => Expression::builtin("eprint", eprint, "print to stderr without newline", "<args>..."),
        String::from("eprintln") => Expression::builtin("eprintln", eprintln, "print to stderr with newline", "<args>..."),
        String::from("debug") => Expression::builtin("debug", debug, "print debug representation", "<args>..."),
        String::from("read") => Expression::builtin("read", read, "get user input", "[prompt]"),
        String::from("throw") => Expression::builtin("throw", throw, "return a runtime error", "<msg>"),

        // Data manipulation
        String::from("get") => Expression::builtin("get", get, "get value from nested map/list/range using dot notation path", "<path> <map|list|range>"),
        String::from("typeof") => Expression::builtin("typeof", get_type, "get data type", "<value>"),
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
fn help(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    match args.is_empty() {
        false => match args[0].to_string().as_str() {
            "doc" => {
                parse_and_eval("xdg-open https://lumesh.codeberg.page", env);
            }
            "tops" => {
                println!("Top level Functions List\n");
                let m = super::get_builtin_map()
                    .into_iter()
                    .filter(|item| matches!(item.1, Expression::Builtin(_)))
                    .collect::<HashMap<_, _>>();
                pretty_printer(&Expression::from(m))?;
                println!("\njust use them directly anywhere!");
            }
            "libs" | "modules" => {
                println!("Modules List\n");
                let m = super::get_builtin_map()
                    .iter()
                    .filter_map(|item| match item.1 {
                        Expression::HMap(second) => {
                            let mut s_keys = second.keys().cloned().collect::<Vec<_>>();
                            s_keys.sort();
                            Some((item.0.clone(), Expression::from(s_keys)))
                        }
                        _ => None,
                    })
                    .collect::<HashMap<String, Expression>>();
                pretty_printer(&Expression::from(m))?;
                println!("\ntype `help <module-name>` to list functions of the module.");
                println!("\n\nUsage:");
                println!("\n    <module-name>.<function-name> params");
                println!("\nExample:");
                println!("\n    String.green hi");
                println!("\n    String.green(hi)");
                println!("\n    'hi'.green()");
            }
            mo => match super::get_builtin(mo) {
                Some(m) => {
                    if mo
                        .char_indices()
                        .next()
                        .is_some_and(|f| f.1.is_ascii_uppercase())
                    {
                        println!("Functions for module {mo}\n");
                        pretty_printer(m)?;
                        println!("\ntype `{mo}.<function-name>` to see details of the function");
                        println!("\ntype `{mo}.<tab>`           to cycle functions in the module");
                        println!("\ntype `{mo}. <tab>`          to popup functions in the module");
                    } else {
                        pretty_printer(m)?; //for top funcs
                        println!("\nit's a top level function. just use it directly anywhere!");
                    }
                }
                _ => return Err(LmError::CustomError(format!("no lib named {mo}"))),
            },
        },
        true => {
            let mut stdout = std::io::stdout().lock();
            writeln!(&mut stdout, "\nWelcome to Lumesh help center")?;
            writeln!(&mut stdout, "=================\n")?;
            writeln!(
                &mut stdout,
                "type `help libs/modules`         to list libs/modules."
            )?;
            writeln!(
                &mut stdout,
                "type `help <module-name>`        to list functions of the module."
            )?;
            writeln!(
                &mut stdout,
                "type `help tops`                 to list functions of the top level."
            )?;
            writeln!(
                &mut stdout,
                "type `help <func-name>`          to see the detail of top functions."
            )?;
            writeln!(
                &mut stdout,
                "type `<module-name>.<func-name>` to see the detail of the function."
            )?;
            writeln!(
                &mut stdout,
                "type `help doc`                  to visit document on https://lumesh.codeberg.page"
            )?;

            stdout.flush()?;
        }
    }
    Ok(Expression::None)
}
fn import(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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
fn include(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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
fn exit(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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
fn cd(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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
            cd(&[Expression::String(path)], env)
        }
    }
}
fn pwd(_: &[Expression], _: &mut Environment) -> Result<Expression, LmError> {
    let path = std::env::current_dir()?;
    // println!("{}", path.display());
    Ok(Expression::String(path.to_string_lossy().into_owned()))
}

fn get_type(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("typeof", args, 1)?;
    let rs = if matches!(args[0], Expression::Symbol(_) | Expression::Variable(_)) {
        args[0].eval(env)?.type_name()
    } else {
        args[0].type_name()
    };
    Ok(Expression::String(rs))
}

fn debug(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    for (i, arg) in args.iter().enumerate() {
        let x = arg.eval(env)?;
        if i < args.len() - 1 {
            print!("{x:?} ")
        } else {
            println!("{x:?}")
        }
    }
    Ok(Expression::None)
}

fn tap(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    let mut stdout = std::io::stdout().lock();
    let mut result: Vec<Expression> = Vec::with_capacity(args.len());
    for (i, arg) in args.iter().enumerate() {
        let x = arg.eval(env)?;
        if i < args.len() - 1 {
            write!(&mut stdout, "{x} ")?;
        } else {
            writeln!(&mut stdout, "{x}")?;
        }
        result.push(x)
    }
    stdout.flush()?;
    if result.len() == 1 {
        return Ok(result[0].clone());
    }
    Ok(Expression::from(result))
}

fn print(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    let mut stdout = std::io::stdout().lock();
    for arg in args.iter() {
        let x = arg.eval(env)?;
        write!(&mut stdout, "{x} ")?;
    }
    writeln!(&mut stdout)?;
    stdout.flush()?;
    Ok(Expression::None)
}
fn println(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    let mut stdout = std::io::stdout().lock();
    for arg in args.iter() {
        let x = arg.eval(env)?;
        // println!("{}", x);
        writeln!(&mut stdout, "{x}")?;
    }
    stdout.flush()?;
    Ok(Expression::None)
}
pub fn pretty_print(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_args_len("pprint", args, 1..)?;
    // let _ = args.iter().map(|a| pretty_printer(&a.eval(env)?));
    for arg in args.iter() {
        let r = arg.eval(env)?;
        pretty_printer(&r)?;
    }
    Ok(Expression::None)
}
fn eprint(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    let mut stderr = std::io::stderr().lock();
    for (i, arg) in args.iter().enumerate() {
        let x = arg.eval(env)?;
        if i < args.len() - 1 {
            write!(&mut stderr, "\x1b[38;5;9m{x} \x1b[m\x1b[0m")?;
        } else {
            writeln!(&mut stderr, "\x1b[38;5;9m{x}\x1b[m\x1b[0m")?;
        }
    }
    stderr.flush()?;
    Ok(Expression::None)
}
fn eprintln(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    let mut stderr = std::io::stderr().lock();
    for arg in args.iter() {
        let x = arg.eval(env)?;
        writeln!(&mut stderr, "\x1b[38;5;9m{x}\x1b[m\x1b[0m")?;
    }
    stderr.flush()?;
    Ok(Expression::None)
}

fn read(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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

pub fn insert(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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
                    "index {idx} out of bounds for {arr:?}"
                )))
            }
        }
        (Expression::String(s), Expression::Integer(i)) => {
            if *i as usize <= s.len() {
                s.insert_str(*i as usize, &val.to_string());
                Ok(Expression::String(s.clone()))
            } else {
                Err(LmError::CustomError(format!(
                    "index {idx} out of bounds for {arr:?}"
                )))
            }
        }
        _ => Err(LmError::CustomError(format!(
            "cannot insert {val:?} into {arr:?} with index {idx:?}"
        ))),
    }

    // Ok(arr)
}

pub fn len(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("len", args, 1)?;
    match args[0].eval(env)? {
        Expression::HMap(m) => Ok(Expression::Integer(m.as_ref().len() as Int)),
        Expression::Map(m) => Ok(Expression::Integer(m.as_ref().len() as Int)),
        Expression::List(list) => Ok(Expression::Integer(list.as_ref().len() as Int)),
        Expression::Symbol(x) | Expression::String(x) => {
            Ok(Expression::Integer(x.chars().count() as Int))
        }
        Expression::Bytes(bytes) => Ok(Expression::Integer(bytes.len() as Int)),
        Expression::Range(a, b) => Ok(Expression::Integer(a.step_by(b).count() as Int)),
        otherwise => Err(LmError::CustomError(format!(
            "cannot get length of {}:{}",
            otherwise,
            otherwise.type_name()
        ))),
    }
}

pub fn rev(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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

fn exec_str(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("exec_str", args, 1)?;
    match &args[0] {
        Expression::String(cmd) => {
            if !cmd.is_empty() {
                println!("\n  >> Excuting: \x1b[38;5;208m\x1b[1m{cmd}\x1b[m\x1b[0m");
                parse_and_eval(cmd, env);
            }
            Ok(Expression::None)
        }
        _ => Err(LmError::CustomError(
            "only String acceptable to exec_str".to_owned(),
        )),
    }
}
fn repeat(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("repeat", args, 2)?;
    let n = get_integer_arg(args[0].eval(env)?)?;
    let r = (0..n)
        .map(|_| args[1].eval(env))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expression::from(r))
}
fn eval(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("eval", args, 1)?;
    let mut new_env = env.clone();
    Ok(args[0].eval(env)?.eval(&mut new_env)?)
}

fn exec(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("exec", args, 1)?;
    Ok(args[0].eval(env)?.eval(env)?)
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

pub fn flatten_wrapper(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("flatten", args, 1)?;
    Ok(Expression::from(flatten(args[0].eval(env)?)))
}

fn filter_rows(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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

fn select_columns(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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

fn get(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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
            Expression::HMap(m) => {
                current = m
                    .as_ref()
                    .get(segment)
                    .ok_or_else(|| {
                        LmError::CustomError(format!(
                            "path segment '{}' not found in HMap `{:?}`",
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
                        "path index '{segment}' is not valid for List"
                    )));
                }
            },
            Expression::Range(m, step) => match segment.parse::<usize>() {
                Ok(key) => {
                    current = m
                        .step_by(step)
                        .nth(key)
                        .map(Expression::Integer)
                        .ok_or_else(|| {
                            LmError::CustomError(format!(
                                "path index '{segment}' not found in Range"
                            ))
                        })?
                        .clone();
                }
                _ => {
                    return Err(LmError::CustomError(format!(
                        "path index '{segment}' is not valid for Range"
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
