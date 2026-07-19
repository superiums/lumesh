use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
    rc::Rc,
};

use crate::{
    Environment, Expression, Int, RuntimeError, RuntimeErrorKind, VERSION,
    expression::table::TableData,
    libs::{
        BuiltinFunc, BuiltinInfo, LIBS_INFO,
        bin::{
            boolean_lib::not,
            table_lib::{select, sortby},
        },
        helper::{check_args_len, check_exact_args_len, get_string_ref},
        pretty_printer,
    },
    parse_and_eval, reg_all, reg_info,
    utils::{abs_script, canon, get_current_path_string},
};

pub fn regist_all() -> HashMap<&'static str, Rc<BuiltinFunc>> {
    reg_all!({
        exit, cd, cwd, symof,
        tap, print, pprint, println, eprint, eprintln, read,
        get, len, rev, flatten,  select, sortby,
        not,
        eval, exec, eval_str, exec_str, include, import,
        help,
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
        symof => "get type of data symbol", "<value>"
        tap => "print and return result", "<args>..."
        print => "print arguments without newline", "<args>..."
        pprint => "pretty print", "<list>|<map>"
        println => "print arguments with newline", "<args>..."
        // printf => "print formatted string with vars", "<template> <args>..."
        eprint => "print to stderr without newline", "<args>..."
        eprintln => "print to stderr with newline", "<args>..."
        // debug => "print debug representation", "<args>..."
        // ddebug => "pretty debug", "<args>..."
        read => "get user input", "[prompt]"
        throw => "return a runtime error", "<msg>"

        // Data manipulation
        get => "get value from nested map/list/range using dot notation path", "<map|list|range> <path>"
        // typeof => "get data type", "<value>"
        len => "get length of expression", "<collection>"
        rev => "reverse sequence", "<string|list|bytes>"
        flatten => "flatten nested structure", "<collection>"
        // where => "filter rows by condition", "<list[map]> <condition> "
        select => "select columns from list of maps", "<table> <columns...>"
        sortby => "sort a table by column", "<table> <col>"
        not => "logic not", "<boolean1>..."

        // Execution control
        // repeat => "evaluate without env change", "<expr>"
        eval => "evaluate expression in current env", "<expr>"
        exec => "execute expression in new env", "<expr>"
        eval_str => "evaluate string in current env", "<expr>"
        exec_str => "execute string in new env", "<string>"
        include => "evaluate file in current env", "<path>"
        import => "evaluate file in new env", "<path>"

        // env
        // set_root => "define a variable in root environment", "<var> <val>"
        // unset_root => "undefine a variable in root environment", "<var>"
        // getvar => "get a variable value", "<var>"

        // Help system
        help => "display help", "[module]"
    })
}

fn help(
    args: Vec<Expression>,
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    use std::fmt::Write as FmtWrite;
    let mut s = String::new();

    match args.is_empty() {
        false => match args[0].to_string().as_str() {
            "doc" => {
                if cfg!(target_os = "macos") {
                    parse_and_eval("open https://www.lumesh.cc.cd", env);
                } else if cfg!(windows) {
                    parse_and_eval("start https://www.lumesh.cc.cd", env);
                } else {
                    parse_and_eval("xdg-open https://www.lumesh.cc.cd", env);
                }
                return Ok(Expression::None);
            }
            "libs" => {
                writeln!(s, "Builtin Library List\n").unwrap();
                LIBS_INFO.with(|h| {
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
                LIBS_INFO.with(|h| {
                    if let Some(map) = h.get("") {
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
                });
                writeln!(s, "\njust use them directly anywhere!").unwrap();
            }
            name => match name.split_once(".") {
                Some((name, func)) => {
                    LIBS_INFO.with(|h| match h.get(&name) {
                        Some(map) => match map.get(func) {
                            Some(info) => {
                                writeln!(
                                    s,
                                    "{name}.\x1b[92m\x1b[1m{func}\x1b[m\x1b[0m \x1b[2m{}\x1b[m\x1b[0m",
                                    info.hint
                                )
                                .unwrap();
                                writeln!(s, "\t{}\n", info.descr).unwrap();
                            }
                            _ => {
                                writeln!(s, "no function named `{func}` in `{name}`\n").unwrap();
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
                                writeln!(
                                    s,
                                    "{name}.\x1b[92m\x1b[1m{func}\x1b[m\x1b[0m \x1b[2m{}\x1b[m\x1b[0m",
                                    info.hint
                                )
                                .unwrap();
                                writeln!(s, "\t{}\n", info.descr).unwrap();
                            }
                        }
                        _ => {
                            LIBS_INFO.with(|h| {
                                if let Some(map) = h.get("") {
                                    match map.get(name) {
                                        Some(info) => {
                                            writeln!(
                                                s,
                                                "\x1b[92m\x1b[1m{name}\x1b[m\x1b[0m \x1b[2m{}\x1b[m\x1b[0m",
                                                info.hint
                                            )
                                            .unwrap();
                                            writeln!(s, "\t{}\n", info.descr).unwrap();
                                        }
                                        _ => {
                                            writeln!(
                                                s,
                                                "no function named `{name}` in top\n"
                                            )
                                            .unwrap();
                                        }
                                    }
                                }
                            });
                            writeln!(s, "no lib named `{name}`\n").unwrap();
                        }
                    });
                }
            },
        },
        true => {
            writeln!(s, "\n\x1b[92m\x1b[1mWelcome to Lumesh help center").unwrap();
            writeln!(s, "=============================\x1b[m\x1b[0m\n").unwrap();
            writeln!(s, "version: \x1b[92m{}\x1b[m\x1b[0m\n", VERSION).unwrap();
            writeln!(s, "help libs               : list libs.").unwrap();
            writeln!(s, "help <lib>              : list functions of the lib.").unwrap();
            writeln!(
                s,
                "help tops               : list functions of the top level."
            )
            .unwrap();
            writeln!(
                s,
                "help <lib>.<func>       : see the detail of the function."
            )
            .unwrap();
            writeln!(
                s,
                "help <func>             : see the detail of top functions."
            )
            .unwrap();
            writeln!(
                s,
                "help doc                : visit document on https://www.lumesh.cc.cd"
            )
            .unwrap();
        }
    }

    Ok(Expression::String(s))
}

fn import(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("import", &args, 1, ctx)?;
    let path = get_string_ref(&args[0], ctx)?;
    let canon_path = abs_script(path, env);
    // Read the file.
    let contents = std::fs::read_to_string(canon_path).map_err(|e| {
        RuntimeError::common(
            format!("failed to import file {}: {}", path, e).into(),
            ctx.clone(),
            0,
        )
    })?;
    // Evaluate the file.
    let r = parse_and_eval(&contents, &mut env.fork());
    Ok(Expression::from(r))
}
fn include(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("include", &args, 1, ctx)?;
    let path = get_string_ref(&args[0], ctx)?;
    let canon_path = abs_script(path, env);

    // Read the file.
    let contents = std::fs::read_to_string(canon_path).map_err(|e| {
        RuntimeError::new(
            RuntimeErrorKind::CustomError(
                format!("failed to include file '{}': {}", path, e).into(),
            ),
            ctx.clone(),
            0,
        )
    })?;
    // Evaluate the file.
    let r = parse_and_eval(&contents, env);
    Ok(Expression::from(r))
}
fn exit(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let code = if args.is_empty() {
        0
    } else {
        match args[0] {
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
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    // target
    let p = if args.is_empty() {
        "~".to_string()
    } else {
        match args.into_iter().next().unwrap() {
            Expression::Symbol(path) | Expression::String(path) if path == "-" => {
                env.get("LWD").map_or("~".to_string(), |x| x.to_string())
            }
            Expression::Symbol(path) | Expression::String(path) => path,
            other => other.to_string(),
        }
    };
    let path = canon(&p, env)?;
    // before cd
    let current = get_current_path_string(env);
    // cd
    std::env::set_current_dir(&path).map_err(|io_err| {
        RuntimeError::from_io_error(io_err, "set_current_dir".into(), ctx.clone(), 0)
    })?;

    // after cd
    env.define_in_root(
        "PWD",
        Expression::String(path.to_string_lossy().to_string()),
    );

    // let current = current_dir.to_string_lossy();
    let dir_new = path.to_string_lossy();
    env.define("LWD", Expression::String(current.to_string()));
    if let Some(Expression::List(paths)) = env.get("PATH_SESSION") {
        if !paths.iter().any(|p| {
            if let Expression::String(ps) = p {
                ps.as_str() == dir_new.as_ref()
            } else {
                false
            }
        }) {
            let mut pn = paths.as_ref().clone();
            pn.push(Expression::String(dir_new.to_string()));
            env.define("PATH_SESSION", Expression::from(pn));
        }
    } else {
        env.define(
            "PATH_SESSION",
            Expression::from(vec![Expression::String(dir_new.to_string())]),
        );
    }

    Ok(Expression::None)
}

fn cwd(
    _: Vec<Expression>,
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let path = get_current_path_string(env);
    Ok(Expression::String(path))
}

fn symof(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("symof", &args, 1, ctx)?;
    let t = args[0].type_name();
    Ok(Expression::from(t))
}

fn tap(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("tap", &args, 1.., ctx)?;
    let mut stdout = std::io::stdout().lock();
    let mut result: Vec<Expression> = Vec::with_capacity(args.len());
    for (i, x) in args.iter().enumerate() {
        if i < args.len() - 1 {
            let _ = write!(&mut stdout, "{x} ");
        } else {
            let _ = writeln!(&mut stdout, "{x}");
        }
        result.push(x.clone())
    }
    let _ = stdout.flush();
    if result.len() == 1 {
        return Ok(result[0].clone());
    }
    Ok(Expression::from(result))
}

fn print(
    args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    // let is_tty = std::io::stdout().is_terminal();
    let mut stdout = std::io::stdout().lock();
    for x in args.iter() {
        let s = format!("{x} ");
        // if is_tty {
        //     let _ = write!(&mut stdout, "{}", s);
        //     // let _ = write!(&mut stdout, "{}", s.replace('\n', "\r\n"));
        // } else {
        let _ = write!(&mut stdout, "{s}");
        // }
    }
    // if is_tty {
    //     let _ = write!(&mut stdout, "\r\n");
    // } else {
    let _ = writeln!(&mut stdout);
    // }
    // let _ = stdout.flush();
    Ok(Expression::None)
}
fn println(
    args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut stdout = std::io::stdout().lock();
    for x in args.iter() {
        let s = format!("{x}");
        let _ = writeln!(&mut stdout, "{s}");
    }
    // let _ = stdout.flush();
    Ok(Expression::None)
}
pub fn pprint(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("pprint", &args, 1.., ctx)?;
    for arg in args.iter() {
        pretty_printer(arg)?;
    }
    Ok(Expression::None)
}
fn eprint(
    args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut stderr = std::io::stderr().lock();
    for (i, x) in args.iter().enumerate() {
        let s = format!("\x1b[38;5;9m{x}\x1b[m\x1b[0m");
        if i < args.len() - 1 {
            let _ = write!(&mut stderr, "{s} ");
        } else {
            let _ = writeln!(&mut stderr, "{s}");
        }
    }
    // let _ = stderr.flush();
    Ok(Expression::None)
}
fn eprintln(
    args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut stderr = std::io::stderr().lock();
    for x in args.iter() {
        let s = format!("\x1b[38;5;9m{x}\x1b[m\x1b[0m");
        let _ = writeln!(&mut stderr, "{s}");
    }
    // let _ = stderr.flush();
    Ok(Expression::None)
}

fn read(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    print(args, env, ctx)?;
    // let _ = std::io::stdout().flush();

    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    Ok(Expression::String(input.trim().to_owned()))
}

pub fn len(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("len", &args, 1, ctx)?;
    let i = match &args[0] {
        Expression::HMap(m) => m.as_ref().len() as Int,
        Expression::Map(m) => m.as_ref().len() as Int,
        Expression::List(list) => list.as_ref().len() as Int,
        Expression::BSet(s) => s.as_ref().len() as Int,
        Expression::Table(t) => t.row_count() as Int,
        Expression::Symbol(x) | Expression::String(x) => x.chars().count() as Int,
        Expression::Bytes(bytes) => bytes.len() as Int,
        Expression::Range(a, b) => a.to_owned().step_by(*b).count() as Int,
        Expression::None => 0,
        expr => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "List/Set/Range/Table/Map/HMap/Symbol/String/Bytes/None".into(),
                    sym: expr.to_string(),
                    found: expr.type_name(),
                    // format!("len not supported for type {}", expr.type_name()).into(),
                },
                ctx.clone(),
                0,
            ));
        }
    };
    Ok(Expression::Integer(i))
}

pub fn rev(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("rev", &args, 1, ctx)?;
    match args.first().unwrap() {
        Expression::List(list) => {
            let r = list.iter().rev().cloned().collect::<Vec<_>>();
            Ok(Expression::from(r))
        }
        Expression::Table(t) => {
            let r = t.rows().iter().rev().cloned().collect::<Vec<_>>();
            let tn = TableData::new(t.headers().to_vec(), r);
            Ok(Expression::from(tn))
        }
        Expression::String(s) => Ok(Expression::String(s.chars().rev().collect())),
        Expression::Symbol(s) => Ok(Expression::Symbol(s.chars().rev().collect())),
        Expression::Bytes(b) => Ok(Expression::Bytes(b.iter().rev().cloned().collect())),
        _ => Err(RuntimeError::new(
            RuntimeErrorKind::CustomError("rev requires a string, list, or bytes".into()),
            ctx.clone(),
            0,
        )),
    }
}

fn eval_str(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("eval_str", &args, 1, ctx)?;
    let exp = match &args[0] {
        Expression::String(cmd) => cmd,
        Expression::Symbol(s) | Expression::Variable(s) => s,
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
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("exec_str", &args, 1, ctx)?;
    let mut new_env = env.fork();
    eval_str(args, &mut new_env, ctx)
}

// need args evaled already
fn eval(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("eval", &args, 1, ctx)?;
    args[0].eval_in_assign(env)
}
// need args evaled already
fn exec(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("exec", &args, 1, ctx)?;
    let mut new_env = env.fork();
    args[0].eval_in_assign(&mut new_env)
}

fn flat(expr: &Expression) -> Vec<Expression> {
    match expr {
        Expression::List(list) => list.as_ref().iter().flat_map(flat).collect(),
        Expression::HMap(map) => map.as_ref().values().flat_map(flat).collect(),
        Expression::Map(map) => map.as_ref().values().flat_map(flat).collect(),
        expr => vec![expr.clone()],
    }
}

pub fn flatten(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("flatten", &args, 1, ctx)?;
    Ok(Expression::from(flat(&args[0])))
}

pub fn get(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("get", &args, 2, ctx)?;
    let mut it = args.into_iter();
    let mut current = it.next().unwrap();
    let index = it.next().unwrap();

    // let path = get_string_arg(index)?;
    let path = match index {
        Expression::Symbol(s) | Expression::String(s) => s,
        Expression::Integer(i) => i.to_string(),
        _ => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "Symbol/String/Integer as path".into(),
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
            Expression::BSet(s) => match segment.parse::<usize>() {
                Ok(key) => {
                    current = s
                        .as_ref()
                        .iter()
                        .nth(key)
                        .ok_or_else(|| {
                            RuntimeError::common(
                                format!("path index '{}' not found in BSet", segment).into(),
                                ctx.clone(),
                                0,
                            )
                        })?
                        .clone();
                }
                _ => {
                    return Err(RuntimeError::common(
                        format!("path index '{}' is not valid for BSet", segment).into(),
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
                        })?;
                }
                _ => {
                    return Err(RuntimeError::common(
                        format!("path index '{segment}' is not valid for Range").into(),
                        ctx.clone(),
                        0,
                    ));
                }
            },
            Expression::Table(table) => match segment.parse::<usize>() {
                Ok(key) => {
                    current = table
                        .rows()
                        .get(key)
                        .cloned()
                        .map(Expression::from)
                        .ok_or_else(|| {
                            RuntimeError::common(
                                format!("row '{segment}' not found in Table").into(),
                                ctx.clone(),
                                0,
                            )
                        })?;
                }
                _ => {
                    return Err(RuntimeError::common(
                        format!("path index '{segment}' is not valid for Table").into(),
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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("sys.error", &args, 1, ctx)?;
    let msg = &args[0];
    Err(RuntimeError::common(msg.to_string().into(), ctx.clone(), 0))
}
