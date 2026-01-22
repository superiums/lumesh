use std::{collections::HashMap, rc::Rc};

use crate::{
    Environment, Expression, RuntimeError,
    libs::{BuiltinFunc, BuiltinInfo, helper::check_args_len},
    reg_all, reg_info,
};
use std::collections::BTreeMap;

pub fn regist_all() -> HashMap<&'static str, Rc<BuiltinFunc>> {
    reg_all!({
        and,or,not
    })
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        and => "logic and", "<boolean1>..."
        or => "logic or", "<boolean1>..."
        not => "logic not", "<boolean1>..."
    })
}

fn and(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("and", args, 2.., ctx)?;
    let r = args
        .iter()
        .any(|x| !x.eval(env).map_or(false, |y| y.is_truthy()));
    Ok(Expression::Boolean(r))
}
fn or(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("or", args, 2.., ctx)?;
    let r = args
        .iter()
        .any(|x| x.eval(env).map_or(false, |y| y.is_truthy()));
    Ok(Expression::Boolean(r))
}
pub fn not(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("not", args, 2.., ctx)?;
    let r = args
        .iter()
        .any(|x| !x.eval(env).map_or(false, |y| y.is_truthy()));
    Ok(Expression::Boolean(!r))
}
