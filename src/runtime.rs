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
