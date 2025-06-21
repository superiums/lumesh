use crate::{Environment, Expression, PRINT_DIRECT, SyntaxError};
use crate::{SyntaxErrorKind, parse_script};
use std::io::{self, Write};
use std::path::PathBuf;

// pub fn run_text(text: &str, env: &mut Environment) -> Result<Expression, Error> {
//     parse(text)?.eval(env)
// }

pub fn run_file(path: PathBuf, env: &mut Environment) -> bool {
    match std::fs::read_to_string(path) {
        Ok(prelude) => parse_and_eval(&prelude, env),
        Err(e) => {
            eprintln!("\x1b[31m[ERROR]\x1b[0mFailed to read file:\n  {}", e);
            let _ = io::stderr().flush();
            false
        }
    }
}

pub fn parse(input: &str) -> Result<Expression, SyntaxError> {
    // dbg!(&input);
    match parse_script(input) {
        Ok(result) => Ok(result),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => Err(SyntaxError {
            source: format!("{}   ", input).into(),
            kind: e,
        }),
        Err(nom::Err::Incomplete(_)) => Err(SyntaxError {
            source: input.into(),
            kind: SyntaxErrorKind::InternalError,
        }),
    }
}

pub fn check(input: &str) -> bool {
    match parse_script(input) {
        Ok(_) => {
            // eprint!("parse ok");
            true
        }
        Err(_) => {
            // eprint!("parse failed:{}", e);
            false
        } // _ => {
          //     // eprint!("parse failed without err");

          //     false
          // }
    }
}
/// return whether parse success. no matter execute result is.
pub fn parse_and_eval(text: &str, env: &mut Environment) -> bool {
    if text.is_empty() {
        return true;
    };
    match parse(text) {
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
                Ok(Expression::Builtin(b)) => {
                    println!(
                        "  >> [Builtin] {}\n\x1b[1;32mDescription\x1b[0m: {}\n\x1b[1;32mParams     \x1b[0m: {}\n",
                        b.name, b.help, b.hint
                    );
                    let _ = io::stdout().flush();
                }
                Ok(result) => unsafe {
                    if PRINT_DIRECT {
                        println!("\n  >> [{}] <<\n{}", result.type_name(), result);
                        let _ = io::stdout().flush();
                    }
                },
                Err(e) => {
                    eprintln!("\x1b[31m[ERROR]\x1b[0m {}", e);
                    let _ = io::stderr().flush();
                }
            }
            // match val.clone() {
            //     Ok(Expression::Symbol(name)) => {
            //         if let Err(e) =
            //             Expression::Apply(Box::new(Expression::Symbol(name)), vec![]).eval(env)
            //         {
            //             eprintln!("{}", e)
            //         }
            //     }
            //     Ok(Expression::None) => {}
            //     Ok(Expression::Macro(_, _)) => {
            //         let _ = Expression::Apply(
            //             Box::new(Expression::Symbol("report".to_string())),
            //             vec![Expression::Apply(
            //                 Box::new(val.unwrap().clone()),
            //                 vec![env.get_cwd().into()],
            //             )],
            //         )
            //         .eval(env);
            //     }
            //     Ok(val) => {
            //         let _ = Expression::Apply(
            //             Box::new(Expression::Symbol("report".to_string())),
            //             vec![Expression::Quote(Box::new(val))],
            //         )
            //         .eval(env);
            //     }
            //     Err(e) => {
            //         eprintln!("{}", e)
            //     }
            // }
            // lines = vec![];
            return true;
        }

        Err(e) => {
            eprintln!("[PARSE FAILED] {}", e);
            let _ = io::stderr().flush();
            // if line.is_empty() {
            //     eprintln!("{}", e);
            //     lines = vec![];
            // } else {
            //     rl.add_history_entry(text.as_str());
            // }
        }
    }
    false
}

pub fn init_config(env: &mut Environment) {
    const INTRO_PRELUDE: &str = include_str!("config/config.lm");

    let profile = match env.get("LUME_PROFILE") {
        Some(p) => PathBuf::from(p.to_string()),
        _ => match dirs::config_dir() {
            Some(config_dir) => {
                let config_path = config_dir.join("lumesh");
                if !config_path.exists() {
                    if let Err(e) = std::fs::create_dir(&config_path) {
                        eprintln!("Error while writing prelude: {}", e);
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
            "Could not find profile file at: {}\nWould you like me to write the default prelude to this location? (Y/n)\n>>> ",
            profile.display()
        );

        let response = read_user_input(prompt);

        if response.is_empty() || response.to_lowercase() == "y" {
            if let Err(e) = std::fs::write(&profile, INTRO_PRELUDE) {
                eprintln!("Error while writing prelude: {}", e);
            }
        }

        if !parse_and_eval(INTRO_PRELUDE, env) {
            eprintln!("Error while running introduction prelude");
        }
    } else if !run_file(profile, env) {
        eprintln!("Error while running introduction prelude");
    }

    unsafe { PRINT_DIRECT = env.get("LUME_PRINT_DIRECT").is_none_or(|p| p.is_truthy()) }
    // cmds
    init_cmds(env);
}

fn init_cmds(env: &mut Environment) {
    if !env.is_defined("clear") {
        parse_and_eval("let clear = () -> console@clear()", env);
    }
}

pub fn read_user_input(prompt: impl ToString) -> String {
    print!("{}", prompt.to_string());
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin()
        .read_line(&mut input)
        .map_err(|e| eprintln!("Read Failed: {}", e));
    input.trim().to_owned()
}
