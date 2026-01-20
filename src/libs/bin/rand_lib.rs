use crate::{Environment, Expression};
use common_macros::hash_map;
use rand::distr::SampleString;
use rand::prelude::*;
use rand::{Rng, prelude::SliceRandom};
use std::collections::HashMap;
use std::rc::Rc;

use crate::libs::BuiltinInfo;
use crate::libs::helper::{check_args_len, check_exact_args_len, get_string_arg};
use crate::libs::lazy_module::LazyModule;
use crate::{Int, RuntimeError, RuntimeErrorKind, reg_info, reg_lazy};
use std::collections::BTreeMap;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // 概率函数
         ratio,
         // 随机字符串生成
         alpha,
         alphanum,
         // 数值随机
         int,
         // 集合操作
         choose,
         shuffle,
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        // 概率函数
         ratio => "get a bool with given probability", "<probability>"
         // 随机字符串生成
         alpha => "get random alphabetic character(s)", "[length]"
         alphanum => "get random alphanumeric string", "[length]"
         // 数值随机
         int => "get random integer in range (exclusive upper bound)", "[min] [max]"
         // 集合操作
         choose => "choose random item from collection", "<list>"
         shuffle => "randomly shuffle collection items", "<list>"
    })
}
// Helper Functions
// fn get_list_arg(expr: Expression, ctx: &Expression) -> Result<Rc<Vec<Expression>>, RuntimeError> {
//     match expr {
//         Expression::List(s) => Ok(s),
//         Expression::Range(r, step) => Ok(Rc::new(
//             r.step_by(step).map(Expression::Integer).collect::<Vec<_>>(),
//         )),
//         e => Err(RuntimeError::new(
//             RuntimeErrorKind::TypeError {
//                 expected: "List".to_string(),
//                 sym: e.to_string(),
//                 found: e.type_name(),
//             },
//             ctx.clone(),
//             0,
//         )),
//     }
// }
// Probability Functions
fn ratio(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    match args.len() {
        0 => {
            let mut rng = rand::rng();
            let b = rng.random_bool(0.5);
            Ok(Expression::Boolean(b))
        }
        1 => {
            if let Expression::Float(f) = args[0].eval(env)? {
                let mut rng = rand::rng();
                let b = rng.random_bool(f);
                Ok(Expression::Boolean(b))
            } else {
                Err(RuntimeError::common(
                    "rand.ratio expected float probability".into(),
                    ctx.clone(),
                    0,
                ))
            }
        }
        2 => match (args[0].eval(env)?, args[1].eval(env)?) {
            (Expression::Integer(numerator), Expression::Integer(denominator)) => {
                let mut rng = rand::rng();
                let b = rng.random_ratio(numerator as u32, denominator as u32);
                Ok(Expression::Boolean(b))
            }
            (l, h) => Err(RuntimeError::common(
                format!("rand.ratio expected two integers, but got {l} and {h}").into(),
                ctx.clone(),
                0,
            )),
        },
        _ => Err(RuntimeError::common(
            "rand.ratio expected 0, 1, or 2 arguments".into(),
            ctx.clone(),
            0,
        )),
    }
}
// Random String Generation
fn alpha(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    match args.len() {
        0 => {
            let mut rng = rand::rng();
            let c = rng.random::<char>();
            Ok(Expression::String(c.to_string()))
        }
        1 => {
            if let Expression::Integer(size) = args[0].eval(env)? {
                let mut rng = rand::rng();
                let a = rand::distr::Alphabetic.sample_string(&mut rng, size as usize);
                Ok(Expression::String(a))
            } else {
                Err(RuntimeError::common(
                    "rand.alpha expected integer size".into(),
                    ctx.clone(),
                    0,
                ))
            }
        }
        _ => Err(RuntimeError::common(
            "rand.alpha expected 0 or 1 arguments".into(),
            ctx.clone(),
            0,
        )),
    }
}

fn alphanum(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    match args.len() {
        0 => {
            let mut rng = rand::rng();
            let c = rng.sample(rand::distr::Alphanumeric) as char;
            Ok(Expression::String(c.to_string()))
        }
        1 => {
            if let Expression::Integer(size) = args[0].eval(env)? {
                let mut rng = rand::rng();
                let a = rand::distr::Alphanumeric.sample_string(&mut rng, size as usize);
                Ok(Expression::String(a))
            } else {
                Err(RuntimeError::common(
                    "rand.alphanum expected integer size".into(),
                    ctx.clone(),
                    0,
                ))
            }
        }
        _ => Err(RuntimeError::common(
            "rand.alphanum expected 0 or 1 arguments".into(),
            ctx.clone(),
            0,
        )),
    }
}
// Random Integer Generation
fn int(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    match args.len() {
        0 => {
            let n: i64 = rand::rng().random();
            Ok(Expression::Integer(n))
        }
        1 => {
            if let Expression::Integer(max) = args[0].eval(env)? {
                let mut rng = rand::rng();
                if max < 0 {
                    let n = rng.random_range(max..=0);
                    Ok(Expression::Integer(n))
                } else {
                    let n = rng.random_range(0..=max);
                    Ok(Expression::Integer(n))
                }
            } else {
                Err(RuntimeError::common(
                    "rand.int expected integer max".into(),
                    ctx.clone(),
                    0,
                ))
            }
        }
        2 => match (args[0].eval(env)?, args[1].eval(env)?) {
            (Expression::Integer(l), Expression::Integer(h)) => {
                let mut rng = rand::rng();
                Ok(Expression::Integer(rng.random_range(l..h)))
            }
            (l, h) => Err(RuntimeError::common(
                format!("rand.int expected two integers, but got {l} and {h}").into(),
                ctx.clone(),
                0,
            )),
        },
        _ => Err(RuntimeError::common(
            "rand.int expected 0, 1 or 2 arguments".into(),
            ctx.clone(),
            0,
        )),
    }
}
// Collection Operations
fn choose(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("choose", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::List(list) => {
            let mut rng = rand::rng();
            Ok(match list.choose(&mut rng) {
                Some(s) => s.clone(),
                None => Expression::None,
            })
        }
        otherwise => Err(RuntimeError::common(
            format!("rand.choose expected a list, but got {otherwise}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn shuffle(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("shuffle", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::List(list) => {
            let mut rng = rand::rng();
            let mut s = list.as_ref().clone();
            s.shuffle(&mut rng);
            Ok(Expression::from(s))
        }
        otherwise => Err(RuntimeError::common(
            format!("rand.shuffle expected a list, but got {otherwise}").into(),
            ctx.clone(),
            0,
        )),
    }
}
