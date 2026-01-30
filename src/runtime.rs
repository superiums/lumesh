use crate::libs::pretty_printer;
use crate::{CFM_ENABLED, set_cfm_enabled, set_print_direct};

use crate::utils::expand_home;
use crate::with_print_direct;
use crate::{Environment, Expression, MAX_RUNTIME_RECURSION, MAX_SYNTAX_RECURSION, SyntaxError};
use crate::{SyntaxErrorKind, parse_script};
use std::collections::HashSet;
use std::fs::{create_dir, read_to_string, write};
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run_file(path: &str, env: &mut Environment) -> bool {
    let pb = PathBuf::from(path);
    run_file_via_pb(pb, env)
}
fn run_file_via_pb(pb: PathBuf, env: &mut Environment) -> bool {
    match read_to_string(pb.clone()) {
        Ok(prelude) => parse_and_eval(&prelude, env),
        Err(e) => {
            eprintln!(
                "\x1b[31m[IO ERROR]\x1b[0mFailed to read file '{}':\n  {e}",
                pb.display()
            );
            let _ = io::stderr().flush();
            false
        }
    }
}

pub fn parse_with_mode(text: &str) -> Result<Expression, SyntaxError> {
    let temp_cfm = if text.starts_with(">") && CFM_ENABLED.with_borrow(|cfm| cfm == &false) {
        set_cfm_enabled(true);
        true
    } else {
        false
    };
    let parsed = parse(text);
    if temp_cfm {
        set_cfm_enabled(false);
    }
    parsed
}
pub fn parse(input: &str) -> Result<Expression, SyntaxError> {
    // dbg!(&input);
    match parse_script(input) {
        Ok(result) => Ok(result),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => Err(SyntaxError {
            source: format!("{input}   ").into(),
            kind: e,
        }),
        Err(nom::Err::Incomplete(_)) => Err(SyntaxError {
            source: input.into(),
            kind: SyntaxErrorKind::InternalError("incomplted".to_string()),
        }),
    }
}

pub fn check(input: &str) -> bool {
    parse_script(input).is_ok()
}
/// return whether parse success. no matter execute result is.
pub fn parse_and_eval(text: &str, env: &mut Environment) -> bool {
    if text.is_empty() {
        return true;
    };

    let parsed = parse_with_mode(text);

    match parsed {
        Ok(expr) => {
            // rl.add_history_entry(text.as_str());
            // if let Some(path) = &history_path {
            //     if rl.save_history(path).is_err() {
            //         eprintln!("Failed to save history");
            //     }
            // }
            let val = expr.eval_cmd(env);
            // dbg!(env.get("cd"));
            match val {
                Ok(Expression::None) => {}
                // Ok(Expression::Builtin(b)) => {
                //     println!(
                //         "  >> [Builtin] {}\n\x1b[1;32mDescription\x1b[0m: {}\n\x1b[1;32mParams     \x1b[0m: {}\n",
                //         b.name, b.help, b.hint
                //     );
                //     let _ = io::stdout().flush();
                // }
                Ok(m)
                    if matches!(
                        m,
                        Expression::Map(_) | Expression::HMap(_) | Expression::List(_)
                    ) =>
                {
                    let _ = pretty_printer(&m);
                }
                Ok(result) => {
                    if with_print_direct(|v| v) {
                        match result {
                            // skip function and lambda define PD
                            Expression::Function(..) => {}
                            Expression::Lambda(..) => {}
                            r => println!("\n  >> [{}] <<\n{}", r.type_name(), r),
                        };
                        let _ = io::stdout().flush();
                    }
                }
                Err(e) => {
                    let _ = io::stdout().flush();
                    eprintln!("\x1b[31m_____________\x1b[0m\n{e}");
                    let _ = io::stderr().flush();
                }
            }

            return true;
        }

        Err(e) => {
            eprintln!("\x1b[31m[PARSE ERROR]\x1b[0m\n{e}");
            let _ = io::stderr().flush();
        }
    }
    false
}

pub fn init_config(env: &mut Environment) {
    #[cfg(unix)]
    const INTRO_PRELUDE: &str = include_str!("config/config.lm");
    #[cfg(windows)]
    const INTRO_PRELUDE: &str = include_str!("config/config_win.lm");

    let profile = match env.get("LUME_PROFILE") {
        Some(p) => PathBuf::from(p.to_string()),
        _ => match dirs::config_dir() {
            Some(config_dir) => {
                let config_path = config_dir.join("lumesh");
                if !config_path.exists() {
                    if let Err(e) = create_dir(&config_path) {
                        eprintln!("Error while create prelude dir: {e}");
                    }
                }
                config_path.join("config.lm")
            }
            _ => PathBuf::from(".lume_config"),
        },
    };

    // If file doesn't exist
    if !profile.exists() {
        let prompt = format!(
            "Could not find profile file at: {}\nWould you like me to write a default one? (Y/n)\n>>> ",
            profile.display()
        );

        let response = read_user_input(prompt);

        if response.is_empty() || response.to_lowercase() == "y" {
            if let Err(e) = write(&profile, INTRO_PRELUDE) {
                eprintln!("Error while writing prelude: {e}");
            }
        }

        if !parse_and_eval(INTRO_PRELUDE, env) {
            eprintln!("Sorry, the config seems has some issue");
        }
    } else if !run_file_via_pb(profile, env) {
        eprintln!("Error while loading config");
    }

    // turn PD off while in script mode.
    let pd =
        env.get("LUME_PRINT_DIRECT").is_none_or(|p| p.is_truthy()) && env.get("SCRIPT").is_none();
    set_print_direct(pd);
    if let Some(Expression::Integer(run_rec)) = env.get("LUME_MAX_RUNTIME_RECURSION") {
        // MAX_RUNTIME_RECURSION = run_rec as usize;
        MAX_RUNTIME_RECURSION.with_borrow_mut(|v| *v = run_rec as usize)
    }
    if let Some(Expression::Integer(run_rec)) = env.get("LUME_MAX_SYNTAX_RECURSION") {
        // MAX_SYNTAX_RECURSION = run_rec as usize;
        MAX_SYNTAX_RECURSION.with_borrow_mut(|v| *v = run_rec as usize)
    }

    // cmds
    init_cmds(env);
}

fn init_cmds(env: &mut Environment) {
    if !env.has("IFS") {
        env.define("IFS", Expression::None);
    }
    if !env.has("LUME_IFS_MODE") {
        env.define("IFS", Expression::Integer(60));
    }
    #[cfg(unix)]
    let sp = ":";
    #[cfg(windows)]
    let sp = ";";
    if let Some(Expression::String(pathes)) = env.get("PATH") {
        let np = pathes
            .split_terminator(sp)
            .into_iter()
            .filter(|p| !p.is_empty()) // 可选：过滤空字符串
            .map(|p| expand_home(p))
            .collect::<HashSet<_>>() // 使用 HashSet 去重
            .into_iter()
            .collect::<Vec<_>>()
            .join(sp);
        env.define_in_root("PATH", Expression::String(np));
    } else {
        #[cfg(unix)]
        env.define_in_root(
            "PATH",
            Expression::String("/usr/local/bin:/usr/sbin:/usr/bin:".to_owned()),
        );
        #[cfg(windows)]
        env.define_in_root(
            "PATH",
            Expression::String("C:\\windows\\system32;C:\\windows\\;".to_owned()),
        );
    }
}

pub fn read_user_input(prompt: impl ToString) -> String {
    print!("{}", prompt.to_string());
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin()
        .read_line(&mut input)
        .map_err(|e| eprintln!("Read Failed: {e}"));
    input.trim().to_owned()
}

pub const IFS_CMD: u8 = 1 << 1; // cmd str_arg
pub const IFS_FOR: u8 = 1 << 2; // for i in str; str |> do
pub const IFS_STR: u8 = 1 << 3; // string.split
pub const IFS_CSV: u8 = 1 << 4; // parse.to_csv
pub const IFS_PCK: u8 = 1 << 5; // ui.pick
pub fn ifs_contains(mode: u8, env: &mut Environment) -> bool {
    if let Some(Expression::Integer(m)) = env.get("LUME_IFS_MODE") {
        if m as u8 & mode != 0 {
            return true;
        }
    }
    false
}
