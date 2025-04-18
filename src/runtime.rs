use crate::parse_script;
use crate::{Environment, Error, Expression, SyntaxError};
use std::path::PathBuf;

pub fn run_text(text: &str, env: &mut Environment) -> Result<Expression, Error> {
    parse(text)?.eval(env)
}

pub fn run_file(path: PathBuf, env: &mut Environment) -> Result<Expression, Error> {
    match std::fs::read_to_string(path) {
        Ok(prelude) => run_text(&prelude, env),
        Err(e) => Err(Error::CustomError(format!("Failed to read file: {}", e))),
    }
}

pub fn parse(input: &str) -> Result<Expression, Error> {
    match parse_script(input) {
        Ok(result) => Ok(result),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            Err(Error::SyntaxError(input.into(), e))
        }
        Err(nom::Err::Incomplete(_)) => {
            Err(Error::SyntaxError(input.into(), SyntaxError::InternalError))
        }
    }
}

pub fn parse_and_eval(text: &str, env: &mut Environment) {
    match parse(text) {
        Ok(expr) => {
            // rl.add_history_entry(text.as_str());
            // if let Some(path) = &history_path {
            //     if rl.save_history(path).is_err() {
            //         eprintln!("Failed to save history");
            //     }
            // }
            let val = expr.eval(env);
            match val.clone() {
                Ok(Expression::Symbol(name)) => {
                    if let Err(e) =
                        Expression::Apply(Box::new(Expression::Symbol(name)), vec![]).eval(env)
                    {
                        eprintln!("{}", e)
                    }
                }
                Ok(Expression::None) => {}
                Ok(Expression::Macro(_, _)) => {
                    let _ = Expression::Apply(
                        Box::new(Expression::Symbol("report".to_string())),
                        vec![Expression::Apply(
                            Box::new(val.unwrap().clone()),
                            vec![env.get_cwd().into()],
                        )],
                    )
                    .eval(env);
                }
                Ok(val) => {
                    let _ = Expression::Apply(
                        Box::new(Expression::Symbol("report".to_string())),
                        vec![Expression::Quote(Box::new(val))],
                    )
                    .eval(env);
                }
                Err(e) => {
                    eprintln!("{}", e)
                }
            }
            // lines = vec![];
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
}
