use crate::{Environment, Expression, LmError};
use common_macros::hash_map;

pub fn get() -> Expression {
    (hash_map! {
        String::from("and") => Expression::builtin("and", and, "logic and", "<boolean1> <boolean2>"),
        String::from("or") => Expression::builtin("or", or, "logic or", "<boolean1> <boolean2>"),
        String::from("not") => Expression::builtin("not", not, "logic not", "<boolean>"),

    }).into()
}

fn and(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("and", args, 2)?;
    let a = args[0].eval(env)?;
    let b = args[1].eval(env)?;

    Ok(Expression::Boolean(a.is_truthy() && b.is_truthy()))
}
fn or(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("or", args, 2)?;
    let a = args[0].eval(env)?;
    let b = args[1].eval(env)?;

    Ok(Expression::Boolean(a.is_truthy() || b.is_truthy()))
}
fn not(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("not", args, 2)?;
    let a = args[0].eval(env)?;

    Ok(Expression::Boolean(!a.is_truthy()))
}
