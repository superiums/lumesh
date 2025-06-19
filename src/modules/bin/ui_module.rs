use crate::{Environment, Expression, LmError};
use common_macros::hash_map;
use fuzzypicker::FuzzyPicker;

pub fn get() -> Expression {
    (hash_map! {
        String::from("fzp") => Expression::builtin("fzp", fzp, "fuzzy picker ui", "<list>"),
        String::from("fzpi") => Expression::builtin("fzpi", fzpi, "fuzzy picker with index", "<list>"),

    })
    .into()
}

fn fzp(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    let items = match args.len() {
        1 => match args[0].eval(env)? {
            Expression::List(list) => list.as_ref().clone(),
            Expression::String(str) => str
                .lines()
                .map(|line| Expression::String(line.to_string()))
                .collect::<Vec<_>>(),
            _ => {
                return Err(LmError::CustomError(
                    "fzp requires a list as argument".to_string(),
                ));
            }
        },
        1.. => args.clone(),
        0 => {
            return Err(LmError::CustomError(
                "fzp requires a list as argument".to_string(),
            ));
        }
    };
    // Create a new FuzzyPicker instance
    let mut picker = FuzzyPicker::new(&items);
    match picker.pick() {
        Ok(r) => Ok(r.unwrap_or(Expression::None)),
        Err(e) => Err(LmError::CustomError(format!(
            "fzp failed: {}",
            e.to_string()
        ))),
    }
}
fn fzpi(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    let items = match args.len() {
        1 => match args[0].eval(env)? {
            Expression::List(list) => list
                .as_ref()
                .iter()
                .enumerate()
                .map(|(i, li)| Expression::from(vec![Expression::Integer(i as i64), li.clone()]))
                .collect::<Vec<_>>()
                .clone(),
            Expression::String(str) => str
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    Expression::from(vec![
                        Expression::Integer(i as i64),
                        Expression::String(line.to_string()),
                    ])
                })
                .collect::<Vec<_>>(),
            _ => {
                return Err(LmError::CustomError(
                    "fzp requires a list as argument".to_string(),
                ));
            }
        },
        1.. => args
            .iter()
            .enumerate()
            .map(|(i, arg)| Expression::from(vec![Expression::Integer(i as i64), arg.clone()]))
            .collect::<Vec<_>>(),
        0 => {
            return Err(LmError::CustomError(
                "fzp requires a list as argument".to_string(),
            ));
        }
    };
    // Create a new FuzzyPicker instance
    let mut picker = FuzzyPicker::new(&items);
    match picker.pick() {
        Ok(r) => Ok(r.unwrap_or(Expression::None)),
        Err(e) => Err(LmError::CustomError(format!(
            "fzp failed: {}",
            e.to_string()
        ))),
    }
}
