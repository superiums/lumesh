use std::rc::Rc;

use crate::{Environment, Expression, RuntimeError};
use common_macros::hash_map;

pub fn get() -> Expression {
    Expression::from(hash_map! {
        String::from("env") => Expression::builtin("env", env_builtin, "get root environment as a map", ""),
        String::from("set") => Expression::builtin("set", set_builtin, "define a variable in root environment", "<var> <val>"),
        String::from("unset") => Expression::builtin("unset", unset_builtin, "undefine a variable in root environment", "<var>"),

        String::from("vars") => Expression::builtin("vars", vars_builtin, "get defined variables in current enviroment", ""),
        String::from("has") => Expression::builtin("has", has_builtin, "check if a variable is defined in current environment", "<var>"),
        String::from("defined") => Expression::builtin("defined", defined_builtin, "check if a variable is defined in current environment tree", "<var>"),

        String::from("quote") => Expression::builtin("quote", quote_builtin, "quote an expression", "<expr>"),
        String::from("err-codes") => Expression::builtin("err-codes", err_codes_builtin, "display runtime error codes", ""),
    })
}

fn quote_builtin(
    args: &Vec<Expression>,
    _env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("quote", &args, 1)?;
    Ok(Expression::Quote(Rc::new(args[0].clone())))
}

fn env_builtin(
    _args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    Ok(Expression::from(env.get_root().clone()))
}

fn vars_builtin(_: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    Ok(Expression::from(env.get_bindings_map()))
}

fn set_builtin(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("set", &args, 2)?;
    let name = args[0].to_string();
    let expr = args[1].eval(env)?;
    env.define_in_root(&name, expr);
    Ok(Expression::None)
}

fn unset_builtin(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("unset", &args, 1)?;
    let name = args[0].to_string();
    env.undefine_in_root(&name);
    Ok(Expression::None)
}

fn has_builtin(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("has", &args, 1)?;
    let name = args[0].to_string();
    Ok(Expression::Boolean(env.has(&name)))
}
fn defined_builtin(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("defined", &args, 1)?;
    let name = args[0].to_string();
    Ok(Expression::Boolean(env.is_defined(&name)))
}

fn err_codes_builtin(
    _args: &Vec<Expression>,
    _env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    Ok(RuntimeError::codes())
}
