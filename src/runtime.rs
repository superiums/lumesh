use crate::repl::read_user_input;
use crate::{Diagnostic, Environment, Expression, LmError, SyntaxError, TokenKind};
use crate::{SyntaxErrorKind, parse_script};
use std::path::PathBuf;
const INTRO_PRELUDE: &str = include_str!("config/config.lsh");

// pub fn run_text(text: &str, env: &mut Environment) -> Result<Expression, Error> {
//     parse(text)?.eval(env)
// }

pub fn run_file(path: PathBuf, env: &mut Environment) -> bool {
    match std::fs::read_to_string(path) {
        Ok(prelude) => parse_and_eval(&prelude, env),
        Err(e) => {
            eprintln!("\x1b[31m[ERROR]\x1b[0mFailed to read file:\n  {}", e);
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

pub fn syntax_highlight(line: &str) -> String {
    let (tokens, diagnostics) = super::tokenize(line);
    // dbg!(tokens);

    let mut result = String::new();
    let mut is_colored = false;

    for (token, diagnostic) in tokens.iter().zip(&diagnostics) {
        match (token.kind, token.range.to_str(line)) {
            (TokenKind::BooleanLiteral, b) => {
                result.push_str("\x1b[95m");
                is_colored = true;
                result.push_str(b);
            }
            (
                TokenKind::Punctuation,
                o @ ("@" | "\'" | "=" | "|" | ">>" | "<<" | ">>>" | "->" | "~>"),
            ) => {
                result.push_str("\x1b[96m");
                is_colored = true;
                result.push_str(o);
            }
            (TokenKind::Punctuation, o) => {
                if is_colored {
                    result.push_str("\x1b[m\x1b[0m");
                    is_colored = false;
                }
                result.push_str(o);
            }
            (TokenKind::Keyword, k) => {
                result.push_str("\x1b[95m");
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::Operator, k) => {
                result.push_str("\x1b[38;5;220m");
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::OperatorPrefix, k) => {
                result.push_str("\x1b[38;5;221m");
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::OperatorInfix, k) => {
                result.push_str("\x1b[38;5;222m");
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::OperatorPostfix, k) => {
                result.push_str("\x1b[38;5;223m");
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::StringRaw, s) => {
                result.push_str("\x1b[38;5;203m");
                is_colored = true;
                result.push_str(s);
            }
            (TokenKind::StringLiteral, s) => {
                result.push_str("\x1b[38;5;208m");
                is_colored = true;

                if let Diagnostic::InvalidStringEscapes(ranges) = diagnostic {
                    let mut last_end = token.range.start();

                    for &range in ranges.iter() {
                        result.push_str(&line[last_end..range.start()]);
                        result.push_str("\x1b[38;5;9m");
                        result.push_str(range.to_str(line));
                        result.push_str("\x1b[38;5;208m");
                        last_end = range.end();
                    }

                    result.push_str(&line[last_end..token.range.end()]);
                } else {
                    result.push_str(s);
                }
            }
            (TokenKind::IntegerLiteral | TokenKind::FloatLiteral, l) => {
                if let Diagnostic::InvalidNumber(e) = diagnostic {
                    result.push_str("\x1b[38;5;9m");
                    result.push_str(e.to_str(line));
                    is_colored = true;
                } else {
                    if is_colored {
                        result.push_str("\x1b[m\x1b[0m");
                        is_colored = false;
                    }
                    result.push_str(l);
                }
            }
            (TokenKind::Symbol, l) => {
                if let Diagnostic::IllegalChar(e) = diagnostic {
                    result.push_str("\x1b[38;5;9m");
                    result.push_str(e.to_str(line));
                    is_colored = true;
                } else {
                    if l == "None" {
                        result.push_str("\x1b[91m");
                        is_colored = true;
                    } else if matches!(l, "echo" | "exit" | "clear" | "cd" | "rm") {
                        result.push_str("\x1b[94m");
                        is_colored = true;
                    } else if is_colored {
                        result.push_str("\x1b[m\x1b[0m");
                        is_colored = false;
                    }
                    result.push_str(l);
                }
            }
            (TokenKind::Whitespace, w) => {
                result.push_str(w);
            }
            (TokenKind::LineBreak, w) => {
                result.push_str(w);
            }
            // (TokenKind::LineContinuation, w) => {
            //     result.push_str(w);
            // }
            (TokenKind::Comment, w) => {
                result.push_str("\x1b[38;5;247m");
                is_colored = true;
                result.push_str(w);
            }
        }
    }
    if diagnostics.len() > tokens.len() {
        for diagnostic in &diagnostics[tokens.len()..] {
            if let Diagnostic::NotTokenized(e) = diagnostic {
                result.push_str("\x1b[38;5;9m");
                result.push_str(e.to_str(line));
                is_colored = true;
            }
        }
    }
    if is_colored {
        result.push_str("\x1b[m\x1b[0m");
    }

    result
}

pub fn check(input: &str) -> bool {
    match parse_script(input) {
        Ok(_) => true,
        _ => false,
    }
}
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
                Ok(result) => println!("{}", result),
                Err(e) => eprintln!("\x1b[31m[ERROR]\x1b[0m {}", e),
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
            // if line.is_empty() {
            //     eprintln!("{}", e);
            //     lines = vec![];
            // } else {
            //     rl.add_history_entry(text.as_str());
            // }
        }
    }
    return false;
}

pub fn init_config(env: &mut Environment) {
    if let Some(config_dir) = dirs::config_dir() {
        let config_path = config_dir.join("lumesh");
        if !config_path.exists() {
            if let Err(e) = std::fs::create_dir(&config_path) {
                eprintln!("Error while writing prelude: {}", e);
            }
        }
        let prelude_path = config_path.join("config.lsh");
        // If file doesn't exist
        if !prelude_path.exists() {
            let prompt = format!(
                "Could not find prelude file at: {}\nWould you like me to write the default prelude to this location? (y/n)\n>>> ",
                prelude_path.display()
            );

            let response = read_user_input(prompt);

            if response.to_lowercase().trim() == "y" {
                if let Err(e) = std::fs::write(&prelude_path, INTRO_PRELUDE) {
                    eprintln!("Error while writing prelude: {}", e);
                }
            }

            if !parse_and_eval(INTRO_PRELUDE, env) {
                eprintln!("Error while running introduction prelude");
            }
        } else if !run_file(prelude_path, env) {
            eprintln!("Error while running introduction prelude");
        }
    }
    // cmds
    init_cmds(env);
}

fn init_cmds(env: &mut Environment) {
    if !env.is_defined("clear") {
        parse_and_eval("let clear = _ ~> console@clear()", env);
    }
    if !env.is_defined("pwd") {
        parse_and_eval("let pwd = _ ~> echo CWD", env);
    }
}
