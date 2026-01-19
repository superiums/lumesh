use std::{collections::HashMap, rc::Rc};

use crate::{
    Environment, Expression, RuntimeError,
    libs::{BuiltinFunc, BuiltinInfo, helper::check_exact_args_len},
    reg_all, reg_info,
};

pub fn regist_all() -> HashMap<&'static str, Rc<BuiltinFunc>> {
    reg_all!({
        and,or,not
    })
}

pub fn regist_info() -> HashMap<&'static str, BuiltinInfo> {
    reg_info!({
        and => "logic and", "<boolean1> <boolean2>"
        or => "logic or", "<boolean1> <boolean2>"
        not => "logic not", "<boolean1> <boolean2>"
    })
}

fn and(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("and", args, 2, ctx)?;
    let a = args[0].eval(env)?;
    let b = args[1].eval(env)?;

    Ok(Expression::Boolean(a.is_truthy() && b.is_truthy()))
}
fn or(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("or", args, 2, ctx)?;
    let a = args[0].eval(env)?;
    let b = args[1].eval(env)?;

    Ok(Expression::Boolean(a.is_truthy() || b.is_truthy()))
}
fn not(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("not", args, 2, ctx)?;
    let a = args[0].eval(env)?;

    Ok(Expression::Boolean(!a.is_truthy()))
}
