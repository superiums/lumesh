use std::{thread, time::Duration};

use crate::{Environment, Expression, LmError};
use chrono::{Datelike, Timelike};
use common_macros::b_tree_map;

pub fn get() -> Expression {
    let now = chrono::Local::now();

    (b_tree_map! {
        String::from("sleep") => Expression::builtin("sleep", sleep,
            "sleep for a given number of milliseconds"),
        String::from("display") => Expression::builtin("display", display,
            "get preformatted datetime"),
        String::from("year") => Expression::Integer(now.year() as i64),
        String::from("month") => Expression::Integer(now.month() as i64),
        String::from("weekday") => Expression::Integer(now.weekday() as i64),
        String::from("day") => Expression::Integer(now.day() as i64),
        String::from("hour") => Expression::Integer(now.hour() as i64),
        String::from("minute") => Expression::Integer(now.minute() as i64),
        String::from("second") => Expression::Integer(now.second() as i64),
        String::from("seconds") => Expression::Integer(now.num_seconds_from_midnight() as i64),
        String::from("stamp") => Expression::Integer(now.timestamp() as i64),
        String::from("fmt") => Expression::builtin("fmt", fmt, "get formatted datetime"),
    })
    .into()
}

fn sleep(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("sleep", &args, 1)?;

    match args[0].eval(env)? {
        Expression::Float(n) => thread::sleep(Duration::from_millis(n as u64)),
        Expression::Integer(n) => thread::sleep(Duration::from_millis(n as u64)),
        otherwise => {
            return Err(LmError::CustomError(format!(
                "expected integer or float, but got {}",
                otherwise
            )));
        }
    }

    Ok(Expression::None)
}

fn fmt(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("fmt", &args, 1)?;
    match args[0].eval(env)? {
        Expression::String(f) => Ok(Expression::String(
            chrono::Local::now().format(&f).to_string(),
        )),
        // Expression::Symbol(f) => Ok(f),
        e => Err(LmError::CustomError(format!(
            "invalid abs argument {:?}",
            e
        ))),
    }
}

fn display(_: Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    let now = chrono::Local::now();

    Ok(Expression::Map(b_tree_map! {
        String::from("time") => Expression::String(now.time().format("%H:%M:%S").to_string()),
        String::from("timepm") => Expression::String(now.format("%-I:%M %p").to_string()),
        String::from("date") => Expression::String(now.format("%D").to_string()),
        String::from("datetime") =>  Expression::String(now.format("%Y-%m-%d %H:%M:%S").to_string()),
    }))
}
