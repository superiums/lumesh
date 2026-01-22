use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
    rc::Rc,
};

use crate::{
    Environment, Expression, Int, RuntimeError, RuntimeErrorKind, VERSION,
    libs::{
        BuiltinFunc, BuiltinInfo, LIBS_INFO,
        bin::boolean_lib::not,
        helper::{check_args_len, check_exact_args_len, get_integer_arg, get_string_arg},
        pretty_printer,
    },
    parse_and_eval, reg_all, reg_info,
    utils::{canon, get_current_path},
};

pub fn regist_all() -> HashMap<&'static str, Rc<BuiltinFunc>> {
    reg_all!({
        exit, cd, cwd,
        tap, print, pprint, println, eprint, eprintln, debug, read,
        r#typeof => "typeof",
        get, len, insert, rev, flatten, r#where => "where", select,
        not,
        repeat, eval, exec, eval_str, exec_str, include, import,
        help,
        // set;
        // unset;
        throw,

    })
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        // console control
        // Shell control
        exit => "exit the shell", "[status]"
        cd => "change current directory", "[path]"
        cwd => "print current working directory", ""
        // env control
        // set => "define a variable in root environment", "<var> <val>"
        // unset => "undefine a variable in root environment", "<var>"

        // I/O operations
        tap => "print and return result", "<args>..."
        print => "print arguments without newline", "<args>..."
        pprint => "pretty print", "<list>|<map>"
        println => "print arguments with newline", "<args>..."
        eprint => "print to stderr without newline", "<args>..."
        eprintln => "print to stderr with newline", "<args>..."
        debug => "print debug representation", "<args>..."
        read => "get user input", "[prompt]"
        throw => "return a runtime error", "<msg>"

        // Data manipulation
        get => "get value from nested map/list/range using dot notation path", "<map|list|range> <path>"
        typeof => "get data type", "<value>"
        len => "get length of expression", "<collection>"
        insert => "insert item into collection", "<collection> <key/index> <value>"
        rev => "reverse sequence", "<string|list|bytes>"
        flatten => "flatten nested structure", "<collection>"
        where => "filter rows by condition", "<list[map]> <condition> "
        select => "select columns from list of maps", "<list[map]> <columns>..."
        not => "logic not", "<boolean1>..."

        // Execution control
        repeat => "evaluate without env change", "<expr>"
        eval => "evaluate expression in current env", "<expr>"
        exec => "execute expression in new env", "<expr>"
        eval_str => "evaluate string in current env", "<expr>"
        exec_str => "execute string in new env", "<string>"
        include => "evaluate file in current env", "<path>"
        import => "evaluate file in new env", "<path>"

        // Help system
        help => "display help", "[module]"
    })
}

fn help(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut s = std::io::stdout().lock();
    match args.is_empty() {
        false => match args[0].to_string().as_str() {
            "doc" => {
                parse_and_eval("xdg-open https://lumesh.codeberg.page", env);
            }
            "libs" => {
                writeln!(s, "Builtin Library List\n").unwrap();
                LIBS_INFO.with(|h| {
                    // let _ = pprint(
                    //     &vec![Expression::from(
                    //         h.keys()
                    //             .map(|k| Expression::String(k.to_string()))
                    //             .collect::<Vec<_>>(),
                    //     )],
                    //     env,
                    //     _ctx,
                    // );
                    for (i, lib) in h.keys().enumerate() {
                        if lib.is_empty() {
                            continue;
                        }
                        write!(s, "  \x1b[92m\x1b[1m{lib:20}\x1b[m\x1b[0m").unwrap();
                        if i % 3 == 0 {
                            writeln!(s, "\n").unwrap();
                        }
                    }
                });
                // let m = super::get_builtin_map()
                //     .iter()
                //     .filter_map(|item| match item.1 {
                //         Expression::HMap(second) => {
                //             let mut s_keys = second.keys().cloned().collect::<Vec<_>>();
                //             s_keys.sort();
                //             Some((item.0.clone(), Expression::from(s_keys)))
                //         }
                //         _ => None,
                //     })
                //     .collect::<HashMap<String, Expression>>();
                // pretty_printer(&Expression::from(m))?;
                writeln!(
                    s,
                    "\n\nhelp <lib>              : list functions of the lib."
                )
                .unwrap();
                writeln!(
                    s,
                    "help <lib>.<func>       : to see details of the function"
                )
                .unwrap();
                writeln!(s, "\n\nUsage:").unwrap();
                writeln!(s, "\n    <lib>.<func> params").unwrap();
                writeln!(s, "\nExample:").unwrap();
                writeln!(s, "\n    string.green lume").unwrap();
                writeln!(s, "\n    string.green(lume)").unwrap();
                writeln!(s, "\n    'lume'.green()").unwrap();
                writeln!(s, "\n    'lume' | string.green()").unwrap();
                writeln!(s, "\n    'lume' | .green()").unwrap();
            }
            "tops" => {
                writeln!(s, "Top level Functions List\n").unwrap();
                LIBS_INFO.with(|h| match h.get("") {
                    Some(map) => {
                        for (func, info) in map {
                            writeln!(
                                s,
                                "\n\x1b[92m\x1b[1m{func}\x1b[m\x1b[0m \x1b[2m{}\x1b[m\x1b[0m",
                                info.hint
                            )
                            .unwrap();
                            writeln!(s, "\t{}", info.descr).unwrap();
                        }
                    }
                    _ => {}
                });
                writeln!(s, "\njust use them directly anywhere!").unwrap();
            }
            name => match name.split_once(".") {
                Some((name, func)) => {
                    LIBS_INFO.with(|h| match h.get(&name) {
                            Some(map) => match map.get(func) {
                                Some(info) => {
                                    writeln!(s, "{name}.\x1b[92m\x1b[1m{func}\x1b[m\x1b[0m \x1b[2m{}\x1b[m\x1b[0m",info.hint).unwrap();
                                    writeln!(s, "\t{}\n", info.descr).unwrap();
                                }
                                _ => {
                                    writeln!(s, "no function named `{func}` in `{name}`\n")
                                        .unwrap();
                                }
                            },
                            _ => {
                                writeln!(s, "no lib named `{name}`\n").unwrap();
                            }
                        });
                }
                _ => {
                    LIBS_INFO.with(|h| match h.get(&name) {
                            Some(map) => {
                                writeln!(s, "Functions for lib {name}\n").unwrap();
                                for (func, info) in map {
                                    writeln!(s, "{name}.\x1b[92m\x1b[1m{func}\x1b[m\x1b[0m \x1b[2m{}\x1b[m\x1b[0m",info.hint).unwrap();
                                    writeln!(s, "\t{}\n", info.descr).unwrap();
                                }
                            }
                            _ => {
                                LIBS_INFO.with(|h| match h.get("") {
                                    Some(map) => match map.get(name) {
                                        Some(info) => {
                                            writeln!(s, "\x1b[92m\x1b[1m{name}\x1b[m\x1b[0m \x1b[2m{}\x1b[m\x1b[0m",info.hint).unwrap();
                                            writeln!(s, "\t{}\n", info.descr).unwrap();
                                        }
                                        _ => {
                                            writeln!(s, "no function named `{name}` in top\n")
                                                .unwrap();
                                        }
                                    },
                                    _ => {
                                    }
                                });
                                 writeln!(s, "no lib named `{name}`\n").unwrap();
                            }
                        });
                }
            },
        },
        true => {
            let _ = writeln!(s, "\n\x1b[92m\x1b[1mWelcome to Lumesh help center");
            let _ = writeln!(&mut s, "=============================\x1b[m\x1b[0m\n");
            let _ = writeln!(&mut s, "version: \x1b[92m{}\x1b[m\x1b[0m\n", VERSION);
            let _ = writeln!(&mut s, "help libs               : list libs.");
            let _ = writeln!(
                &mut s,
                "help <lib>              : list functions of the lib."
            );
            let _ = writeln!(
                &mut s,
                "help tops               : list functions of the top level."
            );
            let _ = writeln!(
                &mut s,
                "help <lib>.<func>       : see the detail of the function."
            );
            let _ = writeln!(
                &mut s,
                "help <func>             : see the detail of top functions."
            );
            let _ = writeln!(
                &mut s,
                "help doc                : visit document on https://lumesh.codeberg.page"
            );
        }
    }
    let _ = s.flush();
    Ok(Expression::None)
}
fn import(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("import", args, 1, ctx)?;

    let canon_path = canon(&args[0].eval(env)?.to_string())?;
    // Read the file.
    let contents = std::fs::read_to_string(canon_path.clone()).map_err(|e| {
        RuntimeError::common(
            format!("could not read file {}: {}", canon_path.display(), e).into(),
            ctx.clone(),
            0,
        )
    })?;
    // Evaluate the file.
    let r = parse_and_eval(&contents, &mut env.fork());
    Ok(Expression::from(r))
}
fn include(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("include", args, 1, ctx)?;
    let path = get_string_arg(args[0].eval(env)?, ctx)?;

    let canon_path = canon(&path)?;

    // Read the file.
    let contents = std::fs::read_to_string(canon_path.clone()).map_err(|e| {
        RuntimeError::new(
            RuntimeErrorKind::CustomError(format!("failed to read file '{}': {}", path, e).into()),
            ctx.clone(),
            0,
        )
    })?;
    // Evaluate the file.
    let r = parse_and_eval(&contents, env);
    Ok(Expression::from(r))
}
fn exit(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let code = if args.is_empty() {
        0
    } else {
        match args[0].eval(env)? {
            Expression::Integer(n) => n as i32,
            _ => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::CustomError("exit code must be integer".into()),
                    ctx.clone(),
                    0,
                ));
            }
        }
    };
    std::process::exit(code);
}
fn cd(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut path = if args.len() == 0 {
        "~".to_string()
    } else {
        match args[0].eval_in_assign(env)? {
            Expression::Symbol(path) | Expression::String(path) => path,
            other => other.to_string(),
        }
    };
    if path == "-" {
        path = env.get("LWD").map_or("~".to_string(), |x| x.to_string());
    }
    let _ = std::env::current_dir()
        .and_then(|x| Ok(env.define("LWD", Expression::String(x.to_string_lossy().into()))));

    if path.starts_with("~") {
        if let Some(home_dir) = dirs::home_dir() {
            path = path.replace("~", home_dir.to_string_lossy().as_ref());
        }
    }
    std::env::set_current_dir(&path).map_err(|io_err| {
        RuntimeError::from_io_error(io_err, "set env path".into(), ctx.clone(), 0)
    })?;

    env.define_in_root("PWD", Expression::String(path));
    Ok(Expression::None)
}

fn cwd(
    _: &[Expression],
    _: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let path = get_current_path();
    Ok(Expression::String(path.to_string_lossy().into_owned()))
}

fn r#typeof(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("typeof", args, 1, ctx)?;
    let rs = if matches!(args[0], Expression::Symbol(_) | Expression::Variable(_)) {
        args[0].eval_in_assign(env)?.type_name()
    } else {
        args[0].type_name()
    };
    Ok(Expression::String(rs))
}

fn debug(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    for (i, arg) in args.iter().enumerate() {
        let x = arg.eval_in_assign(env)?;
        if i < args.len() - 1 {
            print!("{x:?} ")
        } else {
            println!("{x:?}")
        }
    }
    Ok(Expression::None)
}

fn tap(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut stdout = std::io::stdout().lock();
    let mut result: Vec<Expression> = Vec::with_capacity(args.len());
    for (i, arg) in args.iter().enumerate() {
        let x = arg.eval_in_assign(env)?;
        if i < args.len() - 1 {
            let _ = write!(&mut stdout, "{x} ");
        } else {
            let _ = writeln!(&mut stdout, "{x}");
        }
        result.push(x)
    }
    let _ = stdout.flush();
    if result.len() == 1 {
        return Ok(result[0].clone());
    }
    Ok(Expression::from(result))
}

fn print(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut stdout = std::io::stdout().lock();
    for arg in args.iter() {
        let x = arg.eval_in_assign(env)?;
        let _ = write!(&mut stdout, "{x} ");
    }
    let _ = writeln!(&mut stdout);
    let _ = stdout.flush();
    Ok(Expression::None)
}
fn println(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut stdout = std::io::stdout().lock();
    for arg in args.iter() {
        let x = arg.eval_in_assign(env)?;
        // println!("{}", x);
        let _ = writeln!(&mut stdout, "{x}");
    }
    let _ = stdout.flush();
    Ok(Expression::None)
}
pub fn pprint(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("pprint", args, 1.., ctx)?;
    // let _ = args.iter().map(|a| pretty_printer(&a.eval(env)?));
    for arg in args.iter() {
        let r = arg.eval_in_assign(env)?;
        pretty_printer(&r)?;
    }
    Ok(Expression::None)
}
fn eprint(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut stderr = std::io::stderr().lock();
    for (i, arg) in args.iter().enumerate() {
        let x = arg.eval_in_assign(env)?;
        if i < args.len() - 1 {
            let _ = write!(&mut stderr, "\x1b[38;5;9m{x} \x1b[m\x1b[0m");
        } else {
            let _ = writeln!(&mut stderr, "\x1b[38;5;9m{x}\x1b[m\x1b[0m");
        }
    }
    let _ = stderr.flush();
    Ok(Expression::None)
}
fn eprintln(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut stderr = std::io::stderr().lock();
    for arg in args.iter() {
        let x = arg.eval_in_assign(env)?;
        let _ = writeln!(&mut stderr, "\x1b[38;5;9m{x}\x1b[m\x1b[0m");
    }
    let _ = stderr.flush();
    Ok(Expression::None)
}

fn read(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    print(args, env, ctx)?;
    let _ = std::io::stdout().flush();

    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
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

pub fn insert(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("insert", args, 3, ctx)?;
    let mut arr = args[0].eval(env)?;
    let idx = args[1].eval(env)?;
    let val = args[2].eval(env)?;
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
                Err(RuntimeError::new(
                    RuntimeErrorKind::CustomError(
                        format!("index {} out of bounds for insertion", i).into(),
                    ),
                    ctx.clone(),
                    0,
                ))
            }
        }
        (Expression::String(s), Expression::Integer(i)) => {
            if *i as usize <= s.len() {
                s.insert_str(*i as usize, &val.to_string());
                Ok(Expression::String(s.clone()))
            } else {
                Err(RuntimeError::new(
                    RuntimeErrorKind::CustomError(
                        format!("index {} out of bounds for insertion", i).into(),
                    ),
                    ctx.clone(),
                    0,
                ))
            }
        }
        _ => Err(RuntimeError::new(
            RuntimeErrorKind::CustomError("insert requires a list or map as first argument".into()),
            ctx.clone(),
            0,
        )),
    }

    // Ok(arr)
}

pub fn len(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("len", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::HMap(m) => Ok(Expression::Integer(m.as_ref().len() as Int)),
        Expression::Map(m) => Ok(Expression::Integer(m.as_ref().len() as Int)),
        Expression::List(list) => Ok(Expression::Integer(list.as_ref().len() as Int)),
        Expression::Symbol(x) | Expression::String(x) => {
            Ok(Expression::Integer(x.chars().count() as Int))
        }
        Expression::Bytes(bytes) => Ok(Expression::Integer(bytes.len() as Int)),
        Expression::Range(a, b) => Ok(Expression::Integer(a.step_by(b).count() as Int)),
        expr => Err(RuntimeError::new(
            RuntimeErrorKind::CustomError(
                format!("len not supported for type {}", expr.type_name()).into(),
            ),
            ctx.clone(),
            0,
        )),
    }
}

pub fn rev(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("rev", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::List(list) => {
            let mut reversed = list.as_ref().to_vec();
            reversed.reverse();
            Ok(Expression::from(reversed))
        }
        Expression::String(s) => Ok(Expression::String(s.chars().rev().collect())),
        Expression::Symbol(s) => Ok(Expression::Symbol(s.chars().rev().collect())),
        Expression::Bytes(b) => Ok(Expression::Bytes(b.into_iter().rev().collect())),
        _ => Err(RuntimeError::new(
            RuntimeErrorKind::CustomError("rev requires a string, list, or bytes".into()),
            ctx.clone(),
            0,
        )),
    }
}

fn eval_str(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("eval_str", args, 1, ctx)?;
    let exp = match &args[0] {
        Expression::String(cmd) => cmd,
        Expression::StringTemplate(_) | Expression::Symbol(_) | Expression::Variable(_) => {
            &args[0].eval(env)?.to_string()
        }
        Expression::Group(cmd) => match cmd.as_ref() {
            Expression::String(cmd) => cmd,
            _ => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::CustomError("only String acceptable to exec_str".into()),
                    ctx.clone(),
                    0,
                ));
            }
        },
        _ => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::CustomError("only String acceptable to exec_str".into()),
                ctx.clone(),
                0,
            ));
        }
    };
    if exp.is_empty() {
        Ok(Expression::None)
    } else {
        println!("\n  >> Excuting: \x1b[38;5;208m\x1b[1m{exp}\x1b[m\x1b[0m");
        Ok(Expression::Boolean(parse_and_eval(exp, env)))
    }
}
fn exec_str(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("exec_str", args, 1, ctx)?;
    let mut new_env = env.clone();
    eval_str(args, &mut new_env, ctx)
}
fn repeat(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("repeat", args, 2, ctx)?;
    let n = get_integer_arg(args[1].eval(env)?, ctx)?;
    let r = (0..n)
        .map(|_| args[0].eval_in_assign(env))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expression::from(r))
}
fn eval(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("eval", args, 1, ctx)?;
    Ok(args[0].eval(env)?.eval(env)?)
}

fn exec(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("exec", args, 1, ctx)?;
    let mut new_env = env.clone();
    Ok(args[0].eval(env)?.eval(&mut new_env)?)
}

fn flat(expr: Expression) -> Vec<Expression> {
    match expr {
        Expression::List(list) => list
            .as_ref()
            .iter()
            .flat_map(|item| flat(item.clone()))
            .collect(),
        Expression::HMap(map) => map
            .as_ref()
            .values()
            .flat_map(|item| flat(item.clone()))
            .collect(),
        Expression::Map(map) => map
            .as_ref()
            .values()
            .flat_map(|item| flat(item.clone()))
            .collect(),
        expr => vec![expr],
    }
}

pub fn flatten(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("flatten", args, 1, ctx)?;
    Ok(Expression::from(flat(args[0].eval(env)?)))
}

fn r#where(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    // dbg!(&args);
    check_exact_args_len("where", args, 2, ctx)?;

    let data = if let Expression::List(list) = args[0].eval(env)? {
        list
    } else {
        return Err(RuntimeError::new(
            RuntimeErrorKind::CustomError("Expected list for filtering".into()),
            ctx.clone(),
            0,
        ));
    };

    let mut filtered = Vec::new();

    let mut row_env = env.fork();
    row_env.define("LINES", Expression::Integer(data.len() as i64));
    for (i, row) in data.as_ref().iter().enumerate() {
        row_env.define("LINENO", Expression::Integer(i as i64));

        // dbg!(row_env.get("LINENO"));
        if let Expression::HMap(row_map) = row {
            for (k, v) in row_map.as_ref() {
                row_env.define(k, v.clone());
            }
            if let Expression::Boolean(true) = args[1].eval(&mut row_env)? {
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

fn select(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    // check_exact_args_len("select", args, 2)?;

    let headers = match args.len() {
        3.. => args[1..].iter().map(|a| a.to_string()).collect::<Vec<_>>(),
        2 => {
            let a = args[1].eval(env)?;
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
            return Err(RuntimeError::common(
                "select required 2 or more args".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let data = if let Expression::List(list) = args[0].eval(env)? {
        list
    } else {
        return Err(RuntimeError::common(
            "Expected list for column selection".into(),
            ctx.clone(),
            0,
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

pub fn get(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("get", args, 2, ctx)?;

    let index = args[1].eval(env)?;
    let mut current = args[0].eval(env)?;

    // let path = get_string_arg(index)?;
    let path = match index {
        Expression::Symbol(s) | Expression::String(s) => s,
        Expression::Integer(i) => i.to_string(),
        _ => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "symbol/string/integer as path".to_owned(),
                    sym: index.to_string(),
                    found: index.type_name(),
                },
                ctx.clone(),
                0,
            ));
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
                        RuntimeError::common(
                            format!(
                                "path segment '{}' not found in Map:\n`{:?}`",
                                segment,
                                m.as_ref()
                            )
                            .into(),
                            ctx.clone(),
                            0,
                        )
                    })?
                    .clone();
            }
            Expression::HMap(m) => {
                current = m
                    .as_ref()
                    .get(segment)
                    .ok_or_else(|| {
                        RuntimeError::common(
                            format!(
                                "path segment '{}' not found in HMap:\n`{:?}`",
                                segment,
                                m.as_ref()
                            )
                            .into(),
                            ctx.clone(),
                            0,
                        )
                    })?
                    .clone();
            }
            Expression::List(m) => match segment.parse::<usize>() {
                Ok(key) => {
                    current = m
                        .as_ref()
                        .get(key)
                        .ok_or_else(|| {
                            RuntimeError::common(
                                format!(
                                    "path index '{}' not found in List:\n`{:?}`",
                                    segment,
                                    m.as_ref()
                                )
                                .into(),
                                ctx.clone(),
                                0,
                            )
                        })?
                        .clone();
                }
                _ => {
                    return Err(RuntimeError::common(
                        format!("path index '{segment}' is not valid for List").into(),
                        ctx.clone(),
                        0,
                    ));
                }
            },
            Expression::Range(m, step) => match segment.parse::<usize>() {
                Ok(key) => {
                    current = m
                        .step_by(step)
                        .nth(key)
                        .map(Expression::Integer)
                        .ok_or_else(|| {
                            RuntimeError::common(
                                format!("path index '{segment}' not found in Range").into(),
                                ctx.clone(),
                                0,
                            )
                        })?
                        .clone();
                }
                _ => {
                    return Err(RuntimeError::common(
                        format!("path index '{segment}' is not valid for Range").into(),
                        ctx.clone(),
                        0,
                    ));
                }
            },
            _ => {
                return Err(RuntimeError::common(
                    format!(
                        "path segment '{}' attempt to access on non-indexable type: {}",
                        segment,
                        current.type_name()
                    )
                    .into(),
                    ctx.clone(),
                    0,
                ));
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

pub fn throw(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("sys.error", args, 1, ctx)?;
    let msg = args[0].eval(env)?;
    Err(RuntimeError::common(msg.to_string().into(), ctx.clone(), 0))
}
