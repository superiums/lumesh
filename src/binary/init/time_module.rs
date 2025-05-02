use std::{thread, time::Duration};

use crate::{Environment, Expression, LmError};
use chrono::{Datelike, Local, Timelike};
use common_macros::b_tree_map;

pub fn get() -> Expression {
    (b_tree_map! {
        String::from("sleep") => Expression::builtin("sleep", sleep,
            "sleep for a given number of milliseconds"),
        String::from("display") => Expression::builtin("display", display,
            "get preformatted datetime"),
   	    String::from("year") =>Expression::builtin("year",|_,_| Ok(Expression::Integer(Local::now().year() as i64)),"get current year"),
           String::from("month") =>Expression::builtin("month",|_,_| Ok(Expression::Integer(Local::now().month() as i64)),"get current month"),
           String::from("weekday") =>Expression::builtin("weekday",|_,_| Ok(Expression::Integer(Local::now().weekday() as i64)),"get current weekday"),
           String::from("day") =>Expression::builtin("day",|_,_| Ok(Expression::Integer(Local::now().day() as i64)),"get current day"),
           String::from("hour") =>Expression::builtin("hour",|_,_| Ok(Expression::Integer(Local::now().hour() as i64)),"get current hour"),
           String::from("minute") =>Expression::builtin("minute",|_,_| Ok(Expression::Integer(Local::now().minute() as i64)),"get current minute"),
           String::from("second") =>Expression::builtin("second",|_,_| Ok(Expression::Integer(Local::now().second() as i64)),"get current second"),
           String::from("seconds") =>Expression::builtin("seconds",|_,_| Ok(Expression::Integer(Local::now().num_seconds_from_midnight() as i64)),"get parsed seconds today"),
           String::from("stamp") =>Expression::builtin("stamp",|_,_| Ok(Expression::Integer(Local::now().timestamp())),"get current unix time stamp seconds"),
           String::from("stamp-ms") =>Expression::builtin("stamp_ms",|_,_| Ok(Expression::Integer(Local::now().timestamp_millis() as i64)),"get current unix time stamp millis seconds"),
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
        Expression::String(f) => Ok(Expression::String(Local::now().format(&f).to_string())),
        // Expression::Symbol(f) => Ok(f),
        e => Err(LmError::CustomError(format!(
            "invalid abs argument {:?}",
            e
        ))),
    }
}

fn display(_: Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    Ok(Expression::Map(b_tree_map! {
        String::from("time") => Expression::String(Local::now().time().format("%H:%M:%S").to_string()),
        String::from("timepm") => Expression::String(Local::now().format("%-I:%M %p").to_string()),
        String::from("date") => Expression::String(Local::now().format("%D").to_string()),
        String::from("datetime") =>  Expression::String(Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
    }))
}
